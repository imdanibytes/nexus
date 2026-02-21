//! Host MCP server implementation.
//!
//! This module implements the `ServerHandler` trait from the `rmcp` crate,
//! serving as the primary gateway for external AI clients (like Claude Desktop)
//! to interact with Nexus plugins.
//!
//! Following the **Model Context Protocol (MCP) 2024-11-05 specification**, this
//! server aggregates tools, resources, and prompts from multiple sources into a
//! single namespaced view.

use std::sync::Arc;
use rmcp::model::*;
use rmcp::service::{NotificationContext, RequestContext};
use rmcp::{ErrorData as McpError, RoleServer, ServerHandler};

use crate::AppState;
use super::registry::McpRegistry;
use crate::host_api::approval::ApprovalBridge;

/// The Nexus host MCP server.
///
/// Implements the Server role in the MCP protocol. It manages the lifecycle
/// of client connections and dispatches requests to the `McpRegistry`.
pub struct NexusMcpServer {
    state: AppState,
    registry: Arc<McpRegistry>,
}

impl NexusMcpServer {
    pub fn new(state: AppState, approval_bridge: Arc<ApprovalBridge>) -> Self {
        let registry = Arc::new(McpRegistry::new(state.clone(), approval_bridge));
        Self { state, registry }
    }
}

impl ServerHandler for NexusMcpServer {
    /// Returns server metadata and capabilities.
    /// Ref: MCP Spec - "Lifecycle" section.
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder()
                .enable_tools().enable_tool_list_changed()
                .enable_resources().enable_resources_list_changed()
                .enable_prompts().enable_prompts_list_changed()
                .build(),
            server_info: Implementation {
                name: "nexus".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                title: None, description: None, icons: None, website_url: None,
            },
            instructions: Some("Nexus plugin hub — exposes tools, resources, and prompts from installed plugins.".to_string()),
        }
    }

    /// Called after the initial handshake is complete.
    /// Spawns a background task to monitor the tool list version and notify clients
    /// of changes using `notifications/tools/list_changed`.
    fn on_initialized(&self, context: NotificationContext<RoleServer>) -> impl std::future::Future<Output = ()> + Send + '_ {
        let peer = context.peer;
        let state = self.state.clone();
        async move {
            let mut rx = { let mgr = state.read().await; mgr.tool_version_rx.clone() };
            tokio::spawn(async move {
                loop {
                    // Wait for the tool version counter to bump in the PluginManager
                    if rx.changed().await.is_err() { break; }
                    // Ref: MCP Spec - "Notifications" section -> `notifications/tools/list_changed`
                    if let Err(_) = peer.notify_tool_list_changed().await { break; }
                }
            });
        }
    }

    /// List all available tools across all plugins and built-in handlers.
    /// Ref: MCP Spec - "Tools" section -> `tools/list`
    async fn list_tools(&self, _request: Option<PaginatedRequestParams>, _context: RequestContext<RoleServer>) -> Result<ListToolsResult, McpError> {
        let tools = self.registry.list_tools().await;
        Ok(ListToolsResult { tools, next_cursor: None, meta: None })
    }

    /// Dispatch a tool call to the correct provider.
    /// Ref: MCP Spec - "Tools" section -> `tools/call`
    async fn call_tool(&self, request: CallToolRequestParams, _context: RequestContext<RoleServer>) -> Result<CallToolResult, McpError> {
        self.registry.call_tool(&request.name, request.arguments).await
    }

    /// List available resources (files, logs, data streams).
    /// Ref: MCP Spec - "Resources" section -> `resources/list`
    async fn list_resources(&self, _request: Option<PaginatedRequestParams>, _context: RequestContext<RoleServer>) -> Result<ListResourcesResult, McpError> {
        let resources = self.registry.list_resources().await;
        Ok(ListResourcesResult { resources, next_cursor: None, meta: None })
    }

    /// List available resource templates.
    /// Ref: MCP Spec - "Resources" section -> `resources/templates/list`
    async fn list_resource_templates(&self, _request: Option<PaginatedRequestParams>, _context: RequestContext<RoleServer>) -> Result<ListResourceTemplatesResult, McpError> {
        let resource_templates = self.registry.list_resource_templates().await;
        Ok(ListResourceTemplatesResult { resource_templates, next_cursor: None, meta: None })
    }

    /// Read a specific resource by URI.
    /// Ref: MCP Spec - "Resources" section -> `resources/read`
    async fn read_resource(&self, request: ReadResourceRequestParams, _context: RequestContext<RoleServer>) -> Result<ReadResourceResult, McpError> {
        let mgr = self.state.read().await;
        for (plugin_id, cache) in mgr.mcp_clients.iter() {
            if cache.resources.iter().any(|r| r.uri == request.uri) {
                return mgr.mcp_clients.read_resource(plugin_id, &request.uri).await.map_err(|e| McpError::internal_error(e, None));
            }
        }
        Err(McpError::invalid_request(format!("Resource not found: {}", request.uri), None))
    }

    /// List available prompts (AI templates).
    /// Ref: MCP Spec - "Prompts" section -> `prompts/list`
    async fn list_prompts(&self, _request: Option<PaginatedRequestParams>, _context: RequestContext<RoleServer>) -> Result<ListPromptsResult, McpError> {
        let prompts = self.registry.list_prompts().await;
        Ok(ListPromptsResult { prompts, next_cursor: None, meta: None })
    }

    /// Get a specific prompt by name.
    /// Ref: MCP Spec - "Prompts" section -> `prompts/get`
    async fn get_prompt(&self, request: GetPromptRequestParams, _context: RequestContext<RoleServer>) -> Result<GetPromptResult, McpError> {
        let (plugin_id, local_name) = self.registry.resolve_namespace(&request.name).await?;
        let mgr = self.state.read().await;
        mgr.mcp_clients.get_prompt(&plugin_id, &local_name, request.arguments).await.map_err(|e| McpError::internal_error(e, None))
    }
}
