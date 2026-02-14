use super::classify::ClassifiedTool;
use super::{McpWrapError, PluginMetadata};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// The MCP bridge server source, embedded at compile time.
const BRIDGE_SERVER_JS: &str = include_str!("../../../tools/mcp-bridge/src/server.js");

/// Extract an npm package name from an MCP server command for the package.json dependency.
fn extract_npm_package(cmd: &str) -> Option<String> {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    for part in parts.iter().skip(1) {
        if part.starts_with('-') {
            continue;
        }
        if part.starts_with('@') || part.chars().next().is_some_and(|c| c.is_ascii_lowercase()) {
            return Some(part.to_string());
        }
    }
    None
}

/// Generate a complete plugin directory for an MCP wrapper plugin.
///
/// Writes to `{output_dir}/{plugin_id}/`:
/// - `plugin.json`  — headless manifest
/// - `package.json` — bridge + MCP server dependencies
/// - `src/server.js` — MCP bridge server
/// - `Dockerfile`   — Node 20 Alpine
///
/// Returns the path to the generated plugin directory.
pub fn generate_plugin(
    tools: &[ClassifiedTool],
    metadata: &PluginMetadata,
    mcp_command: &str,
    output_dir: &Path,
) -> Result<PathBuf, McpWrapError> {
    let plugin_dir = output_dir.join(&metadata.id);

    // If it already exists, remove and regenerate
    if plugin_dir.exists() {
        std::fs::remove_dir_all(&plugin_dir)?;
    }

    let src_dir = plugin_dir.join("src");
    std::fs::create_dir_all(&src_dir)?;

    // Union of all tool permissions
    let all_permissions: Vec<String> = tools
        .iter()
        .flat_map(|t| t.permissions.iter().cloned())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    // plugin.json — headless manifest
    let manifest = serde_json::json!({
        "id": metadata.id,
        "name": metadata.name,
        "version": "0.1.0",
        "description": metadata.description,
        "author": metadata.author,
        "license": "MIT",
        "image": format!("nexus-mcp-{}:latest", metadata.id.replace('.', "-")),
        "ui": null,
        "permissions": all_permissions,
        "health": {
            "endpoint": "/health",
            "interval_secs": 30
        },
        "env": {
            "MCP_SERVER_COMMAND": mcp_command
        },
        "mcp": {
            "tools": tools.iter().map(|t| serde_json::json!({
                "name": t.name,
                "description": t.description,
                "permissions": t.permissions,
                "input_schema": t.input_schema,
                "requires_approval": t.requires_approval
            })).collect::<Vec<_>>()
        }
    });
    std::fs::write(
        plugin_dir.join("plugin.json"),
        serde_json::to_string_pretty(&manifest)? + "\n",
    )?;

    // package.json
    let npm_pkg = extract_npm_package(mcp_command);
    let mut deps = serde_json::Map::new();
    deps.insert(
        "@modelcontextprotocol/sdk".to_string(),
        serde_json::json!("^1.12.1"),
    );
    if let Some(ref pkg) = npm_pkg {
        deps.insert(pkg.clone(), serde_json::json!("*"));
    }

    let package_json = serde_json::json!({
        "name": metadata.id,
        "version": "0.1.0",
        "description": metadata.description,
        "type": "module",
        "main": "src/server.js",
        "scripts": {
            "start": "node src/server.js"
        },
        "dependencies": deps
    });
    std::fs::write(
        plugin_dir.join("package.json"),
        serde_json::to_string_pretty(&package_json)? + "\n",
    )?;

    // src/server.js — embedded bridge
    std::fs::write(src_dir.join("server.js"), BRIDGE_SERVER_JS)?;

    // Dockerfile
    let dockerfile = "FROM node:20-alpine\n\
        \n\
        WORKDIR /app\n\
        \n\
        COPY package.json package-lock.json* ./\n\
        RUN npm install --production\n\
        \n\
        COPY src/ ./src/\n\
        \n\
        EXPOSE 80\n\
        \n\
        CMD [\"node\", \"src/server.js\"]\n";
    std::fs::write(plugin_dir.join("Dockerfile"), dockerfile)?;

    Ok(plugin_dir)
}
