use std::borrow::Cow;
use std::sync::Arc;

use rmcp::{
    ErrorData as McpError, RoleServer, ServerHandler,
    model::*,
    service::RequestContext,
};
use serde_json::Map;

use crate::gateway::NexusGateway;

/// MCP server that dynamically proxies tools from the Nexus host API.
///
/// Because tools are fetched at runtime from the host, we implement
/// `ServerHandler` manually rather than using the `#[tool_handler]` macro.
pub struct NexusServer {
    gateway: Arc<NexusGateway>,
}

impl NexusServer {
    pub fn new(gateway: NexusGateway) -> Self {
        Self {
            gateway: Arc::new(gateway),
        }
    }
}

impl ServerHandler for NexusServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .enable_tool_list_changed()
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
                "Nexus plugin hub — exposes tools from installed Nexus plugins.".to_string(),
            ),
        }
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        let tools = self.gateway.fetch_tools().await.map_err(|e| {
            tracing::error!("Failed to fetch tools from host: {e}");
            McpError::internal_error(format!("host unreachable: {e}"), None)
        })?;

        let mcp_tools: Vec<Tool> = tools
            .into_iter()
            .filter(|t| t.enabled && t.permissions_granted)
            .map(|t| {
                let schema = match t.input_schema {
                    serde_json::Value::Object(map) => map,
                    _ => Map::new(),
                };
                Tool {
                    name: Cow::Owned(t.name),
                    title: None,
                    description: Some(Cow::Owned(t.description)),
                    input_schema: Arc::new(schema),
                    output_schema: None,
                    annotations: None,
                    execution: None,
                    icons: None,
                    meta: None,
                }
            })
            .collect();

        Ok(ListToolsResult {
            tools: mcp_tools,
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
        let arguments = request
            .arguments
            .map(|obj| serde_json::Value::Object(obj))
            .unwrap_or(serde_json::Value::Object(Map::new()));

        tracing::info!(tool = tool_name, "Forwarding tool call to Nexus host");

        let resp = self
            .gateway
            .forward_call(tool_name, arguments)
            .await
            .map_err(|e| {
                tracing::error!("Host call failed: {e}");
                McpError::internal_error(format!("host call failed: {e}"), None)
            })?;

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
}
