use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{
        sse::{Event, KeepAlive, Sse},
        Response,
    },
    Extension, Json,
};
use futures_util::stream::Stream;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::sync::Arc;

use crate::permissions::Permission;
use crate::plugin_manager::storage::PluginStatus;
use crate::AppState;

use super::approval::ApprovalBridge;
use super::auth::SessionStore;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct McpToolEntry {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub plugin_id: String,
    pub plugin_name: String,
    pub required_permissions: Vec<String>,
    pub permissions_granted: bool,
    pub enabled: bool,
    pub requires_approval: bool,
}

#[derive(Debug, Deserialize)]
pub struct McpCallRequest {
    pub tool_name: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct McpCallResponse {
    pub content: Vec<McpContent>,
    pub is_error: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct McpContent {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: String,
}

// ---------------------------------------------------------------------------
// Gateway auth middleware
// ---------------------------------------------------------------------------

pub async fn gateway_auth_middleware(
    State(state): State<AppState>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // Try X-Nexus-Gateway-Token first (MCP sidecar path)
    if let Some(token) = req
        .headers()
        .get("X-Nexus-Gateway-Token")
        .and_then(|v| v.to_str().ok())
    {
        let mgr = state.read().await;
        if mgr.verify_gateway_token(token) {
            drop(mgr);
            return Ok(next.run(req).await);
        }
    }

    // Fall back to Bearer token (plugin path — requires mcp:call permission)
    if let Some(bearer) = req
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
    {
        let sessions = req
            .extensions()
            .get::<Arc<SessionStore>>()
            .cloned()
            .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

        if let Some(plugin_id) = sessions.validate(bearer) {
            let mgr = state.read().await;
            if mgr
                .permissions
                .has_permission(&plugin_id, &Permission::McpCall)
            {
                drop(mgr);
                return Ok(next.run(req).await);
            }
            log::warn!(
                "AUDIT plugin={} tried MCP access without mcp:call permission",
                plugin_id
            );
            return Err(StatusCode::FORBIDDEN);
        }
    }

    Err(StatusCode::UNAUTHORIZED)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// List all MCP tools from all installed plugins, with status info.
pub async fn list_tools(State(state): State<AppState>) -> Json<Vec<McpToolEntry>> {
    let mgr = state.read().await;

    let mut entries = Vec::new();

    if !mgr.mcp_settings.enabled {
        return Json(entries);
    }

    for plugin in mgr.storage.list() {
        // Only running plugins with an mcp section
        if plugin.status != PluginStatus::Running {
            continue;
        }
        let mcp_config = match &plugin.manifest.mcp {
            Some(c) => c,
            None => continue,
        };

        let plugin_mcp = mgr.mcp_settings.plugins.get(&plugin.manifest.id);
        let plugin_enabled = plugin_mcp.map_or(true, |s| s.enabled);

        for tool in &mcp_config.tools {
            let tool_disabled = plugin_mcp
                .is_some_and(|s| s.disabled_tools.contains(&tool.name));

            // Check permissions
            let all_perms_granted = tool.permissions.iter().all(|perm_str| {
                serde_json::from_value::<Permission>(serde_json::Value::String(perm_str.clone()))
                    .is_ok_and(|perm| {
                        mgr.permissions.has_permission(&plugin.manifest.id, &perm)
                    })
            });

            let namespaced_name = format!("{}.{}", plugin.manifest.id, tool.name);

            entries.push(McpToolEntry {
                name: namespaced_name,
                description: tool.description.clone(),
                input_schema: tool.input_schema.clone(),
                plugin_id: plugin.manifest.id.clone(),
                plugin_name: plugin.manifest.name.clone(),
                required_permissions: tool.permissions.clone(),
                permissions_granted: all_perms_granted,
                enabled: plugin_enabled && !tool_disabled && all_perms_granted,
                requires_approval: tool.requires_approval,
            });
        }
    }

    Json(entries)
}

/// Call an MCP tool by its namespaced name.
pub async fn call_tool(
    State(state): State<AppState>,
    Extension(bridge): Extension<Arc<ApprovalBridge>>,
    Json(req): Json<McpCallRequest>,
) -> Result<Json<McpCallResponse>, StatusCode> {
    let mgr = state.read().await;

    if !mgr.mcp_settings.enabled {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }

    // Resolve namespaced tool name to plugin + local tool name.
    // Plugin IDs can contain dots (e.g. "com.nexus.hello-world"), so we find
    // the longest matching plugin ID prefix.
    let mut matched_plugin_id: Option<String> = None;
    let mut local_tool_name: Option<String> = None;

    for plugin in mgr.storage.list() {
        let prefix = format!("{}.", plugin.manifest.id);
        if req.tool_name.starts_with(&prefix) {
            let candidate_local = &req.tool_name[prefix.len()..];
            // Longest prefix match
            if matched_plugin_id
                .as_ref()
                .map_or(true, |prev| plugin.manifest.id.len() > prev.len())
            {
                matched_plugin_id = Some(plugin.manifest.id.clone());
                local_tool_name = Some(candidate_local.to_string());
            }
        }
    }

    let plugin_id = matched_plugin_id.ok_or(StatusCode::NOT_FOUND)?;
    let local_name = local_tool_name.ok_or(StatusCode::NOT_FOUND)?;

    let plugin = mgr.storage.get(&plugin_id).ok_or(StatusCode::NOT_FOUND)?;

    // Plugin must be running
    if plugin.status != PluginStatus::Running {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }

    // Plugin must have MCP config with this tool
    let mcp_config = plugin.manifest.mcp.as_ref().ok_or(StatusCode::NOT_FOUND)?;
    let tool_def = mcp_config
        .tools
        .iter()
        .find(|t| t.name == local_name)
        .ok_or(StatusCode::NOT_FOUND)?;

    // Check MCP enabled for this plugin
    let plugin_mcp = mgr.mcp_settings.plugins.get(&plugin_id);
    let plugin_enabled = plugin_mcp.map_or(true, |s| s.enabled);
    if !plugin_enabled {
        return Err(StatusCode::FORBIDDEN);
    }

    // Check tool not disabled
    let tool_disabled = plugin_mcp.is_some_and(|s| s.disabled_tools.contains(&local_name));
    if tool_disabled {
        return Err(StatusCode::FORBIDDEN);
    }

    // Check all required permissions are granted
    for perm_str in &tool_def.permissions {
        let perm = serde_json::from_value::<Permission>(serde_json::Value::String(
            perm_str.clone(),
        ))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        if !mgr.permissions.has_permission(&plugin_id, &perm) {
            return Err(StatusCode::FORBIDDEN);
        }
    }

    let requires_approval = tool_def.requires_approval;
    let plugin_name = plugin.manifest.name.clone();
    let tool_description = tool_def.description.clone();
    let port = plugin.assigned_port;

    // Check if the user has permanently approved this tool (via prior "Approve" click)
    let already_approved = requires_approval
        && mgr
            .mcp_settings
            .plugins
            .get(&plugin_id)
            .is_some_and(|s| s.approved_tools.contains(&local_name));

    drop(mgr);

    // Runtime approval for tools that require it (unless permanently approved)
    if requires_approval && !already_approved {
        let mut context = std::collections::HashMap::new();
        context.insert("tool_name".to_string(), local_name.clone());
        context.insert("plugin_name".to_string(), plugin_name.clone());
        context.insert("description".to_string(), tool_description);
        // Include argument summary so the user can see what the AI is requesting
        if let serde_json::Value::Object(map) = &req.arguments {
            for (k, v) in map {
                let display = match v {
                    serde_json::Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
                context.insert(format!("arg.{}", k), display);
            }
        }

        let approval_req = super::approval::ApprovalRequest {
            id: uuid::Uuid::new_v4().to_string(),
            plugin_id: plugin_id.clone(),
            plugin_name: plugin_name.clone(),
            category: "mcp_tool".to_string(),
            permission: format!("mcp:{}:{}", plugin_id, local_name),
            context,
        };

        let decision = bridge.request_approval(approval_req).await;

        match decision {
            super::approval::ApprovalDecision::Approve => {
                // Persist: future calls to this tool skip approval
                let mut mgr = state.write().await;
                let plugin_settings = mgr
                    .mcp_settings
                    .plugins
                    .entry(plugin_id.clone())
                    .or_insert_with(|| crate::plugin_manager::storage::McpPluginSettings {
                        enabled: true,
                        disabled_tools: vec![],
                        approved_tools: vec![],
                    });
                if !plugin_settings.approved_tools.contains(&local_name) {
                    plugin_settings.approved_tools.push(local_name.clone());
                }
                let _ = mgr.mcp_settings.save();
                drop(mgr);

                log::info!(
                    "AUDIT MCP tool permanently approved: plugin={} tool={}",
                    plugin_id, local_name
                );
            }
            super::approval::ApprovalDecision::ApproveOnce => {
                log::info!(
                    "AUDIT MCP tool approved once: plugin={} tool={}",
                    plugin_id, local_name
                );
            }
            super::approval::ApprovalDecision::Deny => {
                log::warn!(
                    "AUDIT MCP tool denied: plugin={} tool={}",
                    plugin_id, local_name
                );
                return Ok(Json(McpCallResponse {
                    content: vec![McpContent {
                        content_type: "text".to_string(),
                        text: format!(
                            "[Nexus] Tool '{}' was denied by the user.",
                            local_name
                        ),
                    }],
                    is_error: true,
                }));
            }
        }
    }

    // Forward to plugin container
    let client = reqwest::Client::new();
    let plugin_url = format!("http://localhost:{}/mcp/call", port);

    let forward_body = serde_json::json!({
        "tool_name": local_name,
        "arguments": req.arguments,
    });

    let resp = match client
        .post(&plugin_url)
        .json(&forward_body)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            log::error!("MCP call to plugin {} failed: {}", plugin_id, e);
            return Ok(Json(McpCallResponse {
                content: vec![McpContent {
                    content_type: "text".to_string(),
                    text: format!(
                        "[Nexus] Tool '{}' failed — the plugin '{}' is not responding.",
                        local_name, plugin_id
                    ),
                }],
                is_error: true,
            }));
        }
    };

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let body = resp.text().await.unwrap_or_default();
        log::error!(
            "MCP call to plugin {} returned {}: {}",
            plugin_id,
            status,
            body
        );
        return Ok(Json(McpCallResponse {
            content: vec![McpContent {
                content_type: "text".to_string(),
                text: format!(
                    "[Nexus] Tool '{}' failed — plugin '{}' returned an error.",
                    local_name, plugin_id
                ),
            }],
            is_error: true,
        }));
    }

    let call_resp: McpCallResponse = match resp.json().await {
        Ok(r) => r,
        Err(e) => {
            log::error!("Failed to parse MCP response from plugin {}: {}", plugin_id, e);
            return Ok(Json(McpCallResponse {
                content: vec![McpContent {
                    content_type: "text".to_string(),
                    text: format!(
                        "[Nexus] Tool '{}' failed — plugin '{}' returned an invalid response.",
                        local_name, plugin_id
                    ),
                }],
                is_error: true,
            }));
        }
    };

    // Wrap plugin-reported errors with source context
    if call_resp.is_error {
        let wrapped_content: Vec<McpContent> = call_resp
            .content
            .into_iter()
            .map(|item| McpContent {
                content_type: item.content_type,
                text: format!(
                    "[Nexus] Plugin '{}' reported an error for tool '{}': {}",
                    plugin_id, local_name, item.text
                ),
            })
            .collect();
        return Ok(Json(McpCallResponse {
            content: wrapped_content,
            is_error: true,
        }));
    }

    Ok(Json(call_resp))
}

/// SSE endpoint that emits a `tools_changed` event whenever the MCP tool list
/// is modified (enable/disable, plugin start/stop/remove, permission changes).
pub async fn tool_events(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let mut rx = {
        let mgr = state.read().await;
        mgr.tool_version_rx.clone()
    };

    let stream = async_stream::stream! {
        loop {
            if rx.changed().await.is_err() {
                // Sender dropped — host is shutting down
                break;
            }
            let version = *rx.borrow_and_update();
            yield Ok(Event::default()
                .event("tools_changed")
                .data(version.to_string()));
        }
    };

    Sse::new(stream).keep_alive(KeepAlive::default())
}
