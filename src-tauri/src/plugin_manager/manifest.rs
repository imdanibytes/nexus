use crate::permissions::Permission;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub license: Option<String>,
    pub homepage: Option<String>,
    pub icon: Option<String>,
    pub image: String,
    pub ui: UiConfig,
    #[serde(default)]
    pub permissions: Vec<Permission>,
    pub health: Option<HealthConfig>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    pub min_nexus_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    pub port: u16,
    #[serde(default = "default_path")]
    pub path: String,
}

fn default_path() -> String {
    "/".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthConfig {
    pub endpoint: String,
    #[serde(default = "default_interval")]
    pub interval_secs: u64,
}

fn default_interval() -> u64 {
    30
}

impl PluginManifest {
    pub fn validate(&self) -> Result<(), String> {
        if self.id.is_empty() {
            return Err("Plugin ID is required".to_string());
        }
        if self.name.is_empty() {
            return Err("Plugin name is required".to_string());
        }
        if self.version.is_empty() {
            return Err("Plugin version is required".to_string());
        }
        if self.image.is_empty() {
            return Err("Docker image is required".to_string());
        }
        if self.ui.port == 0 {
            return Err("UI port must be non-zero".to_string());
        }
        Ok(())
    }
}
