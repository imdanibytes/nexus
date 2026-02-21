use crate::api_keys::ApiKeyStore;
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
            .or_insert_with(McpPluginSettings::default)
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
        // Include extension IDs for extension MCP tools
        for ext_info in mgr.extensions.list() {
            if ext_info.operations.iter().any(|op| op.mcp_expose) {
                plugin_ids.push(ext_info.id);
            }
        }

        for pid in &plugin_ids {
            let prefix = format!("{}.", pid);
            if let Some(tool_name) = rest.strip_prefix(&prefix) {
                if !tool_name.is_empty() {
                    let plugin_settings = mgr
                        .mcp_settings
                        .plugins
                        .entry(pid.clone())
                        .or_insert_with(McpPluginSettings::default);
                    if enabled {
                        if !plugin_settings.enabled_tools.contains(&tool_name.to_string()) {
                            plugin_settings
                                .enabled_tools
                                .push(tool_name.to_string());
                        }
                    } else {
                        plugin_settings
                            .enabled_tools
                            .retain(|t| t != tool_name);
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
        let plugin_enabled = plugin_mcp.is_some_and(|s| s.enabled);

        for tool in &mcp_config.tools {
            let tool_in_whitelist =
                plugin_mcp.is_some_and(|s| s.enabled_tools.contains(&tool.name));

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
                tool_enabled: tool_in_whitelist,
                required_permissions: tool.permissions.clone(),
                permissions_granted: all_perms_granted,
                requires_approval: tool.requires_approval,
            });
        }
    }

    // Append built-in Nexus management tools
    let nexus_mcp = mgr.mcp_settings.plugins.get("nexus");
    let nexus_plugin_enabled = nexus_mcp.is_some_and(|s| s.enabled);
    let mcp_global_enabled = mgr.mcp_settings.enabled;
    for builtin in crate::host_api::mcp::builtin::builtin_tools() {
        let local_name = builtin.name.strip_prefix("nexus.").unwrap_or(&builtin.name);
        let tool_in_whitelist =
            nexus_mcp.is_some_and(|s| s.enabled_tools.contains(&local_name.to_string()));

        tools.push(McpToolStatus {
            name: builtin.name,
            description: builtin.description,
            input_schema: builtin.input_schema,
            plugin_id: "nexus".to_string(),
            plugin_name: "Nexus".to_string(),
            plugin_running: true,
            mcp_global_enabled,
            mcp_plugin_enabled: nexus_plugin_enabled,
            tool_enabled: tool_in_whitelist,
            required_permissions: vec![],
            permissions_granted: true,
            requires_approval: builtin.requires_approval,
        });
    }

    // Drop the read lock before calling extension_mcp_tools (which acquires its own)
    drop(mgr);

    // Append extension MCP tools (operations with mcp_expose: true)
    for ext_tool in crate::host_api::mcp::builtin::extension_mcp_tools(&state).await {
        let ext_id = &ext_tool.plugin_id;
        let local_name = ext_tool
            .name
            .strip_prefix(&format!("{}.", ext_id))
            .unwrap_or(&ext_tool.name);

        let mgr = state.read().await;
        let ext_mcp = mgr.mcp_settings.plugins.get(ext_id);
        let ext_plugin_enabled = ext_mcp.is_some_and(|s| s.enabled);
        let tool_in_whitelist =
            ext_mcp.is_some_and(|s| s.enabled_tools.contains(&local_name.to_string()));

        tools.push(McpToolStatus {
            name: ext_tool.name,
            description: ext_tool.description,
            input_schema: ext_tool.input_schema,
            plugin_id: ext_id.clone(),
            plugin_name: ext_tool.plugin_name,
            plugin_running: true, // extension is running if it's in the registry
            mcp_global_enabled,
            mcp_plugin_enabled: ext_plugin_enabled,
            tool_enabled: tool_in_whitelist,
            required_permissions: vec![],
            permissions_granted: true,
            requires_approval: ext_tool.requires_approval,
        });
    }

    Ok(tools)
}

#[tauri::command]
pub async fn mcp_config_snippet(
    _state: tauri::State<'_, AppState>,
    api_keys: tauri::State<'_, ApiKeyStore>,
) -> Result<serde_json::Value, String> {
    let default_key = api_keys.get_default_raw().unwrap_or_default();

    let desktop_config = serde_json::json!({
        "mcpServers": {
            "nexus": {
                "url": "http://127.0.0.1:9600/mcp",
                "headers": {
                    "Authorization": format!("Bearer {}", default_key)
                }
            }
        }
    });

    let claude_code_command = format!(
        "claude mcp add -s user --transport http \\\n  nexus http://127.0.0.1:9600/mcp \\\n  -H \"Authorization: Bearer {}\"",
        default_key
    );

    let bearer = format!("Bearer {}", default_key);

    // Cursor: ~/.cursor/mcp.json — plain url + headers
    let cursor_config = serde_json::json!({
        "mcpServers": {
            "nexus": {
                "url": "http://127.0.0.1:9600/mcp",
                "headers": {
                    "Authorization": &bearer
                }
            }
        }
    });

    // Cline: cline_mcp_settings.json — requires "type": "streamableHttp" (camelCase)
    let cline_config = serde_json::json!({
        "mcpServers": {
            "nexus": {
                "type": "streamableHttp",
                "url": "http://127.0.0.1:9600/mcp",
                "headers": {
                    "Authorization": &bearer
                }
            }
        }
    });

    // Kiro: ~/.kiro/settings/mcp.json — plain url + headers
    let kiro_config = serde_json::json!({
        "mcpServers": {
            "nexus": {
                "url": "http://127.0.0.1:9600/mcp",
                "headers": {
                    "Authorization": &bearer
                }
            }
        }
    });

    Ok(serde_json::json!({
        "desktop_config": desktop_config,
        "claude_code_command": claude_code_command,
        "cursor_config": cursor_config,
        "cline_config": cline_config,
        "kiro_config": kiro_config
    }))
}
