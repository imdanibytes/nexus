use rmcp::{
    ErrorData as McpError, RoleClient, RoleServer, ServerHandler,
    model::*,
    service::{RequestContext, ServiceExt},
    transport::StreamableHttpClientTransport,
};

/// MCP server that proxies all requests through the Nexus host's native
/// MCP endpoint via streamable HTTP.
///
/// This is the "Option A" sidecar simplification: the sidecar acts as
/// a stdio ↔ streamable HTTP bridge. It no longer implements any tool
/// logic itself — everything is forwarded to the host.
pub struct NexusServer {
    host_url: String,
    _token: String,
}

impl NexusServer {
    pub fn new(host_url: String, token: String) -> Self {
        Self {
            host_url,
            _token: token,
        }
    }

    /// Create a fresh MCP client connection to the host.
    async fn connect_to_host(&self) -> Result<rmcp::service::RunningService<RoleClient, ()>, McpError> {
        let transport = StreamableHttpClientTransport::from_uri(self.host_url.as_str());

        ().serve(transport).await.map_err(|e| {
            McpError::internal_error(format!("host connection failed: {e}"), None)
        })
    }
}

/// Convert a ServiceError to an McpError for ServerHandler return types.
fn service_err(e: rmcp::service::ServiceError) -> McpError {
    McpError::internal_error(format!("proxy error: {e}"), None)
}

impl ServerHandler for NexusServer {
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

    async fn list_tools(
        &self,
        request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        let service = self.connect_to_host().await?;
        let result = service.list_tools(request).await.map_err(service_err);
        let _ = service.cancel().await;
        result
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let service = self.connect_to_host().await?;
        let result = service.call_tool(request).await.map_err(service_err);
        let _ = service.cancel().await;
        result
    }

    async fn list_resources(
        &self,
        request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        let service = self.connect_to_host().await?;
        let result = service.list_resources(request).await.map_err(service_err);
        let _ = service.cancel().await;
        result
    }

    async fn list_resource_templates(
        &self,
        request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourceTemplatesResult, McpError> {
        let service = self.connect_to_host().await?;
        let result = service.list_resource_templates(request).await.map_err(service_err);
        let _ = service.cancel().await;
        result
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, McpError> {
        let service = self.connect_to_host().await?;
        let result = service.read_resource(request).await.map_err(service_err);
        let _ = service.cancel().await;
        result
    }

    async fn list_prompts(
        &self,
        request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListPromptsResult, McpError> {
        let service = self.connect_to_host().await?;
        let result = service.list_prompts(request).await.map_err(service_err);
        let _ = service.cancel().await;
        result
    }

    async fn get_prompt(
        &self,
        request: GetPromptRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<GetPromptResult, McpError> {
        let service = self.connect_to_host().await?;
        let result = service.get_prompt(request).await.map_err(service_err);
        let _ = service.cancel().await;
        result
    }
}
