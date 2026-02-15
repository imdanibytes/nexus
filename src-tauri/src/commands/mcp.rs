use crate::plugin_manager::storage::{McpPluginSettings, McpSettings};
use crate::AppState;
use serde::Serialize;

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
                approved_tools: vec![],
                disabled_resources: vec![],
                disabled_prompts: vec![],
            })
            .enabled = enabled;
    } else if let Some(rest) = scope.strip_prefix("tool:") {
        // Format: "tool:{plugin_id}.{tool_name}"
        // Find the split point — plugin IDs contain dots, so find the matching plugin
        let mut found = false;
        let mut plugin_ids: Vec<String> = mgr
            .storage
            .list()
            .iter()
            .map(|p| p.manifest.id.clone())
            .collect();
        // Include the virtual "nexus" plugin for built-in MCP tools
        plugin_ids.push("nexus".to_string());

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
                            approved_tools: vec![],
                            disabled_resources: vec![],
                            disabled_prompts: vec![],
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

    // Append built-in Nexus management tools
    let nexus_mcp = mgr.mcp_settings.plugins.get("nexus");
    let nexus_plugin_enabled = nexus_mcp.map_or(true, |s| s.enabled);
    for builtin in crate::host_api::nexus_mcp::builtin_tools() {
        let local_name = builtin.name.strip_prefix("nexus.").unwrap_or(&builtin.name);
        let tool_disabled =
            nexus_mcp.is_some_and(|s| s.disabled_tools.contains(&local_name.to_string()));

        tools.push(McpToolStatus {
            name: builtin.name,
            description: builtin.description,
            input_schema: builtin.input_schema,
            plugin_id: "nexus".to_string(),
            plugin_name: "Nexus".to_string(),
            plugin_running: true,
            mcp_global_enabled: mgr.mcp_settings.enabled,
            mcp_plugin_enabled: nexus_plugin_enabled,
            tool_enabled: !tool_disabled,
            required_permissions: vec![],
            permissions_granted: true,
            requires_approval: builtin.requires_approval,
        });
    }

    Ok(tools)
}

#[tauri::command]
pub async fn mcp_config_snippet(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mgr = state.read().await;

    let token_path = mgr.data_dir.join("mcp_gateway_token");
    let token = std::fs::read_to_string(&token_path)
        .map_err(|e| format!("Failed to read gateway token: {}", e))?;

    let token_trimmed = token.trim();

    let desktop_config = serde_json::json!({
        "mcpServers": {
            "nexus": {
                "url": "http://127.0.0.1:9600/mcp",
                "headers": {
                    "X-Nexus-Gateway-Token": token_trimmed
                }
            }
        }
    });

    let claude_code_command = format!(
        "claude mcp add -t http \\\n  nexus http://127.0.0.1:9600/mcp \\\n  -H \"X-Nexus-Gateway-Token: {}\"",
        token_trimmed
    );

    Ok(serde_json::json!({
        "desktop_config": desktop_config,
        "claude_code_command": claude_code_command
    }))
}
