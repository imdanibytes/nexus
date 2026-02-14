pub mod classify;
pub mod discovery;
pub mod generate;

use serde::{Deserialize, Serialize};

/// Metadata for a generated MCP wrapper plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    pub id: String,
    pub name: String,
    pub description: String,
    pub author: String,
}

/// Extract an npm package name from an MCP server command.
///
/// "npx -y @modelcontextprotocol/server-everything" → "@modelcontextprotocol/server-everything"
/// "npx @org/pkg --flag value" → "@org/pkg"
fn extract_npm_package(cmd: &str) -> Option<String> {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    for part in parts.iter().skip(1) {
        if part.starts_with('-') {
            continue;
        }
        if part.starts_with('@') || part.chars().next().map_or(false, |c| c.is_ascii_lowercase()) {
            return Some(part.to_string());
        }
    }
    None
}

/// Derive sensible default metadata from an MCP server command.
pub fn suggest_metadata(mcp_command: &str) -> PluginMetadata {
    let pkg = extract_npm_package(mcp_command);

    let (id, name) = match pkg {
        Some(ref p) => {
            // "@upstash/context7-mcp" → id: "mcp.context7-mcp", name: "context7-mcp (MCP)"
            let short = p
                .rsplit('/')
                .next()
                .unwrap_or(p)
                .trim_start_matches("server-");
            (
                format!("mcp.{}", short),
                format!("{} (MCP)", short),
            )
        }
        None => {
            let binary = mcp_command.split_whitespace().next().unwrap_or("unknown");
            (
                format!("mcp.{}", binary),
                format!("{} (MCP)", binary),
            )
        }
    };

    PluginMetadata {
        id,
        name,
        description: format!("MCP server wrapped as Nexus plugin: {}", mcp_command),
        author: String::new(),
    }
}

#[derive(Debug, thiserror::Error)]
pub enum McpWrapError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Timeout waiting for MCP server response")]
    Timeout,

    #[error("MCP server exited (code {0}) before returning tools")]
    ServerExited(i32),

    #[error("Unsupported runtime: {0}. Only npx and node are supported.")]
    UnsupportedRuntime(String),

    #[error("{0}")]
    Other(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_npm_package() {
        assert_eq!(
            extract_npm_package("npx -y @upstash/context7-mcp"),
            Some("@upstash/context7-mcp".to_string())
        );
        assert_eq!(
            extract_npm_package("npx @modelcontextprotocol/server-everything"),
            Some("@modelcontextprotocol/server-everything".to_string())
        );
        assert_eq!(extract_npm_package("npx -y"), None);
    }

    #[test]
    fn test_suggest_metadata() {
        let meta = suggest_metadata("npx -y @upstash/context7-mcp");
        assert_eq!(meta.id, "mcp.context7-mcp");
        assert_eq!(meta.name, "context7-mcp (MCP)");

        let meta2 = suggest_metadata("npx -y @modelcontextprotocol/server-filesystem");
        assert_eq!(meta2.id, "mcp.filesystem");
        assert_eq!(meta2.name, "filesystem (MCP)");
    }
}
