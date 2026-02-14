use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::NexusResult;
use crate::extensions::signing::{KeyConsistency, TrustedKeyStore};
use crate::extensions::storage::ExtensionStorage;
use crate::plugin_manager::registry::{
    ExtensionRegistryEntry, RegistryEntry, RegistryStore, RegistryTrust,
};
use crate::plugin_manager::storage::{self, PluginStorage};
use crate::version;

const STATE_FILE: &str = "update_state.json";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum UpdateSecurity {
    Verified,
    KeyMatch,
    KeyChanged,
    DigestAvailable,
    NoDigest,
    UntrustedSource,
    ManifestDomainChanged,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum UpdateItemType {
    Plugin,
    Extension,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AvailableUpdate {
    pub item_id: String,
    pub item_type: UpdateItemType,
    pub item_name: String,
    pub installed_version: String,
    pub available_version: String,
    pub manifest_url: String,
    pub registry_source: String,
    pub security: Vec<UpdateSecurity>,
    pub new_image_digest: Option<String>,
    pub author_public_key: Option<String>,
    /// When present, the image should be rebuilt from this directory on update.
    #[serde(default)]
    pub build_context: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct UpdateCheckState {
    pub last_checked: Option<chrono::DateTime<chrono::Utc>>,
    pub available_updates: Vec<AvailableUpdate>,
    pub dismissed: HashMap<String, String>,
}

/// Load persisted update state from disk.
pub fn load_update_state(data_dir: &Path) -> UpdateCheckState {
    let path = data_dir.join(STATE_FILE);
    if path.exists() {
        if let Ok(data) = std::fs::read_to_string(&path) {
            if let Ok(state) = serde_json::from_str::<UpdateCheckState>(&data) {
                return state;
            }
        }
    }
    UpdateCheckState::default()
}

/// Save update state to disk.
pub fn save_update_state(data_dir: &Path, state: &UpdateCheckState) -> NexusResult<()> {
    let path = data_dir.join(STATE_FILE);
    let data = serde_json::to_string_pretty(state)?;
    std::fs::write(&path, data)?;
    Ok(())
}

/// Scan installed plugins and extensions against registry caches to find available updates.
#[allow(clippy::too_many_arguments)]
pub fn check_for_updates(
    plugin_storage: &PluginStorage,
    extension_storage: &ExtensionStorage,
    plugin_registry: &[RegistryEntry],
    extension_registry: &[ExtensionRegistryEntry],
    trusted_keys: &TrustedKeyStore,
    registry_store: &RegistryStore,
    dismissed: &HashMap<String, String>,
) -> Vec<AvailableUpdate> {
    let mut updates = Vec::new();

    // Check plugins
    for installed in plugin_storage.list() {
        let plugin_id = &installed.manifest.id;
        let installed_version = &installed.manifest.version;

        // Find matching registry entry
        let reg_entry = match plugin_registry.iter().find(|e| &e.id == plugin_id) {
            Some(entry) => entry,
            None => continue,
        };

        // Check if registry version is newer
        let is_newer = version::compare_versions(installed_version, &reg_entry.version)
            .map(|ord| ord == std::cmp::Ordering::Less)
            .unwrap_or(false);

        if !is_newer {
            continue;
        }

        // Skip if this version is dismissed
        if dismissed.get(plugin_id).map(|v| v.as_str()) == Some(&reg_entry.version) {
            continue;
        }

        // Build security flags
        let mut security = Vec::new();

        if reg_entry.image_digest.is_some() {
            security.push(UpdateSecurity::DigestAvailable);
        } else {
            security.push(UpdateSecurity::NoDigest);
        }

        // Check source trust level
        let source_trust = registry_store.source_trust(&reg_entry.source);
        if source_trust != RegistryTrust::Official {
            security.push(UpdateSecurity::UntrustedSource);
        }

        // Domain pinning: flag if manifest URL hostname changed from install time
        if let Some(ref pinned_origin) = installed.manifest_url_origin {
            if let Some(current_origin) = storage::extract_url_host(&reg_entry.manifest_url) {
                if pinned_origin != &current_origin {
                    security.push(UpdateSecurity::ManifestDomainChanged);
                }
            }
        }

        updates.push(AvailableUpdate {
            item_id: plugin_id.clone(),
            item_type: UpdateItemType::Plugin,
            item_name: installed.manifest.name.clone(),
            installed_version: installed_version.clone(),
            available_version: reg_entry.version.clone(),
            manifest_url: reg_entry.manifest_url.clone(),
            registry_source: reg_entry.source.clone(),
            security,
            new_image_digest: reg_entry.image_digest.clone(),
            author_public_key: None,
            build_context: reg_entry.build_context.clone(),
        });
    }

    // Check extensions
    for installed in extension_storage.list() {
        let ext_id = &installed.manifest.id;
        let installed_version = &installed.manifest.version;

        let reg_entry = match extension_registry.iter().find(|e| e.id == *ext_id) {
            Some(entry) => entry,
            None => continue,
        };

        let is_newer = version::compare_versions(installed_version, &reg_entry.version)
            .map(|ord| ord == std::cmp::Ordering::Less)
            .unwrap_or(false);

        if !is_newer {
            continue;
        }

        if dismissed.get(ext_id.as_str()).map(|v| v.as_str()) == Some(&reg_entry.version) {
            continue;
        }

        let mut security = Vec::new();

        // Key consistency check
        if let Some(ref reg_key) = reg_entry.author_public_key {
            match trusted_keys.check_key_consistency(&installed.manifest.author, reg_key) {
                KeyConsistency::Matches => security.push(UpdateSecurity::KeyMatch),
                KeyConsistency::Changed => security.push(UpdateSecurity::KeyChanged),
                KeyConsistency::NewAuthor => security.push(UpdateSecurity::Verified),
            }
        }

        // Source trust
        let source_trust = registry_store.source_trust(&reg_entry.source);
        if source_trust != RegistryTrust::Official {
            security.push(UpdateSecurity::UntrustedSource);
        }

        updates.push(AvailableUpdate {
            item_id: ext_id.clone(),
            item_type: UpdateItemType::Extension,
            item_name: installed.manifest.display_name.clone(),
            installed_version: installed_version.clone(),
            available_version: reg_entry.version.clone(),
            manifest_url: reg_entry.manifest_url.clone(),
            registry_source: reg_entry.source.clone(),
            security,
            new_image_digest: None,
            author_public_key: reg_entry.author_public_key.clone(),
            build_context: None,
        });
    }

    updates
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extensions::manifest::ExtensionManifest;
    use crate::extensions::storage::InstalledExtension;
    use crate::plugin_manager::manifest::{PluginManifest, UiConfig};
    use crate::plugin_manager::storage::{InstalledPlugin, PluginStatus};

    fn make_plugin(id: &str, version: &str, digest: Option<&str>, origin: Option<&str>) -> InstalledPlugin {
        InstalledPlugin {
            manifest: PluginManifest {
                id: id.to_string(),
                name: id.to_string(),
                version: version.to_string(),
                description: "test".to_string(),
                author: "test".to_string(),
                license: None,
                homepage: None,
                icon: None,
                image: "test:latest".to_string(),
                image_digest: digest.map(|d| d.to_string()),
                ui: Some(UiConfig { port: 80, path: "/".to_string() }),
                permissions: vec![],
                health: None,
                env: HashMap::new(),
                min_nexus_version: None,
                settings: vec![],
                mcp: None,
                extensions: HashMap::new(),
            },
            container_id: None,
            status: PluginStatus::Stopped,
            assigned_port: 9700,
            auth_token: "hash".to_string(),
            installed_at: chrono::Utc::now(),
            manifest_url_origin: origin.map(|o| o.to_string()),
        }
    }

    fn make_registry_entry(id: &str, version: &str, digest: Option<&str>, manifest_url: &str) -> RegistryEntry {
        RegistryEntry {
            id: id.to_string(),
            name: id.to_string(),
            version: version.to_string(),
            description: "test".to_string(),
            image: "test:latest".to_string(),
            image_digest: digest.map(|d| d.to_string()),
            manifest_url: manifest_url.to_string(),
            manifest_sha256: None,
            categories: vec![],
            source: "Nexus Community".to_string(),
            source_trust: None,
            author: None,
            author_url: None,
            created_at: None,
            license: None,
            homepage: None,
            icon: None,
            status: None,
            build_context: None,
        }
    }

    fn make_ext_registry_entry(id: &str, version: &str, key: Option<&str>) -> ExtensionRegistryEntry {
        ExtensionRegistryEntry {
            id: id.to_string(),
            name: id.to_string(),
            version: version.to_string(),
            description: "test".to_string(),
            manifest_url: "https://example.com/ext.json".to_string(),
            manifest_sha256: None,
            categories: vec![],
            source: "Nexus Community".to_string(),
            author_public_key: key.map(|k| k.to_string()),
            author: None,
            author_url: None,
            created_at: None,
            platforms: vec![],
            status: None,
        }
    }

    fn default_registry_store() -> RegistryStore {
        RegistryStore::default()
    }

    #[test]
    fn detects_newer_plugin_version() {
        let dir = tempfile::tempdir().unwrap();
        let mut storage = crate::plugin_manager::storage::PluginStorage::load(dir.path()).unwrap();
        storage.add(make_plugin("foo", "1.0.0", None, None)).unwrap();

        let ext_storage = ExtensionStorage::load(dir.path());
        let trusted = TrustedKeyStore::load(dir.path());
        let reg_store = default_registry_store();
        let dismissed = HashMap::new();

        let registry = vec![make_registry_entry("foo", "2.0.0", None, "https://example.com/foo.json")];

        let updates = check_for_updates(
            &storage, &ext_storage, &registry, &[], &trusted, &reg_store, &dismissed,
        );

        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].item_id, "foo");
        assert_eq!(updates[0].available_version, "2.0.0");
    }

    #[test]
    fn ignores_same_version() {
        let dir = tempfile::tempdir().unwrap();
        let mut storage = crate::plugin_manager::storage::PluginStorage::load(dir.path()).unwrap();
        storage.add(make_plugin("foo", "1.0.0", None, None)).unwrap();

        let ext_storage = ExtensionStorage::load(dir.path());
        let trusted = TrustedKeyStore::load(dir.path());
        let reg_store = default_registry_store();
        let dismissed = HashMap::new();

        let registry = vec![make_registry_entry("foo", "1.0.0", None, "https://example.com/foo.json")];

        let updates = check_for_updates(
            &storage, &ext_storage, &registry, &[], &trusted, &reg_store, &dismissed,
        );

        assert!(updates.is_empty());
    }

    #[test]
    fn dismissed_update_filtered() {
        let dir = tempfile::tempdir().unwrap();
        let mut storage = crate::plugin_manager::storage::PluginStorage::load(dir.path()).unwrap();
        storage.add(make_plugin("foo", "1.0.0", None, None)).unwrap();

        let ext_storage = ExtensionStorage::load(dir.path());
        let trusted = TrustedKeyStore::load(dir.path());
        let reg_store = default_registry_store();

        let mut dismissed = HashMap::new();
        dismissed.insert("foo".to_string(), "2.0.0".to_string());

        let registry = vec![make_registry_entry("foo", "2.0.0", None, "https://example.com/foo.json")];

        let updates = check_for_updates(
            &storage, &ext_storage, &registry, &[], &trusted, &reg_store, &dismissed,
        );

        assert!(updates.is_empty());
    }

    #[test]
    fn digest_available_flag() {
        let dir = tempfile::tempdir().unwrap();
        let mut storage = crate::plugin_manager::storage::PluginStorage::load(dir.path()).unwrap();
        storage.add(make_plugin("foo", "1.0.0", None, None)).unwrap();

        let ext_storage = ExtensionStorage::load(dir.path());
        let trusted = TrustedKeyStore::load(dir.path());
        let reg_store = default_registry_store();
        let dismissed = HashMap::new();

        let registry = vec![make_registry_entry("foo", "2.0.0", Some("sha256:abc123"), "https://example.com/foo.json")];

        let updates = check_for_updates(
            &storage, &ext_storage, &registry, &[], &trusted, &reg_store, &dismissed,
        );

        assert!(updates[0].security.contains(&UpdateSecurity::DigestAvailable));
    }

    #[test]
    fn no_digest_flag() {
        let dir = tempfile::tempdir().unwrap();
        let mut storage = crate::plugin_manager::storage::PluginStorage::load(dir.path()).unwrap();
        storage.add(make_plugin("foo", "1.0.0", None, None)).unwrap();

        let ext_storage = ExtensionStorage::load(dir.path());
        let trusted = TrustedKeyStore::load(dir.path());
        let reg_store = default_registry_store();
        let dismissed = HashMap::new();

        let registry = vec![make_registry_entry("foo", "2.0.0", None, "https://example.com/foo.json")];

        let updates = check_for_updates(
            &storage, &ext_storage, &registry, &[], &trusted, &reg_store, &dismissed,
        );

        assert!(updates[0].security.contains(&UpdateSecurity::NoDigest));
    }

    #[test]
    fn manifest_domain_change_detected() {
        let dir = tempfile::tempdir().unwrap();
        let mut storage = crate::plugin_manager::storage::PluginStorage::load(dir.path()).unwrap();
        storage.add(make_plugin("foo", "1.0.0", None, Some("github.com"))).unwrap();

        let ext_storage = ExtensionStorage::load(dir.path());
        let trusted = TrustedKeyStore::load(dir.path());
        let reg_store = default_registry_store();
        let dismissed = HashMap::new();

        // Registry points to a DIFFERENT domain
        let registry = vec![make_registry_entry("foo", "2.0.0", None, "https://evil.com/foo.json")];

        let updates = check_for_updates(
            &storage, &ext_storage, &registry, &[], &trusted, &reg_store, &dismissed,
        );

        assert!(updates[0].security.contains(&UpdateSecurity::ManifestDomainChanged));
    }

    #[test]
    fn manifest_domain_same_no_flag() {
        let dir = tempfile::tempdir().unwrap();
        let mut storage = crate::plugin_manager::storage::PluginStorage::load(dir.path()).unwrap();
        storage.add(make_plugin("foo", "1.0.0", None, Some("github.com"))).unwrap();

        let ext_storage = ExtensionStorage::load(dir.path());
        let trusted = TrustedKeyStore::load(dir.path());
        let reg_store = default_registry_store();
        let dismissed = HashMap::new();

        let registry = vec![make_registry_entry("foo", "2.0.0", None, "https://github.com/other/path.json")];

        let updates = check_for_updates(
            &storage, &ext_storage, &registry, &[], &trusted, &reg_store, &dismissed,
        );

        assert!(!updates[0].security.contains(&UpdateSecurity::ManifestDomainChanged));
    }

    #[test]
    fn extension_key_match() {
        let dir = tempfile::tempdir().unwrap();
        let storage = crate::plugin_manager::storage::PluginStorage::load(dir.path()).unwrap();

        let mut ext_storage = ExtensionStorage::load(dir.path());
        ext_storage.add(InstalledExtension {
            manifest: ExtensionManifest {
                id: "ext1".to_string(),
                display_name: "Ext 1".to_string(),
                version: "1.0.0".to_string(),
                description: "test".to_string(),
                author: "author1".to_string(),
                license: None,
                homepage: None,
                operations: vec![],
                capabilities: vec![],
                author_public_key: "key123".to_string(),
                binaries: HashMap::new(),
                extension_dependencies: vec![],
            },
            enabled: false,
            installed_at: chrono::Utc::now(),
            binary_name: "extension".to_string(),
        }).unwrap();

        let mut trusted = TrustedKeyStore::load(dir.path());
        trusted.trust("author1", "key123").unwrap();

        let reg_store = default_registry_store();
        let dismissed = HashMap::new();

        let ext_registry = vec![make_ext_registry_entry("ext1", "2.0.0", Some("key123"))];

        let updates = check_for_updates(
            &storage, &ext_storage, &[], &ext_registry, &trusted, &reg_store, &dismissed,
        );

        assert_eq!(updates.len(), 1);
        assert!(updates[0].security.contains(&UpdateSecurity::KeyMatch));
    }

    #[test]
    fn extension_key_changed() {
        let dir = tempfile::tempdir().unwrap();
        let storage = crate::plugin_manager::storage::PluginStorage::load(dir.path()).unwrap();

        let mut ext_storage = ExtensionStorage::load(dir.path());
        ext_storage.add(InstalledExtension {
            manifest: ExtensionManifest {
                id: "ext1".to_string(),
                display_name: "Ext 1".to_string(),
                version: "1.0.0".to_string(),
                description: "test".to_string(),
                author: "author1".to_string(),
                license: None,
                homepage: None,
                operations: vec![],
                capabilities: vec![],
                author_public_key: "key123".to_string(),
                binaries: HashMap::new(),
                extension_dependencies: vec![],
            },
            enabled: false,
            installed_at: chrono::Utc::now(),
            binary_name: "extension".to_string(),
        }).unwrap();

        let mut trusted = TrustedKeyStore::load(dir.path());
        trusted.trust("author1", "key123").unwrap();

        let reg_store = default_registry_store();
        let dismissed = HashMap::new();

        // Registry entry has a DIFFERENT key
        let ext_registry = vec![make_ext_registry_entry("ext1", "2.0.0", Some("different_key"))];

        let updates = check_for_updates(
            &storage, &ext_storage, &[], &ext_registry, &trusted, &reg_store, &dismissed,
        );

        assert_eq!(updates.len(), 1);
        assert!(updates[0].security.contains(&UpdateSecurity::KeyChanged));
    }
}
