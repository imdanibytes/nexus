//! Built-in Nexus MCP tools.
//!
//! Injects host-level management tools into the MCP tool list so AI clients
//! can list/start/stop plugins, manage extensions, search the marketplace,
//! and inspect settings — all without the user clicking through the UI.
//!
//! Tools use the virtual plugin ID `nexus` and are namespaced as `nexus.{tool}`.
//! Read-only tools need no approval; mutating tools go through the ApprovalBridge.

use std::sync::Arc;

use axum::http::StatusCode;
use serde_json::json;

use super::approval::{ApprovalBridge, ApprovalDecision, ApprovalRequest};
use super::mcp::{McpCallResponse, McpContent, McpToolEntry};
use crate::plugin_manager::storage::McpPluginSettings;
use crate::AppState;

/// Virtual plugin ID for built-in tools.
const NEXUS_PLUGIN_ID: &str = "nexus";
const NEXUS_PLUGIN_NAME: &str = "Nexus";

// ---------------------------------------------------------------------------
// Tool catalog
// ---------------------------------------------------------------------------

/// Returns the built-in tool definitions injected into `list_tools()`.
pub fn builtin_tools() -> Vec<McpToolEntry> {
    vec![
        // -- Read-only tools --
        McpToolEntry {
            name: "nexus.list_plugins".into(),
            description: "List all installed plugins with their status, version, port, and dev mode flag.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
            plugin_id: NEXUS_PLUGIN_ID.into(),
            plugin_name: NEXUS_PLUGIN_NAME.into(),
            required_permissions: vec![],
            permissions_granted: true,
            enabled: true,
            requires_approval: false,
        },
        McpToolEntry {
            name: "nexus.plugin_logs".into(),
            description: "Get recent log lines from a plugin's Docker container.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "plugin_id": {
                        "type": "string",
                        "description": "The plugin ID to fetch logs for."
                    },
                    "tail": {
                        "type": "integer",
                        "description": "Number of recent lines to return (default: 100)."
                    }
                },
                "required": ["plugin_id"],
                "additionalProperties": false
            }),
            plugin_id: NEXUS_PLUGIN_ID.into(),
            plugin_name: NEXUS_PLUGIN_NAME.into(),
            required_permissions: vec![],
            permissions_granted: true,
            enabled: true,
            requires_approval: false,
        },
        McpToolEntry {
            name: "nexus.list_extensions".into(),
            description: "List all host extensions with their enabled/running status and operations.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
            plugin_id: NEXUS_PLUGIN_ID.into(),
            plugin_name: NEXUS_PLUGIN_NAME.into(),
            required_permissions: vec![],
            permissions_granted: true,
            enabled: true,
            requires_approval: false,
        },
        McpToolEntry {
            name: "nexus.search_marketplace".into(),
            description: "Search the plugin and extension registries by keyword.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search query string."
                    }
                },
                "required": ["query"],
                "additionalProperties": false
            }),
            plugin_id: NEXUS_PLUGIN_ID.into(),
            plugin_name: NEXUS_PLUGIN_NAME.into(),
            required_permissions: vec![],
            permissions_granted: true,
            enabled: true,
            requires_approval: false,
        },
        McpToolEntry {
            name: "nexus.get_settings".into(),
            description: "Get Nexus app settings (resource quotas, update interval).".into(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
            plugin_id: NEXUS_PLUGIN_ID.into(),
            plugin_name: NEXUS_PLUGIN_NAME.into(),
            required_permissions: vec![],
            permissions_granted: true,
            enabled: true,
            requires_approval: false,
        },
        McpToolEntry {
            name: "nexus.get_mcp_settings".into(),
            description: "Get MCP gateway settings (global enabled flag, per-plugin tool states).".into(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
            plugin_id: NEXUS_PLUGIN_ID.into(),
            plugin_name: NEXUS_PLUGIN_NAME.into(),
            required_permissions: vec![],
            permissions_granted: true,
            enabled: true,
            requires_approval: false,
        },
        McpToolEntry {
            name: "nexus.docker_status".into(),
            description: "Check if Docker is installed and the engine is running.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
            plugin_id: NEXUS_PLUGIN_ID.into(),
            plugin_name: NEXUS_PLUGIN_NAME.into(),
            required_permissions: vec![],
            permissions_granted: true,
            enabled: true,
            requires_approval: false,
        },
        // -- Mutating tools --
        McpToolEntry {
            name: "nexus.plugin_start".into(),
            description: "Start a stopped plugin.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "plugin_id": {
                        "type": "string",
                        "description": "The plugin ID to start."
                    }
                },
                "required": ["plugin_id"],
                "additionalProperties": false
            }),
            plugin_id: NEXUS_PLUGIN_ID.into(),
            plugin_name: NEXUS_PLUGIN_NAME.into(),
            required_permissions: vec![],
            permissions_granted: true,
            enabled: true,
            requires_approval: true,
        },
        McpToolEntry {
            name: "nexus.plugin_stop".into(),
            description: "Stop a running plugin.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "plugin_id": {
                        "type": "string",
                        "description": "The plugin ID to stop."
                    }
                },
                "required": ["plugin_id"],
                "additionalProperties": false
            }),
            plugin_id: NEXUS_PLUGIN_ID.into(),
            plugin_name: NEXUS_PLUGIN_NAME.into(),
            required_permissions: vec![],
            permissions_granted: true,
            enabled: true,
            requires_approval: true,
        },
        McpToolEntry {
            name: "nexus.plugin_remove".into(),
            description: "Remove an installed plugin (stops it first if running).".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "plugin_id": {
                        "type": "string",
                        "description": "The plugin ID to remove."
                    }
                },
                "required": ["plugin_id"],
                "additionalProperties": false
            }),
            plugin_id: NEXUS_PLUGIN_ID.into(),
            plugin_name: NEXUS_PLUGIN_NAME.into(),
            required_permissions: vec![],
            permissions_granted: true,
            enabled: true,
            requires_approval: true,
        },
        McpToolEntry {
            name: "nexus.plugin_install".into(),
            description: "Install a plugin from a registry manifest URL. Grants no permissions by default — the user must approve permissions through the UI after installation.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "manifest_url": {
                        "type": "string",
                        "description": "URL of the plugin manifest JSON."
                    }
                },
                "required": ["manifest_url"],
                "additionalProperties": false
            }),
            plugin_id: NEXUS_PLUGIN_ID.into(),
            plugin_name: NEXUS_PLUGIN_NAME.into(),
            required_permissions: vec![],
            permissions_granted: true,
            enabled: true,
            requires_approval: true,
        },
        McpToolEntry {
            name: "nexus.extension_enable".into(),
            description: "Enable a host extension (spawns the process and registers it).".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "ext_id": {
                        "type": "string",
                        "description": "The extension ID to enable."
                    }
                },
                "required": ["ext_id"],
                "additionalProperties": false
            }),
            plugin_id: NEXUS_PLUGIN_ID.into(),
            plugin_name: NEXUS_PLUGIN_NAME.into(),
            required_permissions: vec![],
            permissions_granted: true,
            enabled: true,
            requires_approval: true,
        },
        McpToolEntry {
            name: "nexus.extension_disable".into(),
            description: "Disable a host extension (stops the process and unregisters it).".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "ext_id": {
                        "type": "string",
                        "description": "The extension ID to disable."
                    }
                },
                "required": ["ext_id"],
                "additionalProperties": false
            }),
            plugin_id: NEXUS_PLUGIN_ID.into(),
            plugin_name: NEXUS_PLUGIN_NAME.into(),
            required_permissions: vec![],
            permissions_granted: true,
            enabled: true,
            requires_approval: true,
        },
    ]
}

// ---------------------------------------------------------------------------
// Call dispatch
// ---------------------------------------------------------------------------

/// Handle a `nexus.*` tool call. Returns the MCP response or an error status.
///
/// `tool_name` is the local name (after stripping the `nexus.` prefix).
pub async fn handle_call(
    tool_name: &str,
    arguments: &serde_json::Value,
    state: &AppState,
    bridge: &Arc<ApprovalBridge>,
) -> Result<McpCallResponse, StatusCode> {
    match tool_name {
        // Read-only
        "list_plugins" => handle_list_plugins(state).await,
        "plugin_logs" => handle_plugin_logs(tool_name, arguments, state).await,
        "list_extensions" => handle_list_extensions(state).await,
        "search_marketplace" => handle_search_marketplace(tool_name, arguments, state).await,
        "get_settings" => handle_get_settings(state).await,
        "get_mcp_settings" => handle_get_mcp_settings(state).await,
        "docker_status" => handle_docker_status(state).await,
        // Mutating (require approval)
        "plugin_start" | "plugin_stop" | "plugin_remove" | "plugin_install"
        | "extension_enable" | "extension_disable" => {
            handle_mutating(tool_name, arguments, state, bridge).await
        }
        _ => Err(StatusCode::NOT_FOUND),
    }
}

// ---------------------------------------------------------------------------
// Read-only handlers
// ---------------------------------------------------------------------------

async fn handle_list_plugins(state: &AppState) -> Result<McpCallResponse, StatusCode> {
    let mgr = state.read().await;
    let plugins: Vec<serde_json::Value> = mgr
        .storage
        .list()
        .iter()
        .map(|p| {
            json!({
                "id": p.manifest.id,
                "name": p.manifest.name,
                "version": p.manifest.version,
                "status": p.status,
                "port": p.assigned_port,
                "dev_mode": p.dev_mode,
                "description": p.manifest.description,
            })
        })
        .collect();

    ok_json(&plugins)
}

async fn handle_plugin_logs(
    _tool_name: &str,
    args: &serde_json::Value,
    state: &AppState,
) -> Result<McpCallResponse, StatusCode> {
    let plugin_id = args
        .get("plugin_id")
        .and_then(|v| v.as_str())
        .ok_or(StatusCode::BAD_REQUEST)?;
    let tail = args
        .get("tail")
        .and_then(|v| v.as_u64())
        .unwrap_or(100) as u32;

    let mgr = state.read().await;
    match mgr.logs(plugin_id, tail).await {
        Ok(lines) => ok_json(&lines),
        Err(e) => ok_error(format!("Failed to get logs for '{}': {}", plugin_id, e)),
    }
}

async fn handle_list_extensions(state: &AppState) -> Result<McpCallResponse, StatusCode> {
    let mgr = state.read().await;

    let mut extensions = Vec::new();

    // Running extensions from registry
    for ext_info in mgr.extensions.list() {
        let installed_ext = mgr.extension_loader.storage.get(&ext_info.id);
        extensions.push(json!({
            "id": ext_info.id,
            "display_name": ext_info.display_name,
            "description": ext_info.description,
            "installed": installed_ext.is_some(),
            "enabled": installed_ext.is_some_and(|e| e.enabled),
            "operations": ext_info.operations.iter().map(|op| json!({
                "name": op.name,
                "description": op.description,
                "risk_level": op.risk_level,
            })).collect::<Vec<_>>(),
        }));
    }

    // Installed-but-disabled extensions
    for installed in mgr.extension_loader.storage.list() {
        if !installed.enabled && !extensions.iter().any(|e| e["id"] == installed.manifest.id) {
            extensions.push(json!({
                "id": installed.manifest.id,
                "display_name": installed.manifest.display_name,
                "description": installed.manifest.description,
                "installed": true,
                "enabled": false,
                "operations": installed.manifest.operations.iter().map(|op| json!({
                    "name": op.name,
                    "description": op.description,
                    "risk_level": op.risk_level,
                })).collect::<Vec<_>>(),
            }));
        }
    }

    ok_json(&extensions)
}

async fn handle_search_marketplace(
    _tool_name: &str,
    args: &serde_json::Value,
    state: &AppState,
) -> Result<McpCallResponse, StatusCode> {
    let query = args
        .get("query")
        .and_then(|v| v.as_str())
        .ok_or(StatusCode::BAD_REQUEST)?;

    let mgr = state.read().await;
    let plugins = mgr.search_marketplace(query);
    let extensions = mgr.search_extension_marketplace(query);

    let results = json!({
        "plugins": plugins.iter().map(|p| json!({
            "id": p.id,
            "name": p.name,
            "version": p.version,
            "description": p.description,
            "manifest_url": p.manifest_url,
            "categories": p.categories,
        })).collect::<Vec<_>>(),
        "extensions": extensions.iter().map(|e| json!({
            "id": e.id,
            "name": e.name,
            "version": e.version,
            "description": e.description,
            "manifest_url": e.manifest_url,
            "categories": e.categories,
        })).collect::<Vec<_>>(),
    });

    ok_json(&results)
}

async fn handle_get_settings(state: &AppState) -> Result<McpCallResponse, StatusCode> {
    let mgr = state.read().await;
    let settings = json!({
        "cpu_quota_percent": mgr.settings.cpu_quota_percent,
        "memory_limit_mb": mgr.settings.memory_limit_mb,
        "update_check_interval_minutes": mgr.settings.update_check_interval_minutes,
    });
    ok_json(&settings)
}

async fn handle_get_mcp_settings(state: &AppState) -> Result<McpCallResponse, StatusCode> {
    let mgr = state.read().await;
    ok_json(&mgr.mcp_settings)
}

async fn handle_docker_status(state: &AppState) -> Result<McpCallResponse, StatusCode> {
    let runtime = { state.read().await.runtime.clone() };

    match tokio::time::timeout(std::time::Duration::from_secs(3), runtime.ping()).await {
        Ok(Ok(_)) => {
            let version = runtime.version().await.unwrap_or(None);
            ok_json(&json!({
                "installed": true,
                "running": true,
                "version": version,
            }))
        }
        Ok(Err(e)) => ok_json(&json!({
            "installed": true,
            "running": false,
            "message": format!("Docker not responding: {}", e),
        })),
        Err(_) => ok_json(&json!({
            "installed": true,
            "running": false,
            "message": "Docker connection timed out",
        })),
    }
}

// ---------------------------------------------------------------------------
// Mutating handlers (with approval)
// ---------------------------------------------------------------------------

async fn handle_mutating(
    tool_name: &str,
    arguments: &serde_json::Value,
    state: &AppState,
    bridge: &Arc<ApprovalBridge>,
) -> Result<McpCallResponse, StatusCode> {
    // Check if already permanently approved
    let already_approved = {
        let mgr = state.read().await;
        mgr.mcp_settings
            .plugins
            .get(NEXUS_PLUGIN_ID)
            .is_some_and(|s| s.approved_tools.contains(&tool_name.to_string()))
    };

    if !already_approved {
        // Build context for the approval dialog
        let mut context = std::collections::HashMap::new();
        context.insert("tool_name".to_string(), tool_name.to_string());
        context.insert("plugin_name".to_string(), NEXUS_PLUGIN_NAME.to_string());
        context.insert(
            "description".to_string(),
            describe_mutating_tool(tool_name),
        );

        // Include arguments so the user sees what the AI is requesting
        if let serde_json::Value::Object(map) = arguments {
            for (k, v) in map {
                let display = match v {
                    serde_json::Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
                context.insert(format!("arg.{}", k), display);
            }
        }

        let approval_req = ApprovalRequest {
            id: uuid::Uuid::new_v4().to_string(),
            plugin_id: NEXUS_PLUGIN_ID.to_string(),
            plugin_name: NEXUS_PLUGIN_NAME.to_string(),
            category: "mcp_tool".to_string(),
            permission: format!("mcp:nexus:{}", tool_name),
            context,
        };

        match bridge.request_approval(approval_req).await {
            ApprovalDecision::Approve => {
                // Persist permanent approval
                let mut mgr = state.write().await;
                let plugin_settings = mgr
                    .mcp_settings
                    .plugins
                    .entry(NEXUS_PLUGIN_ID.to_string())
                    .or_insert_with(|| McpPluginSettings {
                        enabled: true,
                        disabled_tools: vec![],
                        approved_tools: vec![],
                        disabled_resources: vec![],
                        disabled_prompts: vec![],
                    });
                if !plugin_settings
                    .approved_tools
                    .contains(&tool_name.to_string())
                {
                    plugin_settings.approved_tools.push(tool_name.to_string());
                }
                let _ = mgr.mcp_settings.save();
                drop(mgr);

                log::info!(
                    "AUDIT Nexus MCP tool permanently approved: tool={}",
                    tool_name
                );
            }
            ApprovalDecision::ApproveOnce => {
                log::info!("AUDIT Nexus MCP tool approved once: tool={}", tool_name);
            }
            ApprovalDecision::Deny => {
                log::warn!("AUDIT Nexus MCP tool denied: tool={}", tool_name);
                return ok_error(format!(
                    "[Nexus] Tool 'nexus.{}' was denied by the user.",
                    tool_name
                ));
            }
        }
    }

    // Dispatch to the actual handler
    match tool_name {
        "plugin_start" => exec_plugin_start(arguments, state).await,
        "plugin_stop" => exec_plugin_stop(arguments, state).await,
        "plugin_remove" => exec_plugin_remove(arguments, state).await,
        "plugin_install" => exec_plugin_install(arguments, state).await,
        "extension_enable" => exec_extension_enable(arguments, state).await,
        "extension_disable" => exec_extension_disable(arguments, state).await,
        _ => Err(StatusCode::NOT_FOUND),
    }
}

fn describe_mutating_tool(tool_name: &str) -> String {
    match tool_name {
        "plugin_start" => "Start a stopped plugin".into(),
        "plugin_stop" => "Stop a running plugin".into(),
        "plugin_remove" => "Remove an installed plugin".into(),
        "plugin_install" => "Install a plugin from a registry manifest URL".into(),
        "extension_enable" => "Enable a host extension".into(),
        "extension_disable" => "Disable a host extension".into(),
        _ => tool_name.to_string(),
    }
}

async fn exec_plugin_start(
    args: &serde_json::Value,
    state: &AppState,
) -> Result<McpCallResponse, StatusCode> {
    let plugin_id = require_str(args, "plugin_id")?;
    let mut mgr = state.write().await;
    match mgr.start(&plugin_id).await {
        Ok(()) => {
            mgr.notify_tools_changed();
            ok_json(&json!({ "status": "started", "plugin_id": plugin_id }))
        }
        Err(e) => ok_error(format!("Failed to start '{}': {}", plugin_id, e)),
    }
}

async fn exec_plugin_stop(
    args: &serde_json::Value,
    state: &AppState,
) -> Result<McpCallResponse, StatusCode> {
    let plugin_id = require_str(args, "plugin_id")?;
    let mut mgr = state.write().await;
    match mgr.stop(&plugin_id).await {
        Ok(()) => {
            mgr.notify_tools_changed();
            ok_json(&json!({ "status": "stopped", "plugin_id": plugin_id }))
        }
        Err(e) => ok_error(format!("Failed to stop '{}': {}", plugin_id, e)),
    }
}

async fn exec_plugin_remove(
    args: &serde_json::Value,
    state: &AppState,
) -> Result<McpCallResponse, StatusCode> {
    let plugin_id = require_str(args, "plugin_id")?;
    let mut mgr = state.write().await;
    match mgr.remove(&plugin_id).await {
        Ok(()) => {
            mgr.notify_tools_changed();
            ok_json(&json!({ "status": "removed", "plugin_id": plugin_id }))
        }
        Err(e) => ok_error(format!("Failed to remove '{}': {}", plugin_id, e)),
    }
}

async fn exec_plugin_install(
    args: &serde_json::Value,
    state: &AppState,
) -> Result<McpCallResponse, StatusCode> {
    let manifest_url = require_str(args, "manifest_url")?;

    // Fetch and validate the manifest
    let manifest = match crate::plugin_manager::registry::fetch_manifest(&manifest_url).await {
        Ok(m) => m,
        Err(e) => return ok_error(format!("Failed to fetch manifest: {}", e)),
    };
    if let Err(e) = manifest.validate() {
        return ok_error(format!("Invalid manifest: {}", e));
    }

    let plugin_id = manifest.id.clone();
    let plugin_name = manifest.name.clone();

    // Install with no permissions — user must approve through the UI
    let mut mgr = state.write().await;
    match mgr
        .install(manifest, vec![], vec![], Some(&manifest_url), None)
        .await
    {
        Ok(_plugin) => {
            mgr.notify_tools_changed();
            ok_json(&json!({
                "status": "installed",
                "plugin_id": plugin_id,
                "plugin_name": plugin_name,
                "note": "Plugin installed with no permissions. Use the Nexus UI to grant permissions and start the plugin."
            }))
        }
        Err(e) => ok_error(format!("Failed to install '{}': {}", plugin_id, e)),
    }
}

async fn exec_extension_enable(
    args: &serde_json::Value,
    state: &AppState,
) -> Result<McpCallResponse, StatusCode> {
    let ext_id = require_str(args, "ext_id")?;
    let mut mgr = state.write().await;
    match mgr.enable_extension(&ext_id) {
        Ok(()) => ok_json(&json!({ "status": "enabled", "ext_id": ext_id })),
        Err(e) => ok_error(format!("Failed to enable '{}': {}", ext_id, e)),
    }
}

async fn exec_extension_disable(
    args: &serde_json::Value,
    state: &AppState,
) -> Result<McpCallResponse, StatusCode> {
    let ext_id = require_str(args, "ext_id")?;
    let mut mgr = state.write().await;
    match mgr.disable_extension(&ext_id) {
        Ok(()) => ok_json(&json!({ "status": "disabled", "ext_id": ext_id })),
        Err(e) => ok_error(format!("Failed to disable '{}': {}", ext_id, e)),
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn require_str(args: &serde_json::Value, key: &str) -> Result<String, StatusCode> {
    args.get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or(StatusCode::BAD_REQUEST)
}

fn ok_json<T: serde::Serialize>(value: &T) -> Result<McpCallResponse, StatusCode> {
    let text = serde_json::to_string_pretty(value).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(McpCallResponse {
        content: vec![McpContent {
            content_type: "text".into(),
            text,
        }],
        is_error: false,
    })
}

fn ok_error(message: String) -> Result<McpCallResponse, StatusCode> {
    Ok(McpCallResponse {
        content: vec![McpContent {
            content_type: "text".into(),
            text: message,
        }],
        is_error: true,
    })
}
