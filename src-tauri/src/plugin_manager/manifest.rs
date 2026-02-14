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
    /// When true, every MCP invocation of this tool requires user approval
    /// before being forwarded to the plugin (similar to filesystem read approval).
    #[serde(default)]
    pub requires_approval: bool,
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
    /// SHA-256 digest of the Docker image (e.g. "sha256:a1b2c3...").
    /// Required for marketplace installs, optional for local dev.
    /// Verified after pull to guarantee content integrity.
    #[serde(default)]
    pub image_digest: Option<String>,
    #[serde(default)]
    pub ui: Option<UiConfig>,
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
    /// Extension dependencies: maps extension ID to list of operations the plugin uses.
    /// Example: { "weather": ["get_forecast", "list_alerts"] }
    #[serde(default)]
    pub extensions: HashMap<String, Vec<String>>,
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
    /// Returns all permissions this plugin needs, including both declared
    /// permissions and generated extension permissions.
    ///
    /// Extension permissions are generated from the `extensions` map as
    /// `Permission::Extension("ext:{ext_id}:{operation}")` for each entry.
    pub fn all_permissions(&self) -> Vec<Permission> {
        let mut perms = self.permissions.clone();
        for (ext_id, operations) in &self.extensions {
            for op in operations {
                let perm_str = format!("ext:{}:{}", ext_id, op);
                perms.push(Permission::Extension(perm_str));
            }
        }
        perms
    }

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
        if let Some(ref ui) = self.ui {
            if ui.port == 0 {
                return Err("UI port must be non-zero".to_string());
            }
        } else {
            // Headless plugins must declare a health endpoint
            if self.health.is_none() {
                return Err("Headless plugins (ui: null) must declare a health endpoint".to_string());
            }
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
        if self.description.is_empty() {
            return Err("Plugin description is required".to_string());
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

        // Validate image_digest format if present
        if let Some(ref digest) = self.image_digest {
            if !digest.starts_with("sha256:") {
                return Err("image_digest must start with \"sha256:\"".to_string());
            }
            let hex_part = &digest[7..];
            if hex_part.len() != 64 || !hex_part.chars().all(|c| c.is_ascii_hexdigit()) {
                return Err("image_digest must be \"sha256:\" followed by 64 hex characters".to_string());
            }
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
                    && name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
            };

            for tool in &mcp.tools {
                if !tool_name_re(&tool.name) {
                    return Err(format!(
                        "MCP tool name '{}' is invalid (must be non-empty, max 100 chars, [a-z0-9_-] only)",
                        tool.name
                    ));
                }
                if !tool_names.insert(&tool.name) {
                    return Err(format!("Duplicate MCP tool name: '{}'", tool.name));
                }
                if tool.description.is_empty() {
                    return Err(format!(
                        "MCP tool '{}' must have a non-empty description",
                        tool.name
                    ));
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

        // Extension dependency validation
        for (ext_id, operations) in &self.extensions {
            if ext_id.is_empty() || ext_id.len() > 100 {
                return Err(format!(
                    "Extension ID '{}' must be 1-100 characters",
                    ext_id
                ));
            }
            if !ext_id
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
            {
                return Err(format!(
                    "Extension ID '{}' must match [a-z0-9_-]",
                    ext_id
                ));
            }
            if operations.is_empty() {
                return Err(format!(
                    "Extension '{}' must declare at least one operation",
                    ext_id
                ));
            }
            for op in operations {
                if op.is_empty() || op.len() > 100 {
                    return Err(format!(
                        "Extension '{}' operation '{}' must be 1-100 characters",
                        ext_id, op
                    ));
                }
                if !op.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-') {
                    return Err(format!(
                        "Extension '{}' operation '{}' must match [a-z0-9_-]",
                        ext_id, op
                    ));
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_manifest() -> PluginManifest {
        PluginManifest {
            id: "com.test.plugin".into(),
            name: "Test Plugin".into(),
            version: "1.0.0".into(),
            description: "A test plugin".into(),
            author: "Test".into(),
            license: None,
            homepage: None,
            icon: None,
            image: "test:latest".into(),
            image_digest: None,
            ui: Some(UiConfig { port: 80, path: "/".into() }),
            permissions: vec![],
            health: None,
            env: HashMap::new(),
            min_nexus_version: None,
            settings: vec![],
            mcp: None,
            extensions: HashMap::new(),
        }
    }

    #[test]
    fn valid_manifest_passes() {
        assert!(valid_manifest().validate().is_ok());
    }

    #[test]
    fn valid_digest_accepted() {
        let mut m = valid_manifest();
        m.image_digest = Some("sha256:a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2".into());
        assert!(m.validate().is_ok());
    }

    #[test]
    fn digest_missing_prefix_rejected() {
        let mut m = valid_manifest();
        m.image_digest = Some("a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2".into());
        let err = m.validate().unwrap_err();
        assert!(err.contains("sha256:"), "error was: {}", err);
    }

    #[test]
    fn digest_wrong_length_rejected() {
        let mut m = valid_manifest();
        m.image_digest = Some("sha256:tooshort".into());
        let err = m.validate().unwrap_err();
        assert!(err.contains("64 hex"), "error was: {}", err);
    }

    #[test]
    fn digest_non_hex_rejected() {
        let mut m = valid_manifest();
        m.image_digest = Some("sha256:zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz".into());
        let err = m.validate().unwrap_err();
        assert!(err.contains("64 hex"), "error was: {}", err);
    }

    #[test]
    fn empty_description_rejected() {
        let mut m = valid_manifest();
        m.description = String::new();
        assert!(m.validate().is_err());
    }

    #[test]
    fn bidi_override_in_name_rejected() {
        let mut m = valid_manifest();
        m.name = "Evil\u{202E}Plugin".into();
        assert!(m.validate().is_err());
    }
}
