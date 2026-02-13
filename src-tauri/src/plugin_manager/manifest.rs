use crate::permissions::Permission;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

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
pub struct McpToolDef {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub permissions: Vec<String>,
    pub input_schema: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConfig {
    pub tools: Vec<McpToolDef>,
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
    #[serde(default)]
    pub mcp: Option<McpConfig>,
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

        // MCP tool validation
        if let Some(mcp) = &self.mcp {
            let mut tool_names = HashSet::new();
            let tool_name_re = |name: &str| -> bool {
                !name.is_empty()
                    && name.len() <= 100
                    && name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
            };

            for tool in &mcp.tools {
                if !tool_name_re(&tool.name) {
                    return Err(format!(
                        "MCP tool name '{}' is invalid (must be non-empty, max 100 chars, [a-z0-9_] only)",
                        tool.name
                    ));
                }
                if !tool_names.insert(&tool.name) {
                    return Err(format!("Duplicate MCP tool name: '{}'", tool.name));
                }
                if tool.description.len() > 2000 {
                    return Err(format!(
                        "MCP tool '{}' description exceeds 2000 characters",
                        tool.name
                    ));
                }
                if strip_bidi_overrides(&tool.description) {
                    return Err(format!(
                        "MCP tool '{}' description contains bidirectional override characters",
                        tool.name
                    ));
                }
                // input_schema must have "type": "object" at root
                if tool.input_schema.get("type").and_then(|v| v.as_str()) != Some("object") {
                    return Err(format!(
                        "MCP tool '{}' input_schema must have \"type\": \"object\" at root",
                        tool.name
                    ));
                }
                // Validate permission strings parse as Permission enum
                for perm_str in &tool.permissions {
                    if serde_json::from_value::<Permission>(serde_json::Value::String(perm_str.clone())).is_err() {
                        return Err(format!(
                            "MCP tool '{}' has invalid permission: '{}'",
                            tool.name, perm_str
                        ));
                    }
                }
            }
        }

        Ok(())
    }
}
