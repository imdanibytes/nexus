use crate::permissions::Permission;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Per-operation declaration in the "extensions" manifest block.
/// Used by the rich object format to pre-declare resource scopes.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExtensionOpDecl {
    /// Pre-declared resource scopes the plugin needs for this operation.
    /// Shown in the install dialog; approved scopes are pre-populated on grant.
    #[serde(default)]
    pub scopes: Vec<String>,
}

/// Extension dependency: either a flat list of operation names or a rich
/// object mapping operation names to declarations with pre-declared scopes.
///
/// Flat: `["subscribe", "poll"]` — no scope declarations (backward-compatible)
/// Rich: `{ "subscribe": { "scopes": ["robot/**"] }, "poll": {} }` — with scopes
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ExtensionDeps {
    /// Simple: `["op1", "op2"]` — no scope declarations
    Flat(Vec<String>),
    /// Rich: `{ "op1": { "scopes": [...] }, "op2": {} }`
    Rich(HashMap<String, ExtensionOpDecl>),
}

impl ExtensionDeps {
    /// Get all operation names regardless of format.
    pub fn operation_names(&self) -> Vec<String> {
        match self {
            ExtensionDeps::Flat(ops) => ops.clone(),
            ExtensionDeps::Rich(map) => map.keys().cloned().collect(),
        }
    }

    /// Get pre-declared scopes for a specific operation, if any.
    /// Returns `None` for the flat format or if the operation isn't found.
    pub fn scopes_for(&self, op: &str) -> Option<Vec<String>> {
        match self {
            ExtensionDeps::Flat(_) => None,
            ExtensionDeps::Rich(map) => map.get(op).map(|decl| decl.scopes.clone()),
        }
    }
}

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
pub struct McpServerConfig {
    /// Path on the plugin's container port where the MCP server listens.
    /// Default: "/mcp"
    #[serde(default = "default_mcp_path")]
    pub path: String,
    /// When true, all tools from this MCP server require user approval.
    #[serde(default)]
    pub requires_approval: bool,
}

fn default_mcp_path() -> String {
    "/mcp".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConfig {
    /// DEPRECATED: Static tool declarations via custom HTTP protocol.
    /// Use `server` instead to run a native MCP server in the plugin container.
    #[serde(default)]
    pub tools: Vec<McpToolDef>,
    /// Native MCP server endpoint on the plugin container.
    #[serde(default)]
    pub server: Option<McpServerConfig>,
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
    /// Extension dependencies: maps extension ID to operations the plugin uses.
    /// Supports both flat format (backward-compatible) and rich format with scope declarations.
    ///
    /// Flat: `{ "zenoh": ["subscribe", "poll"] }`
    /// Rich: `{ "zenoh": { "subscribe": { "scopes": ["robot/**"] }, "poll": {} } }`
    #[serde(default)]
    pub extensions: HashMap<String, ExtensionDeps>,
    /// Per-plugin MCP access declarations: list of target plugin IDs this plugin
    /// needs to call MCP tools from. Each entry generates a `Permission::McpAccess`.
    ///
    /// Example: `["com.nexus.agent", "com.nexus.cookie-jar"]`
    ///
    /// If absent and `mcp:call` is in `permissions`, blanket access is preserved
    /// for backward compatibility.
    #[serde(default)]
    pub mcp_access: Vec<String>,
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
    /// Returns all permissions this plugin needs, including declared permissions,
    /// generated extension permissions, and per-plugin MCP access permissions.
    ///
    /// Extension permissions are generated from the `extensions` map as
    /// `Permission::Extension("ext:{ext_id}:{operation}")` for each entry.
    /// MCP access permissions are generated from `mcp_access` as
    /// `Permission::McpAccess("mcp:{target_plugin_id}")`.
    pub fn all_permissions(&self) -> Vec<Permission> {
        let mut perms = self.permissions.clone();
        for (ext_id, deps) in &self.extensions {
            for op in deps.operation_names() {
                let perm_str = format!("ext:{}:{}", ext_id, op);
                perms.push(Permission::Extension(perm_str));
            }
        }
        for target in &self.mcp_access {
            perms.push(Permission::McpAccess(format!("mcp:{}", target)));
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

        // MCP server validation
        if let Some(mcp) = &self.mcp {
            if let Some(ref server) = mcp.server {
                if !server.path.starts_with('/') {
                    return Err("MCP server path must start with '/'".to_string());
                }
                if server.path.len() > 200 {
                    return Err("MCP server path must be 200 characters or fewer".to_string());
                }
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
        for (ext_id, deps) in &self.extensions {
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
            let operations = deps.operation_names();
            if operations.is_empty() {
                return Err(format!(
                    "Extension '{}' must declare at least one operation",
                    ext_id
                ));
            }
            for op in &operations {
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

        // MCP access validation
        for target in &self.mcp_access {
            if target.is_empty() || target.len() > 200 {
                return Err(format!(
                    "mcp_access target '{}' must be 1-200 characters",
                    target
                ));
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
            mcp_access: vec![],
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

    // ── ExtensionDeps dual format ────────────────────────────────

    #[test]
    fn extension_deps_flat_format_deserializes() {
        let json = serde_json::json!(["subscribe", "poll"]);
        let deps: ExtensionDeps = serde_json::from_value(json).unwrap();
        assert_eq!(deps.operation_names().len(), 2);
        assert!(deps.scopes_for("subscribe").is_none());
    }

    #[test]
    fn extension_deps_rich_format_deserializes() {
        let json = serde_json::json!({
            "subscribe": { "scopes": ["robot/**", "sensor/**"] },
            "poll": {}
        });
        let deps: ExtensionDeps = serde_json::from_value(json).unwrap();
        let mut ops = deps.operation_names();
        ops.sort();
        assert_eq!(ops, vec!["poll", "subscribe"]);
        assert_eq!(
            deps.scopes_for("subscribe"),
            Some(vec!["robot/**".to_string(), "sensor/**".to_string()])
        );
        assert_eq!(deps.scopes_for("poll"), Some(vec![]));
    }

    #[test]
    fn manifest_with_flat_extensions_validates() {
        let mut m = valid_manifest();
        m.extensions.insert(
            "zenoh".into(),
            ExtensionDeps::Flat(vec!["subscribe".into(), "poll".into()]),
        );
        assert!(m.validate().is_ok());
    }

    #[test]
    fn manifest_with_rich_extensions_validates() {
        let mut m = valid_manifest();
        let mut ops = HashMap::new();
        ops.insert("subscribe".into(), ExtensionOpDecl {
            scopes: vec!["robot/**".into()],
        });
        ops.insert("poll".into(), ExtensionOpDecl::default());
        m.extensions.insert("zenoh".into(), ExtensionDeps::Rich(ops));
        assert!(m.validate().is_ok());
    }

    #[test]
    fn all_permissions_includes_rich_extension_ops() {
        let mut m = valid_manifest();
        let mut ops = HashMap::new();
        ops.insert("subscribe".into(), ExtensionOpDecl {
            scopes: vec!["robot/**".into()],
        });
        ops.insert("poll".into(), ExtensionOpDecl::default());
        m.extensions.insert("zenoh".into(), ExtensionDeps::Rich(ops));

        let perms = m.all_permissions();
        let ext_perms: Vec<&str> = perms
            .iter()
            .filter_map(|p| match p {
                Permission::Extension(s) => Some(s.as_str()),
                _ => None,
            })
            .collect();
        assert!(ext_perms.contains(&"ext:zenoh:subscribe"));
        assert!(ext_perms.contains(&"ext:zenoh:poll"));
    }

    // ── mcp_access ───────────────────────────────────────────────

    #[test]
    fn all_permissions_includes_mcp_access() {
        let mut m = valid_manifest();
        m.mcp_access = vec!["com.nexus.agent".into(), "com.nexus.cookie-jar".into()];

        let perms = m.all_permissions();
        let mcp_perms: Vec<&str> = perms
            .iter()
            .filter_map(|p| match p {
                Permission::McpAccess(s) => Some(s.as_str()),
                _ => None,
            })
            .collect();
        assert!(mcp_perms.contains(&"mcp:com.nexus.agent"));
        assert!(mcp_perms.contains(&"mcp:com.nexus.cookie-jar"));
    }

    #[test]
    fn mcp_access_validation_rejects_empty_target() {
        let mut m = valid_manifest();
        m.mcp_access = vec!["".into()];
        assert!(m.validate().is_err());
    }

    #[test]
    fn manifest_json_roundtrip_with_all_new_fields() {
        let mut m = valid_manifest();
        let mut ops = HashMap::new();
        ops.insert("subscribe".into(), ExtensionOpDecl {
            scopes: vec!["robot/**".into()],
        });
        m.extensions.insert("zenoh".into(), ExtensionDeps::Rich(ops));
        m.mcp_access = vec!["com.nexus.agent".into()];

        let json = serde_json::to_string(&m).unwrap();
        let roundtripped: PluginManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtripped.mcp_access, vec!["com.nexus.agent"]);

        let zenoh_deps = roundtripped.extensions.get("zenoh").unwrap();
        assert_eq!(zenoh_deps.scopes_for("subscribe"), Some(vec!["robot/**".to_string()]));
    }

    #[test]
    fn manifest_backward_compatible_no_mcp_access() {
        // Old manifests without mcp_access should deserialize fine (defaults to empty vec)
        let json = serde_json::json!({
            "id": "com.test.old",
            "name": "Old Plugin",
            "version": "1.0.0",
            "description": "A legacy plugin",
            "author": "Test",
            "image": "test:latest",
            "ui": { "port": 80 },
            "permissions": ["mcp:call"],
            "extensions": { "zenoh": ["subscribe", "poll"] }
        });
        let m: PluginManifest = serde_json::from_value(json).unwrap();
        assert!(m.mcp_access.is_empty());
        assert_eq!(m.extensions.get("zenoh").unwrap().operation_names().len(), 2);
        assert!(m.validate().is_ok());
    }
}
