//! MCP client manager for native plugin MCP servers.
//!
//! Manages connections to plugin MCP servers running inside containers.
//! Each plugin that declares `mcp.server` in its manifest gets a cached entry
//! here with its tools, resources, and prompts.

use rmcp::model::*;
use rmcp::service::ServiceExt;
use rmcp::transport::StreamableHttpClientTransport;

/// Cached MCP capabilities for a single plugin's native MCP server.
#[derive(Debug, Clone)]
pub struct PluginMcpCache {
    /// Full URL to the plugin's MCP endpoint (e.g., "http://127.0.0.1:9701/mcp")
    pub url: String,
    pub tools: Vec<Tool>,
    pub resources: Vec<Resource>,
    pub resource_templates: Vec<ResourceTemplate>,
    pub prompts: Vec<Prompt>,
}

/// Manages MCP client connections to plugin servers.
///
/// Stateless connections: each operation opens a fresh HTTP request via rmcp's
/// streamable HTTP client. The cache stores discovered capabilities so we don't
/// need to reconnect for list operations.
pub struct McpClientManager {
    plugins: std::collections::HashMap<String, PluginMcpCache>,
}

impl McpClientManager {
    pub fn new() -> Self {
        Self {
            plugins: std::collections::HashMap::new(),
        }
    }

    /// Connect to a plugin's MCP server and cache its capabilities.
    ///
    /// Called when a plugin with `mcp.server` starts.
    pub async fn connect(
        &mut self,
        plugin_id: &str,
        host_port: u16,
        path: &str,
    ) -> Result<(), String> {
        let url = format!("http://127.0.0.1:{}{}", host_port, path);
        log::info!(
            "Connecting to native MCP server for plugin '{}' at {}",
            plugin_id,
            url
        );

        let transport = StreamableHttpClientTransport::from_uri(url.as_str());

        let service = ().serve(transport).await.map_err(|e| {
            format!(
                "Failed to connect to MCP server for '{}': {}",
                plugin_id, e
            )
        })?;

        // Discover capabilities
        let tools = match service.list_tools(Default::default()).await {
            Ok(result) => result.tools,
            Err(e) => {
                log::warn!(
                    "Plugin '{}' MCP server does not support tools: {}",
                    plugin_id,
                    e
                );
                vec![]
            }
        };

        let resources = match service.list_resources(Default::default()).await {
            Ok(result) => result.resources,
            Err(e) => {
                log::debug!(
                    "Plugin '{}' MCP server does not support resources: {}",
                    plugin_id,
                    e
                );
                vec![]
            }
        };

        let resource_templates = match service.list_resource_templates(Default::default()).await {
            Ok(result) => result.resource_templates,
            Err(e) => {
                log::debug!(
                    "Plugin '{}' MCP server does not support resource templates: {}",
                    plugin_id,
                    e
                );
                vec![]
            }
        };

        let prompts = match service.list_prompts(Default::default()).await {
            Ok(result) => result.prompts,
            Err(e) => {
                log::debug!(
                    "Plugin '{}' MCP server does not support prompts: {}",
                    plugin_id,
                    e
                );
                vec![]
            }
        };

        let _ = service.cancel().await;

        log::info!(
            "Plugin '{}' MCP capabilities: {} tools, {} resources, {} resource templates, {} prompts",
            plugin_id,
            tools.len(),
            resources.len(),
            resource_templates.len(),
            prompts.len(),
        );

        self.plugins.insert(
            plugin_id.to_string(),
            PluginMcpCache {
                url,
                tools,
                resources,
                resource_templates,
                prompts,
            },
        );

        Ok(())
    }

    /// Remove cached state on plugin stop/remove.
    pub fn disconnect(&mut self, plugin_id: &str) {
        if self.plugins.remove(plugin_id).is_some() {
            log::info!("Disconnected native MCP client for plugin '{}'", plugin_id);
        }
    }

    /// Get cached state for a plugin.
    pub fn get(&self, plugin_id: &str) -> Option<&PluginMcpCache> {
        self.plugins.get(plugin_id)
    }

    /// Iterate over all connected plugins.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &PluginMcpCache)> {
        self.plugins.iter().map(|(k, v)| (k.as_str(), v))
    }

    /// Check if a plugin has a native MCP connection.
    pub fn has(&self, plugin_id: &str) -> bool {
        self.plugins.contains_key(plugin_id)
    }

    /// Forward a tool call to a plugin's native MCP server.
    ///
    /// Opens a fresh connection for each call (stateless).
    pub async fn call_tool(
        &self,
        plugin_id: &str,
        name: &str,
        arguments: Option<serde_json::Map<String, serde_json::Value>>,
    ) -> Result<CallToolResult, String> {
        let cache = self
            .plugins
            .get(plugin_id)
            .ok_or_else(|| format!("No native MCP connection for plugin '{}'", plugin_id))?;

        let transport = StreamableHttpClientTransport::from_uri(cache.url.as_str());

        let service = ().serve(transport).await.map_err(|e| {
            format!(
                "Failed to connect to MCP server for '{}': {}",
                plugin_id, e
            )
        })?;

        let result = service
            .call_tool(CallToolRequestParams {
                name: std::borrow::Cow::Owned(name.to_string()),
                arguments,
                task: None,
                meta: None,
            })
            .await
            .map_err(|e| format!("Tool call failed: {}", e))?;

        let _ = service.cancel().await;
        Ok(result)
    }

    /// Forward a resource read to a plugin's native MCP server.
    pub async fn read_resource(
        &self,
        plugin_id: &str,
        uri: &str,
    ) -> Result<ReadResourceResult, String> {
        let cache = self
            .plugins
            .get(plugin_id)
            .ok_or_else(|| format!("No native MCP connection for plugin '{}'", plugin_id))?;

        let transport = StreamableHttpClientTransport::from_uri(cache.url.as_str());

        let service = ().serve(transport).await.map_err(|e| {
            format!(
                "Failed to connect to MCP server for '{}': {}",
                plugin_id, e
            )
        })?;

        let result = service
            .read_resource(ReadResourceRequestParams {
                uri: uri.to_string(),
                meta: None,
            })
            .await
            .map_err(|e| format!("Resource read failed: {}", e))?;

        let _ = service.cancel().await;
        Ok(result)
    }

    /// Forward a prompt get to a plugin's native MCP server.
    pub async fn get_prompt(
        &self,
        plugin_id: &str,
        name: &str,
        arguments: Option<serde_json::Map<String, serde_json::Value>>,
    ) -> Result<GetPromptResult, String> {
        let cache = self
            .plugins
            .get(plugin_id)
            .ok_or_else(|| format!("No native MCP connection for plugin '{}'", plugin_id))?;

        let transport = StreamableHttpClientTransport::from_uri(cache.url.as_str());

        let service = ().serve(transport).await.map_err(|e| {
            format!(
                "Failed to connect to MCP server for '{}': {}",
                plugin_id, e
            )
        })?;

        let result = service
            .get_prompt(GetPromptRequestParams {
                name: name.to_string(),
                arguments,
                meta: None,
            })
            .await
            .map_err(|e| format!("Prompt get failed: {}", e))?;

        let _ = service.cancel().await;
        Ok(result)
    }
}

impl Default for McpClientManager {
    fn default() -> Self {
        Self::new()
    }
}
