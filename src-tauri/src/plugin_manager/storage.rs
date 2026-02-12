use super::manifest::PluginManifest;
use crate::error::NexusResult;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

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

    pub fn find_by_token(&self, token: &str) -> Option<&InstalledPlugin> {
        self.plugins.values().find(|p| p.auth_token == token)
    }
}
