use crate::plugin_manager::storage::{McpPluginSettings, McpSettings};
use crate::AppState;
use serde::Serialize;
use tauri::Manager;

#[derive(Debug, Clone, Serialize)]
pub struct McpToolStatus {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub plugin_id: String,
    pub plugin_name: String,
    pub plugin_running: bool,
    pub mcp_global_enabled: bool,
    pub mcp_plugin_enabled: bool,
    pub tool_enabled: bool,
    pub required_permissions: Vec<String>,
    pub permissions_granted: bool,
    pub requires_approval: bool,
}

#[tauri::command]
pub async fn mcp_get_settings(
    state: tauri::State<'_, AppState>,
) -> Result<McpSettings, String> {
    let mgr = state.read().await;
    Ok(mgr.mcp_settings.clone())
}

#[tauri::command]
pub async fn mcp_set_enabled(
    state: tauri::State<'_, AppState>,
    scope: String,
    enabled: bool,
) -> Result<(), String> {
    let mut mgr = state.write().await;

    if scope == "global" {
        mgr.mcp_settings.enabled = enabled;
    } else if let Some(plugin_id) = scope.strip_prefix("plugin:") {
        mgr.mcp_settings
            .plugins
            .entry(plugin_id.to_string())
            .or_insert_with(|| McpPluginSettings {
                enabled: true,
                disabled_tools: vec![],
            })
            .enabled = enabled;
    } else if let Some(rest) = scope.strip_prefix("tool:") {
        // Format: "tool:{plugin_id}.{tool_name}"
        // Find the split point — plugin IDs contain dots, so find the matching plugin
        let mut found = false;
        let plugin_ids: Vec<String> = mgr
            .storage
            .list()
            .iter()
            .map(|p| p.manifest.id.clone())
            .collect();

        for pid in &plugin_ids {
            let prefix = format!("{}.", pid);
            if let Some(tool_name) = rest.strip_prefix(&prefix) {
                if !tool_name.is_empty() {
                    let plugin_settings = mgr
                        .mcp_settings
                        .plugins
                        .entry(pid.clone())
                        .or_insert_with(|| McpPluginSettings {
                            enabled: true,
                            disabled_tools: vec![],
                        });
                    if enabled {
                        plugin_settings
                            .disabled_tools
                            .retain(|t| t != tool_name);
                    } else if !plugin_settings.disabled_tools.contains(&tool_name.to_string()) {
                        plugin_settings
                            .disabled_tools
                            .push(tool_name.to_string());
                    }
                    found = true;
                    break;
                }
            }
        }

        if !found {
            return Err(format!("Unknown tool scope: {}", scope));
        }
    } else {
        return Err(format!("Invalid scope: {}. Expected 'global', 'plugin:{{id}}', or 'tool:{{plugin_id}}.{{tool_name}}'", scope));
    }

    mgr.mcp_settings.save().map_err(|e| e.to_string())?;
    mgr.notify_tools_changed();
    Ok(())
}

#[tauri::command]
pub async fn mcp_list_tools(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<McpToolStatus>, String> {
    let mgr = state.read().await;
    let mut tools = Vec::new();

    for plugin in mgr.storage.list() {
        let mcp_config = match &plugin.manifest.mcp {
            Some(c) => c,
            None => continue,
        };

        let plugin_running =
            plugin.status == crate::plugin_manager::storage::PluginStatus::Running;
        let plugin_mcp = mgr.mcp_settings.plugins.get(&plugin.manifest.id);
        let plugin_enabled = plugin_mcp.map_or(true, |s| s.enabled);

        for tool in &mcp_config.tools {
            let tool_disabled =
                plugin_mcp.is_some_and(|s| s.disabled_tools.contains(&tool.name));

            let all_perms_granted = tool.permissions.iter().all(|perm_str| {
                serde_json::from_value::<crate::permissions::Permission>(
                    serde_json::Value::String(perm_str.clone()),
                )
                .is_ok_and(|perm| {
                    mgr.permissions.has_permission(&plugin.manifest.id, &perm)
                })
            });

            tools.push(McpToolStatus {
                name: format!("{}.{}", plugin.manifest.id, tool.name),
                description: tool.description.clone(),
                input_schema: tool.input_schema.clone(),
                plugin_id: plugin.manifest.id.clone(),
                plugin_name: plugin.manifest.name.clone(),
                plugin_running,
                mcp_global_enabled: mgr.mcp_settings.enabled,
                mcp_plugin_enabled: plugin_enabled,
                tool_enabled: !tool_disabled,
                required_permissions: tool.permissions.clone(),
                permissions_granted: all_perms_granted,
                requires_approval: tool.requires_approval,
            });
        }
    }

    Ok(tools)
}

#[tauri::command]
pub async fn mcp_config_snippet(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mgr = state.read().await;

    let token_path = mgr.data_dir.join("mcp_gateway_token");
    let token = std::fs::read_to_string(&token_path)
        .map_err(|e| format!("Failed to read gateway token: {}", e))?;

    // Tauri bakes the target triple at compile time and places sidecars at:
    //   {resource_dir}/binaries/{name}-{TAURI_ENV_TARGET_TRIPLE}[.exe]
    let target_triple = env!("TAURI_ENV_TARGET_TRIPLE");
    let exe_suffix = if target_triple.contains("windows") { ".exe" } else { "" };
    let sidecar_name = format!("nexus-mcp-{}{}", target_triple, exe_suffix);

    let sidecar_path = app
        .path()
        .resource_dir()
        .map(|dir| dir.join("binaries").join(&sidecar_name))
        .map_err(|e| format!("Failed to resolve resource dir: {}", e))?;

    let command = if sidecar_path.exists() {
        sidecar_path.display().to_string()
    } else {
        // Dev mode — binary may not be in the resource dir yet.
        // Fall back to the build output in src-tauri/binaries/.
        let dev_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("binaries")
            .join(&sidecar_name);
        if dev_path.exists() {
            dev_path.display().to_string()
        } else {
            // Last resort: assume it's on PATH
            "nexus-mcp".to_string()
        }
    };

    let token_trimmed = token.trim();

    let desktop_config = serde_json::json!({
        "mcpServers": {
            "nexus": {
                "command": command,
                "args": [],
                "env": {
                    "NEXUS_GATEWAY_TOKEN": token_trimmed,
                    "NEXUS_API_URL": "http://localhost:9600"
                }
            }
        }
    });

    let claude_code_command = format!(
        "claude mcp add \\\n  -e NEXUS_GATEWAY_TOKEN={} \\\n  -e NEXUS_API_URL=http://localhost:9600 \\\n  -- nexus {}",
        token_trimmed, command
    );

    Ok(serde_json::json!({
        "desktop_config": desktop_config,
        "claude_code_command": claude_code_command
    }))
}
