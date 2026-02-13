use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::capability::Capability;
use super::OperationDef;

/// Per-platform binary entry in an extension manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryEntry {
    /// Download URL for this platform's binary
    pub url: String,
    /// Base64-encoded Ed25519 signature of sha256(binary)
    pub signature: String,
    /// Hex-encoded SHA-256 hash (for quick integrity checks)
    pub sha256: String,
}

/// The full manifest for a host extension package.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionManifest {
    pub id: String,
    pub display_name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    #[serde(default)]
    pub license: Option<String>,
    #[serde(default)]
    pub homepage: Option<String>,
    pub operations: Vec<OperationDef>,
    #[serde(default)]
    pub capabilities: Vec<Capability>,
    /// Base64-encoded Ed25519 public key of the author
    pub author_public_key: String,
    /// Per target-triple binary info (e.g. "aarch64-apple-darwin" → BinaryEntry)
    pub binaries: HashMap<String, BinaryEntry>,
}

impl ExtensionManifest {
    /// Validate the manifest for completeness and safety.
    pub fn validate(&self) -> Result<(), String> {
        if self.id.is_empty() || self.id.len() > 100 {
            return Err("id must be 1-100 characters".into());
        }
        if !self.id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-') {
            return Err("id must contain only lowercase letters, digits, underscores, and hyphens".into());
        }
        if self.display_name.is_empty() || self.display_name.len() > 100 {
            return Err("display_name must be 1-100 characters".into());
        }
        if self.version.is_empty() || self.version.len() > 50 {
            return Err("version must be 1-50 characters".into());
        }
        if self.description.is_empty() || self.description.len() > 2000 {
            return Err("description must be 1-2000 characters".into());
        }
        if self.author.is_empty() || self.author.len() > 100 {
            return Err("author must be 1-100 characters".into());
        }
        if self.author_public_key.is_empty() {
            return Err("author_public_key is required".into());
        }
        if self.operations.is_empty() {
            return Err("at least one operation is required".into());
        }

        // Validate operation names
        let mut seen_ops = std::collections::HashSet::new();
        for op in &self.operations {
            if op.name.is_empty() || op.name.len() > 100 {
                return Err(format!("operation name must be 1-100 characters: '{}'", op.name));
            }
            if !op.name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_') {
                return Err(format!(
                    "operation name must contain only lowercase letters, digits, and underscores: '{}'",
                    op.name
                ));
            }
            if !seen_ops.insert(&op.name) {
                return Err(format!("duplicate operation name: '{}'", op.name));
            }
            if op.description.is_empty() {
                return Err(format!(
                    "operation '{}' must have a non-empty description",
                    op.name
                ));
            }
            if op.description.len() > 2000 {
                return Err(format!("operation description too long: '{}'", op.name));
            }
            // input_schema must have "type": "object" at root
            if op.input_schema.get("type").and_then(|v| v.as_str()) != Some("object") {
                return Err(format!(
                    "operation '{}' input_schema must have \"type\": \"object\" at root",
                    op.name
                ));
            }
        }

        // Validate binaries — must have at least one platform
        if self.binaries.is_empty() {
            return Err("at least one binary platform entry is required".into());
        }
        for (platform, entry) in &self.binaries {
            if entry.url.is_empty() {
                return Err(format!("binary url is empty for platform '{}'", platform));
            }
            if entry.signature.is_empty() {
                return Err(format!("binary signature is empty for platform '{}'", platform));
            }
            if entry.sha256.is_empty() {
                return Err(format!("binary sha256 is empty for platform '{}'", platform));
            }
        }

        // Check for bidi overrides in display fields
        let bidi_chars = ['\u{202A}', '\u{202B}', '\u{202C}', '\u{202D}', '\u{202E}',
                          '\u{2066}', '\u{2067}', '\u{2068}', '\u{2069}'];
        for field in [&self.display_name, &self.description, &self.author] {
            if field.chars().any(|c| bidi_chars.contains(&c)) {
                return Err("display fields must not contain Unicode bidirectional override characters".into());
            }
        }

        Ok(())
    }

    /// Get the target triple for the current platform.
    pub fn current_platform() -> &'static str {
        #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
        { "aarch64-apple-darwin" }
        #[cfg(all(target_arch = "x86_64", target_os = "macos"))]
        { "x86_64-apple-darwin" }
        #[cfg(all(target_arch = "x86_64", target_os = "linux"))]
        { "x86_64-unknown-linux-gnu" }
        #[cfg(all(target_arch = "aarch64", target_os = "linux"))]
        { "aarch64-unknown-linux-gnu" }
        #[cfg(all(target_arch = "x86_64", target_os = "windows"))]
        { "x86_64-pc-windows-msvc" }
        #[cfg(all(target_arch = "aarch64", target_os = "windows"))]
        { "aarch64-pc-windows-msvc" }
    }

    /// Get the binary entry for the current platform, if available.
    pub fn binary_for_current_platform(&self) -> Option<&BinaryEntry> {
        self.binaries.get(Self::current_platform())
    }
}
