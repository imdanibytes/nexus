//! MCP client manager for native plugin MCP servers.

use rmcp::model::*;
use rmcp::service::ServiceExt;
use rmcp::transport::StreamableHttpClientTransport;

/// Cached MCP capabilities for a single plugin's native MCP server.
#[derive(Debug, Clone)]
pub struct PluginMcpCache {
    pub url: String,
    pub tools: Vec<Tool>,
    pub resources: Vec<Resource>,
    pub resource_templates: Vec<ResourceTemplate>,
    pub prompts: Vec<Prompt>,
}

/// Manages MCP client connections to plugin servers.
pub struct McpClientManager {
    plugins: std::collections::HashMap<String, PluginMcpCache>,
}

impl McpClientManager {
    pub fn new() -> Self {
        Self {
            plugins: std::collections::HashMap::new(),
        }
    }

    pub async fn connect(
        &mut self,
        plugin_id: &str,
        host_port: u16,
        path: &str,
    ) -> Result<(), String> {
        let url = format!("http://127.0.0.1:{}{}", host_port, path);
        log::info!("Connecting to native MCP server for plugin '{}' at {}", plugin_id, url);

        let transport = StreamableHttpClientTransport::from_uri(url.as_str());
        let service = ().serve(transport).await.map_err(|e| format!("Failed to connect: {}", e))?;

        let tools = service.list_tools(Default::default()).await.map(|r| r.tools).unwrap_or_default();
        let resources = service.list_resources(Default::default()).await.map(|r| r.resources).unwrap_or_default();
        let resource_templates = service.list_resource_templates(Default::default()).await.map(|r| r.resource_templates).unwrap_or_default();
        let prompts = service.list_prompts(Default::default()).await.map(|r| r.prompts).unwrap_or_default();

        let _ = service.cancel().await;

        self.plugins.insert(plugin_id.to_string(), PluginMcpCache { url, tools, resources, resource_templates, prompts });
        Ok(())
    }

    pub fn disconnect(&mut self, plugin_id: &str) {
        self.plugins.remove(plugin_id);
    }

    pub fn get(&self, plugin_id: &str) -> Option<&PluginMcpCache> { self.plugins.get(plugin_id) }
    pub fn iter(&self) -> impl Iterator<Item = (&str, &PluginMcpCache)> { self.plugins.iter().map(|(k, v)| (k.as_str(), v)) }
    pub fn has(&self, plugin_id: &str) -> bool { self.plugins.contains_key(plugin_id) }

    pub async fn call_tool(&self, plugin_id: &str, name: &str, arguments: Option<serde_json::Map<String, serde_json::Value>>) -> Result<CallToolResult, String> {
        let cache = self.plugins.get(plugin_id).ok_or_else(|| format!("No connection for '{}'", plugin_id))?;
        let transport = StreamableHttpClientTransport::from_uri(cache.url.as_str());
        let service = ().serve(transport).await.map_err(|e| format!("Failed to connect: {}", e))?;
        let result = service.call_tool(CallToolRequestParams { name: std::borrow::Cow::Owned(name.to_string()), arguments, task: None, meta: None }).await.map_err(|e| format!("Call failed: {}", e))?;
        let _ = service.cancel().await;
        Ok(result)
    }

    pub async fn read_resource(&self, plugin_id: &str, uri: &str) -> Result<ReadResourceResult, String> {
        let cache = self.plugins.get(plugin_id).ok_or_else(|| format!("No connection for '{}'", plugin_id))?;
        let transport = StreamableHttpClientTransport::from_uri(cache.url.as_str());
        let service = ().serve(transport).await.map_err(|e| format!("Failed to connect: {}", e))?;
        let result = service.read_resource(ReadResourceRequestParams { uri: uri.to_string(), meta: None }).await.map_err(|e| format!("Read failed: {}", e))?;
        let _ = service.cancel().await;
        Ok(result)
    }

    pub async fn get_prompt(&self, plugin_id: &str, name: &str, arguments: Option<serde_json::Map<String, serde_json::Value>>) -> Result<GetPromptResult, String> {
        let cache = self.plugins.get(plugin_id).ok_or_else(|| format!("No connection for '{}'", plugin_id))?;
        let transport = StreamableHttpClientTransport::from_uri(cache.url.as_str());
        let service = ().serve(transport).await.map_err(|e| format!("Failed to connect: {}", e))?;
        let result = service.get_prompt(GetPromptRequestParams { name: name.to_string(), arguments, meta: None }).await.map_err(|e| format!("Prompt failed: {}", e))?;
        let _ = service.cancel().await;
        Ok(result)
    }
}

impl Default for McpClientManager { fn default() -> Self { Self::new() } }
