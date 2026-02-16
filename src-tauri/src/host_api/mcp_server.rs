//! Host MCP server — the Nexus gateway as a native MCP endpoint.
//!
//! Implements `ServerHandler` to expose an aggregated view of all plugin
//! tools, resources, and prompts. Clients (Claude Desktop, Claude Code, etc.)
//! connect directly via streamable HTTP at `http://127.0.0.1:9600/mcp`.
//!
//! This replaces the custom HTTP protocol (`GET /mcp/tools`, `POST /mcp/call`,
//! `GET /mcp/events`) with a standards-compliant MCP server.

use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;

use rmcp::model::*;
use rmcp::service::{NotificationContext, RequestContext};
use rmcp::{ErrorData as McpError, RoleServer, ServerHandler};

use crate::permissions::Permission;
use crate::plugin_manager::storage::PluginStatus;
use crate::AppState;

use super::approval::{ApprovalBridge, ApprovalDecision, ApprovalRequest};

/// The Nexus host MCP server.
///
/// Aggregates tools/resources/prompts from:
/// 1. Legacy `mcp.tools` declarations (deprecated, custom HTTP protocol)
/// 2. Native MCP plugin servers (via McpClientManager)
/// 3. Built-in `nexus.*` management tools
pub struct NexusMcpServer {
    state: AppState,
    approval_bridge: Arc<ApprovalBridge>,
}

impl NexusMcpServer {
    pub fn new(state: AppState, approval_bridge: Arc<ApprovalBridge>) -> Self {
        Self {
            state,
            approval_bridge,
        }
    }
}

impl ServerHandler for NexusMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .enable_tool_list_changed()
                .enable_resources()
                .enable_resources_list_changed()
                .enable_prompts()
                .enable_prompts_list_changed()
                .build(),
            server_info: Implementation {
                name: "nexus".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                title: None,
                description: None,
                icons: None,
                website_url: None,
            },
            instructions: Some(
                "Nexus plugin hub — exposes tools, resources, and prompts from installed plugins."
                    .to_string(),
            ),
        }
    }

    fn on_initialized(
        &self,
        context: NotificationContext<RoleServer>,
    ) -> impl std::future::Future<Output = ()> + Send + '_ {
        let peer = context.peer;
        let state = self.state.clone();

        async move {
            let mut rx = {
                let mgr = state.read().await;
                mgr.tool_version_rx.clone()
            };

            // Background task: when the tool list version bumps,
            // send notifications/tools/list_changed to this MCP client.
            tokio::spawn(async move {
                loop {
                    if rx.changed().await.is_err() {
                        break; // sender dropped — host shutting down
                    }
                    if let Err(e) = peer.notify_tool_list_changed().await {
                        log::debug!("MCP tool_list_changed notification failed: {e}");
                        break; // client disconnected
                    }
                }
            });
        }
    }

    // ── Tools ────────────────────────────────────────────────────────

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        let mgr = self.state.read().await;
        let mut tools = Vec::new();

        if !mgr.mcp_settings.enabled {
            return Ok(ListToolsResult {
                tools,
                next_cursor: None,
                meta: None,
            });
        }

        // All tools use whitelist model — only visible if explicitly enabled.

        // 1. Legacy mcp.tools (deprecated path)
        for plugin in mgr.storage.list() {
            if plugin.status != PluginStatus::Running {
                continue;
            }
            let mcp_config = match &plugin.manifest.mcp {
                Some(c) => c,
                None => continue,
            };

            if mcp_config.server.is_some() && mcp_config.tools.is_empty() {
                continue;
            }

            let plugin_mcp = mgr.mcp_settings.plugins.get(&plugin.manifest.id);
            let plugin_enabled = plugin_mcp.is_some_and(|s| s.enabled);
            if !plugin_enabled {
                continue;
            }

            let all_perms_granted = plugin.manifest.permissions.iter().all(|perm| {
                mgr.permissions.has_permission(&plugin.manifest.id, perm)
            });
            if !all_perms_granted {
                continue;
            }

            for tool_def in &mcp_config.tools {
                let tool_enabled = plugin_mcp
                    .is_some_and(|s| s.enabled_tools.contains(&tool_def.name));
                if !tool_enabled {
                    continue;
                }

                let schema = match &tool_def.input_schema {
                    serde_json::Value::Object(map) => map.clone(),
                    _ => serde_json::Map::new(),
                };

                tools.push(Tool {
                    name: Cow::Owned(format!("{}.{}", plugin.manifest.id, tool_def.name)),
                    title: None,
                    description: Some(Cow::Owned(tool_def.description.clone())),
                    input_schema: Arc::new(schema),
                    output_schema: None,
                    annotations: None,
                    execution: None,
                    icons: None,
                    meta: None,
                });
            }
        }

        // 2. Native MCP plugin tools (from McpClientManager cache)
        for (plugin_id, cache) in mgr.mcp_clients.iter() {
            let plugin_mcp = mgr.mcp_settings.plugins.get(plugin_id);
            let plugin_enabled = plugin_mcp.is_some_and(|s| s.enabled);
            if !plugin_enabled {
                continue;
            }

            if let Some(plugin) = mgr.storage.get(plugin_id) {
                let all_perms_granted = plugin.manifest.permissions.iter().all(|perm| {
                    mgr.permissions.has_permission(plugin_id, perm)
                });
                if !all_perms_granted {
                    continue;
                }
            }

            for tool in &cache.tools {
                let local_name = tool.name.as_ref();
                let tool_enabled = plugin_mcp
                    .is_some_and(|s| s.enabled_tools.contains(&local_name.to_string()));
                if !tool_enabled {
                    continue;
                }

                tools.push(Tool {
                    name: Cow::Owned(format!("{}.{}", plugin_id, local_name)),
                    title: tool.title.clone(),
                    description: tool.description.clone(),
                    input_schema: tool.input_schema.clone(),
                    output_schema: tool.output_schema.clone(),
                    annotations: tool.annotations.clone(),
                    execution: tool.execution.clone(),
                    icons: tool.icons.clone(),
                    meta: tool.meta.clone(),
                });
            }
        }

        // 3. Built-in nexus.* tools
        let nexus_mcp = mgr.mcp_settings.plugins.get("nexus");
        let nexus_enabled = nexus_mcp.is_some_and(|s| s.enabled);
        if nexus_enabled {
            for builtin in super::nexus_mcp::builtin_tools() {
                let local_name = builtin
                    .name
                    .strip_prefix("nexus.")
                    .unwrap_or(&builtin.name);
                let tool_enabled = nexus_mcp
                    .is_some_and(|s| s.enabled_tools.contains(&local_name.to_string()));
                if !tool_enabled {
                    continue;
                }

                let schema = match builtin.input_schema {
                    serde_json::Value::Object(map) => map,
                    _ => serde_json::Map::new(),
                };

                tools.push(Tool {
                    name: Cow::Owned(builtin.name),
                    title: None,
                    description: Some(Cow::Owned(builtin.description)),
                    input_schema: Arc::new(schema),
                    output_schema: None,
                    annotations: None,
                    execution: None,
                    icons: None,
                    meta: None,
                });
            }
        }

        Ok(ListToolsResult {
            tools,
            next_cursor: None,
            meta: None,
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let tool_name = request.name.as_ref();

        // Built-in nexus.* tools
        if let Some(local_name) = tool_name.strip_prefix("nexus.") {
            let arguments = request
                .arguments
                .map(serde_json::Value::Object)
                .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

            let mgr = self.state.read().await;
            let nexus_mcp = mgr.mcp_settings.plugins.get("nexus");
            let tool_enabled = nexus_mcp
                .is_some_and(|s| s.enabled && s.enabled_tools.contains(&local_name.to_string()));
            if !tool_enabled {
                return Err(McpError::invalid_request(
                    format!("Tool 'nexus.{}' is not enabled", local_name),
                    None,
                ));
            }
            drop(mgr);

            return match super::nexus_mcp::handle_call(
                local_name,
                &arguments,
                &self.state,
                &self.approval_bridge,
            )
            .await
            {
                Ok(resp) => {
                    let content: Vec<Content> = resp
                        .content
                        .into_iter()
                        .map(|item| Content::text(item.text))
                        .collect();
                    if resp.is_error {
                        Ok(CallToolResult::error(content))
                    } else {
                        Ok(CallToolResult::success(content))
                    }
                }
                Err(_status) => Err(McpError::internal_error(
                    format!("Built-in tool 'nexus.{}' failed", local_name),
                    None,
                )),
            };
        }

        // Resolve namespaced tool → plugin_id + local_name
        let (plugin_id, local_name) = resolve_namespace(tool_name, &self.state).await?;

        let mgr = self.state.read().await;

        // Check plugin is running
        let plugin = mgr.storage.get(&plugin_id).ok_or_else(|| {
            McpError::invalid_request(format!("Plugin '{}' not found", plugin_id), None)
        })?;
        if plugin.status != PluginStatus::Running {
            return Err(McpError::invalid_request(
                format!("Plugin '{}' is not running", plugin_id),
                None,
            ));
        }

        // Check tool is explicitly enabled (whitelist model)
        let plugin_mcp = mgr.mcp_settings.plugins.get(&plugin_id);
        let tool_enabled = plugin_mcp
            .is_some_and(|s| s.enabled && s.enabled_tools.contains(&local_name));
        if !tool_enabled {
            return Err(McpError::invalid_request(
                format!("Tool '{}.{}' is not enabled", plugin_id, local_name),
                None,
            ));
        }

        // Determine routing: native MCP or legacy HTTP
        let is_native = mgr.mcp_clients.has(&plugin_id);
        let mcp_config = plugin.manifest.mcp.as_ref();

        // Check approval for native MCP tools
        let requires_approval = if is_native {
            mcp_config
                .and_then(|c| c.server.as_ref())
                .is_some_and(|s| s.requires_approval)
        } else {
            // Legacy: check per-tool requires_approval
            mcp_config
                .and_then(|c| c.tools.iter().find(|t| t.name == local_name))
                .is_some_and(|t| t.requires_approval)
        };

        // Check permissions for legacy tools
        if !is_native {
            if let Some(tool_def) = mcp_config
                .and_then(|c| c.tools.iter().find(|t| t.name == local_name))
            {
                for perm_str in &tool_def.permissions {
                    let perm = serde_json::from_value::<Permission>(
                        serde_json::Value::String(perm_str.clone()),
                    )
                    .map_err(|_| McpError::internal_error("Invalid permission", None))?;
                    if !mgr.permissions.has_permission(&plugin_id, &perm) {
                        return Err(McpError::invalid_request(
                            format!(
                                "Plugin '{}' does not have permission '{}'",
                                plugin_id, perm_str
                            ),
                            None,
                        ));
                    }
                }
            }
        }

        let plugin_name = plugin.manifest.name.clone();
        let port = plugin.assigned_port;
        let already_approved = requires_approval
            && plugin_mcp.is_some_and(|s| s.approved_tools.contains(&local_name));

        drop(mgr);

        // Runtime approval
        if requires_approval && !already_approved {
            let arguments_value = request
                .arguments
                .as_ref()
                .map(|m| serde_json::Value::Object(m.clone()))
                .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

            let mut context = HashMap::new();
            context.insert("tool_name".to_string(), local_name.clone());
            context.insert("plugin_name".to_string(), plugin_name.clone());
            if let serde_json::Value::Object(map) = &arguments_value {
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
                plugin_id: plugin_id.clone(),
                plugin_name: plugin_name.clone(),
                category: "mcp_tool".to_string(),
                permission: format!("mcp:{}:{}", plugin_id, local_name),
                context,
            };

            match self.approval_bridge.request_approval(approval_req).await {
                ApprovalDecision::Approve => {
                    let mut mgr = self.state.write().await;
                    let plugin_settings = mgr
                        .mcp_settings
                        .plugins
                        .entry(plugin_id.clone())
                        .or_insert_with(crate::plugin_manager::storage::McpPluginSettings::default);
                    if !plugin_settings.approved_tools.contains(&local_name) {
                        plugin_settings.approved_tools.push(local_name.clone());
                    }
                    let _ = mgr.mcp_settings.save();
                }
                ApprovalDecision::ApproveOnce => {}
                ApprovalDecision::Deny => {
                    return Ok(CallToolResult::error(vec![Content::text(format!(
                        "[Nexus] Tool '{}.{}' was denied by the user.",
                        plugin_id, local_name
                    ))]));
                }
            }
        }

        // Route the call
        if is_native {
            // Native MCP: forward via McpClientManager
            let mgr = self.state.read().await;
            match mgr
                .mcp_clients
                .call_tool(&plugin_id, &local_name, request.arguments)
                .await
            {
                Ok(result) => Ok(result),
                Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                    "[Nexus] Tool '{}.{}' failed: {}",
                    plugin_id, local_name, e
                ))])),
            }
        } else {
            // Legacy: forward via HTTP POST /mcp/call
            log::warn!(
                "DEPRECATED: Tool call via legacy HTTP protocol for plugin '{}'. \
                 Migrate to mcp.server for native MCP support.",
                plugin_id
            );

            let client = reqwest::Client::new();
            let plugin_url = format!("http://localhost:{}/mcp/call", port);
            let forward_body = serde_json::json!({
                "tool_name": local_name,
                "arguments": request.arguments.map(serde_json::Value::Object)
                    .unwrap_or(serde_json::Value::Object(serde_json::Map::new())),
            });

            match client
                .post(&plugin_url)
                .json(&forward_body)
                .timeout(std::time::Duration::from_secs(30))
                .send()
                .await
            {
                Ok(resp) if resp.status().is_success() => {
                    match resp
                        .json::<super::mcp::McpCallResponse>()
                        .await
                    {
                        Ok(call_resp) => {
                            let content: Vec<Content> = call_resp
                                .content
                                .into_iter()
                                .map(|item| Content::text(item.text))
                                .collect();
                            if call_resp.is_error {
                                Ok(CallToolResult::error(content))
                            } else {
                                Ok(CallToolResult::success(content))
                            }
                        }
                        Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                            "[Nexus] Plugin '{}' returned an invalid response: {}",
                            plugin_id, e
                        ))])),
                    }
                }
                Ok(resp) => Ok(CallToolResult::error(vec![Content::text(format!(
                    "[Nexus] Plugin '{}' returned HTTP {}",
                    plugin_id,
                    resp.status()
                ))])),
                Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                    "[Nexus] Plugin '{}' is not responding: {}",
                    plugin_id, e
                ))])),
            }
        }
    }

    // ── Resources ────────────────────────────────────────────────────

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        let mgr = self.state.read().await;
        let mut resources = Vec::new();

        if !mgr.mcp_settings.enabled {
            return Ok(ListResourcesResult {
                resources,
                next_cursor: None,
                meta: None,
            });
        }

        for (plugin_id, cache) in mgr.mcp_clients.iter() {
            let plugin_mcp = mgr.mcp_settings.plugins.get(plugin_id);
            let plugin_enabled = plugin_mcp.map_or(true, |s| s.enabled);
            if !plugin_enabled {
                continue;
            }

            // Check plugin-level permissions
            if let Some(plugin) = mgr.storage.get(plugin_id) {
                let all_perms_granted = plugin.manifest.permissions.iter().all(|perm| {
                    mgr.permissions.has_permission(plugin_id, perm)
                });
                if !all_perms_granted {
                    continue;
                }
            }

            for resource in &cache.resources {
                let uri_str = resource.uri.to_string();
                let disabled = plugin_mcp
                    .is_some_and(|s| s.disabled_resources.contains(&uri_str));
                if disabled {
                    continue;
                }

                resources.push(resource.clone());
            }
        }

        Ok(ListResourcesResult {
            resources,
            next_cursor: None,
            meta: None,
        })
    }

    async fn list_resource_templates(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourceTemplatesResult, McpError> {
        let mgr = self.state.read().await;
        let mut templates = Vec::new();

        if !mgr.mcp_settings.enabled {
            return Ok(ListResourceTemplatesResult {
                resource_templates: templates,
                next_cursor: None,
                meta: None,
            });
        }

        for (plugin_id, cache) in mgr.mcp_clients.iter() {
            let plugin_mcp = mgr.mcp_settings.plugins.get(plugin_id);
            let plugin_enabled = plugin_mcp.map_or(true, |s| s.enabled);
            if !plugin_enabled {
                continue;
            }

            for template in &cache.resource_templates {
                templates.push(template.clone());
            }
        }

        Ok(ListResourceTemplatesResult {
            resource_templates: templates,
            next_cursor: None,
            meta: None,
        })
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, McpError> {
        let uri_str = request.uri.clone();

        let mgr = self.state.read().await;

        // Find which plugin owns this resource by checking cached resource lists
        let mut owner_plugin_id: Option<String> = None;
        for (plugin_id, cache) in mgr.mcp_clients.iter() {
            if cache.resources.iter().any(|r| r.uri == uri_str) {
                owner_plugin_id = Some(plugin_id.to_string());
                break;
            }
        }

        let plugin_id = owner_plugin_id.ok_or_else(|| {
            McpError::invalid_request(
                format!("No plugin owns resource '{}'", uri_str),
                None,
            )
        })?;

        // Check plugin MCP enabled
        let plugin_mcp = mgr.mcp_settings.plugins.get(&plugin_id);
        if plugin_mcp.is_some_and(|s| !s.enabled) {
            return Err(McpError::invalid_request(
                format!("MCP is disabled for plugin '{}'", plugin_id),
                None,
            ));
        }
        if plugin_mcp.is_some_and(|s| s.disabled_resources.contains(&uri_str)) {
            return Err(McpError::invalid_request(
                format!("Resource '{}' is disabled", uri_str),
                None,
            ));
        }

        match mgr.mcp_clients.read_resource(&plugin_id, &uri_str).await {
            Ok(result) => Ok(result),
            Err(e) => Err(McpError::internal_error(
                format!("Failed to read resource '{}': {}", uri_str, e),
                None,
            )),
        }
    }

    // ── Prompts ──────────────────────────────────────────────────────

    async fn list_prompts(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListPromptsResult, McpError> {
        let mgr = self.state.read().await;
        let mut prompts = Vec::new();

        if !mgr.mcp_settings.enabled {
            return Ok(ListPromptsResult {
                prompts,
                next_cursor: None,
                meta: None,
            });
        }

        for (plugin_id, cache) in mgr.mcp_clients.iter() {
            let plugin_mcp = mgr.mcp_settings.plugins.get(plugin_id);
            let plugin_enabled = plugin_mcp.map_or(true, |s| s.enabled);
            if !plugin_enabled {
                continue;
            }

            for prompt in &cache.prompts {
                let local_name = prompt.name.clone();
                let disabled = plugin_mcp
                    .is_some_and(|s| s.disabled_prompts.contains(&local_name));
                if disabled {
                    continue;
                }

                prompts.push(Prompt {
                    name: format!("{}.{}", plugin_id, local_name),
                    title: prompt.title.clone(),
                    description: prompt.description.clone(),
                    arguments: prompt.arguments.clone(),
                    icons: prompt.icons.clone(),
                    meta: prompt.meta.clone(),
                });
            }
        }

        Ok(ListPromptsResult {
            prompts,
            next_cursor: None,
            meta: None,
        })
    }

    async fn get_prompt(
        &self,
        request: GetPromptRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<GetPromptResult, McpError> {
        let prompt_name = request.name.clone();

        let (plugin_id, local_name) = resolve_namespace(&prompt_name, &self.state).await?;

        let mgr = self.state.read().await;
        let plugin_mcp = mgr.mcp_settings.plugins.get(&plugin_id);
        if plugin_mcp.is_some_and(|s| !s.enabled) {
            return Err(McpError::invalid_request(
                format!("MCP is disabled for plugin '{}'", plugin_id),
                None,
            ));
        }
        if plugin_mcp.is_some_and(|s| s.disabled_prompts.contains(&local_name)) {
            return Err(McpError::invalid_request(
                format!("Prompt '{}.{}' is disabled", plugin_id, local_name),
                None,
            ));
        }

        match mgr
            .mcp_clients
            .get_prompt(&plugin_id, &local_name, request.arguments)
            .await
        {
            Ok(result) => Ok(result),
            Err(e) => Err(McpError::internal_error(
                format!("Failed to get prompt '{}.{}': {}", plugin_id, local_name, e),
                None,
            )),
        }
    }
}

// ── Helpers ──────────────────────────────────────────────────────────

/// Resolve a namespaced name (e.g., "com.nexus.hello-world.my_tool")
/// to (plugin_id, local_name) using longest-prefix matching.
async fn resolve_namespace(
    namespaced: &str,
    state: &AppState,
) -> Result<(String, String), McpError> {
    let mgr = state.read().await;
    let mut best_plugin_id: Option<String> = None;
    let mut best_local_name: Option<String> = None;

    for plugin in mgr.storage.list() {
        let prefix = format!("{}.", plugin.manifest.id);
        if let Some(local) = namespaced.strip_prefix(&prefix) {
            if !local.is_empty()
                && best_plugin_id
                    .as_ref()
                    .map_or(true, |prev| plugin.manifest.id.len() > prev.len())
            {
                best_plugin_id = Some(plugin.manifest.id.clone());
                best_local_name = Some(local.to_string());
            }
        }
    }

    match (best_plugin_id, best_local_name) {
        (Some(pid), Some(name)) => Ok((pid, name)),
        _ => Err(McpError::invalid_request(
            format!(
                "Cannot resolve '{}' to a plugin. No matching plugin ID prefix found.",
                namespaced
            ),
            None,
        )),
    }
}
