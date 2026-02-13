use crate::permissions::Permission;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingDef {
    pub key: String,
    #[serde(rename = "type")]
    pub setting_type: String,
    pub label: String,
    pub description: Option<String>,
    pub default: Option<serde_json::Value>,
    pub options: Option<Vec<String>>,
}

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
    #[serde(default)]
    pub settings: Vec<SettingDef>,
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

/// Unicode characters that can be used to spoof display text.
fn strip_bidi_overrides(s: &str) -> bool {
    s.chars().any(|c| matches!(c,
        '\u{200E}' | '\u{200F}' | // LRM, RLM
        '\u{202A}' | '\u{202B}' | '\u{202C}' | '\u{202D}' | '\u{202E}' | // LRE, RLE, PDF, LRO, RLO
        '\u{2066}' | '\u{2067}' | '\u{2068}' | '\u{2069}' // LRI, RLI, FSI, PDI
    ))
}

impl PluginManifest {
    pub fn validate(&self) -> Result<(), String> {
        // Required fields
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

        // Field length limits (prevent UI DoS / storage abuse)
        if self.id.len() > 100 {
            return Err("Plugin ID must be 100 characters or fewer".to_string());
        }
        if self.name.len() > 100 {
            return Err("Plugin name must be 100 characters or fewer".to_string());
        }
        if self.version.len() > 50 {
            return Err("Version must be 50 characters or fewer".to_string());
        }
        if self.description.len() > 2000 {
            return Err("Description must be 2000 characters or fewer".to_string());
        }
        if self.author.len() > 100 {
            return Err("Author must be 100 characters or fewer".to_string());
        }
        if self.image.len() > 200 {
            return Err("Docker image must be 200 characters or fewer".to_string());
        }

        // Reject Unicode bidirectional overrides in display fields
        if strip_bidi_overrides(&self.name)
            || strip_bidi_overrides(&self.description)
            || strip_bidi_overrides(&self.author)
        {
            return Err("Display fields must not contain Unicode bidirectional override characters".to_string());
        }

        // Icon URL validation
        if let Some(icon) = &self.icon {
            if !icon.starts_with("http://") && !icon.starts_with("https://") {
                return Err("Icon must be an http or https URL".to_string());
            }
        }

        Ok(())
    }
}
