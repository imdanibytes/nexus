use super::manifest::PluginManifest;
use crate::error::NexusResult;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::PathBuf;

/// SHA-256 hash a raw token and return the hex digest.
pub fn hash_token(raw_token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(raw_token.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PluginStatus {
    Installing,
    Running,
    Stopped,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledPlugin {
    pub manifest: PluginManifest,
    pub container_id: Option<String>,
    pub status: PluginStatus,
    pub assigned_port: u16,
    pub auth_token: String,
    pub installed_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct PluginStorage {
    plugins: HashMap<String, InstalledPlugin>,
    next_port: u16,
    #[serde(skip)]
    path: PathBuf,
}

impl PluginStorage {
    pub fn load(data_dir: &std::path::Path) -> NexusResult<Self> {
        let path = data_dir.join("plugins.json");
        if path.exists() {
            let data = std::fs::read_to_string(&path)?;
            let mut storage: PluginStorage = serde_json::from_str(&data)?;
            storage.path = path;
            if storage.next_port == 0 {
                storage.next_port = 9700;
            }

            // Migrate any raw (unhashed) tokens to SHA-256 hashes.
            // Raw UUID tokens are 36 chars; SHA-256 hex digests are 64 chars.
            let mut migrated = false;
            for plugin in storage.plugins.values_mut() {
                if plugin.auth_token.len() != 64 {
                    plugin.auth_token = hash_token(&plugin.auth_token);
                    migrated = true;
                }
            }
            if migrated {
                storage.save()?;
                log::info!("Migrated plugin auth tokens to SHA-256 hashes");
            }

            Ok(storage)
        } else {
            Ok(PluginStorage {
                plugins: HashMap::new(),
                next_port: 9700,
                path,
            })
        }
    }

    pub fn save(&self) -> NexusResult<()> {
        let data = serde_json::to_string_pretty(self)?;
        std::fs::write(&self.path, data)?;
        Ok(())
    }

    pub fn add(&mut self, plugin: InstalledPlugin) -> NexusResult<()> {
        self.plugins.insert(plugin.manifest.id.clone(), plugin);
        self.save()
    }

    pub fn remove(&mut self, plugin_id: &str) -> NexusResult<Option<InstalledPlugin>> {
        let removed = self.plugins.remove(plugin_id);
        self.save()?;
        Ok(removed)
    }

    pub fn get(&self, plugin_id: &str) -> Option<&InstalledPlugin> {
        self.plugins.get(plugin_id)
    }

    pub fn get_mut(&mut self, plugin_id: &str) -> Option<&mut InstalledPlugin> {
        self.plugins.get_mut(plugin_id)
    }

    pub fn list(&self) -> Vec<&InstalledPlugin> {
        self.plugins.values().collect()
    }

    pub fn allocate_port(&mut self) -> u16 {
        let port = self.next_port;
        self.next_port += 1;
        port
    }

    /// Look up a plugin by its raw bearer token.
    /// The stored value is a SHA-256 hash, so the input is hashed before comparison.
    pub fn find_by_token(&self, raw_token: &str) -> Option<&InstalledPlugin> {
        let hashed = hash_token(raw_token);
        self.plugins.values().find(|p| p.auth_token == hashed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_token_produces_64_char_hex() {
        let hash = hash_token("test-token-123");
        assert_eq!(hash.len(), 64, "SHA-256 hex digest should be 64 chars");
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn hash_token_is_deterministic() {
        let a = hash_token("same-token");
        let b = hash_token("same-token");
        assert_eq!(a, b);
    }

    #[test]
    fn different_tokens_produce_different_hashes() {
        let a = hash_token("token-a");
        let b = hash_token("token-b");
        assert_ne!(a, b);
    }

    #[test]
    fn find_by_token_matches_hashed() {
        let dir = tempfile::tempdir().unwrap();
        let mut storage = PluginStorage::load(dir.path()).unwrap();

        let raw_token = "my-secret-token";
        let manifest = crate::plugin_manager::manifest::PluginManifest {
            id: "test-plugin".into(),
            name: "Test".into(),
            version: "1.0.0".into(),
            description: "Test plugin".into(),
            author: "Test".into(),
            license: None,
            homepage: None,
            icon: None,
            image: "test:latest".into(),
            image_digest: None,
            ui: crate::plugin_manager::manifest::UiConfig {
                port: 80,
                path: "/".into(),
            },
            permissions: vec![],
            health: None,
            env: HashMap::new(),
            min_nexus_version: None,
            settings: vec![],
            mcp: None,
            extensions: HashMap::new(),
        };

        let plugin = InstalledPlugin {
            manifest,
            container_id: None,
            status: PluginStatus::Stopped,
            assigned_port: 9700,
            auth_token: hash_token(raw_token),
            installed_at: chrono::Utc::now(),
        };
        storage.add(plugin).unwrap();

        assert!(storage.find_by_token(raw_token).is_some());
        assert_eq!(
            storage.find_by_token(raw_token).unwrap().manifest.id,
            "test-plugin"
        );
    }

    #[test]
    fn find_by_token_rejects_wrong_token() {
        let dir = tempfile::tempdir().unwrap();
        let mut storage = PluginStorage::load(dir.path()).unwrap();

        let manifest = crate::plugin_manager::manifest::PluginManifest {
            id: "test-plugin".into(),
            name: "Test".into(),
            version: "1.0.0".into(),
            description: "Test plugin".into(),
            author: "Test".into(),
            license: None,
            homepage: None,
            icon: None,
            image: "test:latest".into(),
            image_digest: None,
            ui: crate::plugin_manager::manifest::UiConfig {
                port: 80,
                path: "/".into(),
            },
            permissions: vec![],
            health: None,
            env: HashMap::new(),
            min_nexus_version: None,
            settings: vec![],
            mcp: None,
            extensions: HashMap::new(),
        };

        let plugin = InstalledPlugin {
            manifest,
            container_id: None,
            status: PluginStatus::Stopped,
            assigned_port: 9700,
            auth_token: hash_token("correct-token"),
            installed_at: chrono::Utc::now(),
        };
        storage.add(plugin).unwrap();

        assert!(storage.find_by_token("wrong-token").is_none());
    }

    #[test]
    fn token_migration_converts_raw_to_hash() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("plugins.json");

        // Write a storage file with a raw UUID token (36 chars, not 64)
        let raw_json = serde_json::json!({
            "plugins": {
                "test-plugin": {
                    "manifest": {
                        "id": "test-plugin",
                        "name": "Test",
                        "version": "1.0.0",
                        "description": "Test plugin",
                        "author": "Test",
                        "image": "test:latest",
                        "ui": { "port": 80, "path": "/" }
                    },
                    "container_id": null,
                    "status": "stopped",
                    "assigned_port": 9700,
                    "auth_token": "550e8400-e29b-41d4-a716-446655440000",
                    "installed_at": "2026-01-01T00:00:00Z"
                }
            },
            "next_port": 9701
        });
        std::fs::write(&path, serde_json::to_string_pretty(&raw_json).unwrap()).unwrap();

        // Load — should trigger migration
        let storage = PluginStorage::load(dir.path()).unwrap();
        let plugin = storage.get("test-plugin").unwrap();

        // Token should now be a 64-char SHA-256 hash
        assert_eq!(plugin.auth_token.len(), 64);

        // And it should match the hash of the original raw token
        assert_eq!(
            plugin.auth_token,
            hash_token("550e8400-e29b-41d4-a716-446655440000")
        );
    }
}

// ---------------------------------------------------------------------------
// Per-plugin settings storage
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct PluginSettingsStore {
    settings: HashMap<String, HashMap<String, serde_json::Value>>,
    #[serde(skip)]
    path: PathBuf,
}

impl PluginSettingsStore {
    pub fn load(data_dir: &std::path::Path) -> NexusResult<Self> {
        let path = data_dir.join("plugin_settings.json");
        if path.exists() {
            let data = std::fs::read_to_string(&path)?;
            let mut store: PluginSettingsStore = serde_json::from_str(&data)?;
            store.path = path;
            Ok(store)
        } else {
            Ok(PluginSettingsStore {
                settings: HashMap::new(),
                path,
            })
        }
    }

    pub fn save(&self) -> NexusResult<()> {
        let data = serde_json::to_string_pretty(self)?;
        std::fs::write(&self.path, data)?;
        Ok(())
    }

    pub fn get(&self, plugin_id: &str) -> HashMap<String, serde_json::Value> {
        self.settings.get(plugin_id).cloned().unwrap_or_default()
    }

    pub fn set(
        &mut self,
        plugin_id: &str,
        values: HashMap<String, serde_json::Value>,
    ) -> NexusResult<()> {
        self.settings.insert(plugin_id.to_string(), values);
        self.save()
    }
}

// ---------------------------------------------------------------------------
// MCP settings storage
// ---------------------------------------------------------------------------

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpPluginSettings {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub disabled_tools: Vec<String>,
    /// Tools the user has permanently approved (skips `requires_approval` prompts).
    /// Populated when the user clicks "Approve" (vs "Approve Once") in the
    /// runtime approval dialog for an MCP tool.
    #[serde(default)]
    pub approved_tools: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpSettings {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub plugins: HashMap<String, McpPluginSettings>,
    #[serde(skip)]
    path: PathBuf,
}

impl Default for McpSettings {
    fn default() -> Self {
        McpSettings {
            enabled: true,
            plugins: HashMap::new(),
            path: PathBuf::new(),
        }
    }
}

impl McpSettings {
    pub fn load(data_dir: &std::path::Path) -> NexusResult<Self> {
        let path = data_dir.join("mcp_settings.json");
        if path.exists() {
            let data = std::fs::read_to_string(&path)?;
            let mut settings: McpSettings = serde_json::from_str(&data)?;
            settings.path = path;
            Ok(settings)
        } else {
            Ok(McpSettings {
                path,
                ..Default::default()
            })
        }
    }

    pub fn save(&self) -> NexusResult<()> {
        let data = serde_json::to_string_pretty(self)?;
        std::fs::write(&self.path, data)?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// App-level settings (resource quotas, etc.)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NexusSettings {
    #[serde(default)]
    pub cpu_quota_percent: Option<f64>,
    #[serde(default)]
    pub memory_limit_mb: Option<u64>,
    #[serde(skip)]
    path: PathBuf,
}

impl NexusSettings {
    pub fn load(data_dir: &std::path::Path) -> NexusResult<Self> {
        let path = data_dir.join("settings.json");
        if path.exists() {
            let data = std::fs::read_to_string(&path)?;
            let mut settings: NexusSettings = serde_json::from_str(&data)?;
            settings.path = path;
            Ok(settings)
        } else {
            Ok(NexusSettings {
                path,
                ..Default::default()
            })
        }
    }

    pub fn save(&self) -> NexusResult<()> {
        let data = serde_json::to_string_pretty(self)?;
        std::fs::write(&self.path, data)?;
        Ok(())
    }
}
