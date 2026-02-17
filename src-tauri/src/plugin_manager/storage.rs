use super::manifest::PluginManifest;
use crate::error::NexusResult;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::PathBuf;

/// Extract the hostname from a URL string (e.g. "github.com" from "https://github.com/foo/bar").
pub fn extract_url_host(url: &str) -> Option<String> {
    // Simple parsing: find the host between :// and the next /
    let after_scheme = url.split("://").nth(1)?;
    let host = after_scheme.split('/').next()?;
    // Strip port if present
    let host = host.split(':').next()?;
    if host.is_empty() {
        None
    } else {
        Some(host.to_string())
    }
}

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
    #[serde(alias = "auth_token")]
    pub oauth_client_id: String,
    pub installed_at: chrono::DateTime<chrono::Utc>,
    /// Hostname of the manifest URL at install time (domain pinning).
    /// If a registry entry later points to a different host, flagged as suspicious.
    #[serde(default)]
    pub manifest_url_origin: Option<String>,
    /// When true, a file watcher auto-rebuilds this plugin on source changes.
    #[serde(default)]
    pub dev_mode: bool,
    /// Absolute path to the plugin.json used for local installs (needed for dev rebuilds).
    #[serde(default)]
    pub local_manifest_path: Option<String>,
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
        crate::util::atomic_write(&self.path, data.as_bytes())?;
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
    fn legacy_auth_token_alias_deserializes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("plugins.json");

        // Old-format storage with "auth_token" should deserialize into oauth_client_id
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
                    "auth_token": "old-hash-value",
                    "installed_at": "2026-01-01T00:00:00Z"
                }
            },
            "next_port": 9701
        });
        std::fs::write(&path, serde_json::to_string_pretty(&raw_json).unwrap()).unwrap();

        let storage = PluginStorage::load(dir.path()).unwrap();
        let plugin = storage.get("test-plugin").unwrap();
        assert_eq!(plugin.oauth_client_id, "old-hash-value");
    }

    #[test]
    fn extract_url_host_https() {
        assert_eq!(
            extract_url_host("https://github.com/foo/bar.json"),
            Some("github.com".to_string())
        );
    }

    #[test]
    fn extract_url_host_http() {
        assert_eq!(
            extract_url_host("http://example.com/manifest.json"),
            Some("example.com".to_string())
        );
    }

    #[test]
    fn extract_url_host_with_port() {
        assert_eq!(
            extract_url_host("https://localhost:8080/path"),
            Some("localhost".to_string())
        );
    }

    #[test]
    fn extract_url_host_file_scheme() {
        assert_eq!(
            extract_url_host("file:///home/user/manifest.json"),
            None // empty host in file URLs
        );
    }

    #[test]
    fn extract_url_host_no_scheme() {
        assert_eq!(extract_url_host("just-a-string"), None);
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
        crate::util::atomic_write(&self.path, data.as_bytes())?;
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
    /// Tools explicitly enabled by the user (whitelist for default-disabled tools).
    /// Used for tools that ship with `enabled: false` (e.g., Nexus Code tools).
    /// A tool with `enabled: false` only appears if it's in this list.
    #[serde(default)]
    pub enabled_tools: Vec<String>,
    /// Tools the user has permanently approved (skips `requires_approval` prompts).
    /// Populated when the user clicks "Approve" (vs "Approve Once") in the
    /// runtime approval dialog for an MCP tool.
    #[serde(default)]
    pub approved_tools: Vec<String>,
    /// Resource URIs disabled by the user (native MCP resources).
    #[serde(default)]
    pub disabled_resources: Vec<String>,
    /// Prompt names disabled by the user (native MCP prompts).
    #[serde(default)]
    pub disabled_prompts: Vec<String>,
}

impl Default for McpPluginSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            disabled_tools: vec![],
            enabled_tools: vec![],
            approved_tools: vec![],
            disabled_resources: vec![],
            disabled_prompts: vec![],
        }
    }
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
        crate::util::atomic_write(&self.path, data.as_bytes())?;
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
    /// How often to auto-check for plugin/extension updates (in minutes).
    /// 0 = manual only. Default: 30.
    #[serde(default = "default_update_interval")]
    pub update_check_interval_minutes: u32,
    /// UI language code (BCP-47). Injected as NEXUS_LANGUAGE into plugin containers.
    #[serde(default = "default_language")]
    pub language: String,
    /// UI theme identifier. "default" = teal accent, "nebula" = purple accent.
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(skip)]
    path: PathBuf,
}

fn default_update_interval() -> u32 {
    30
}

fn default_language() -> String {
    "en".to_string()
}

fn default_theme() -> String {
    "default".to_string()
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
        crate::util::atomic_write(&self.path, data.as_bytes())?;
        Ok(())
    }
}
