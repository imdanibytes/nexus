use reqwest::Client;
use serde::{Deserialize, Serialize};

/// HTTP client that proxies MCP tool operations to the Nexus host API.
pub struct NexusGateway {
    client: Client,
    base_url: String,
    token: String,
}

/// A tool entry returned by the host API's `GET /api/v1/mcp/tools`.
#[derive(Debug, Deserialize)]
pub struct HostToolEntry {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    #[allow(dead_code)]
    pub plugin_id: String,
    #[allow(dead_code)]
    pub plugin_name: String,
    #[allow(dead_code)]
    pub required_permissions: Vec<String>,
    #[allow(dead_code)]
    pub permissions_granted: bool,
    #[allow(dead_code)]
    pub enabled: bool,
}

#[derive(Serialize)]
struct HostCallRequest {
    tool_name: String,
    arguments: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct HostCallResponse {
    pub content: Vec<HostContentItem>,
    pub is_error: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct HostContentItem {
    /// Kept for serde round-tripping; only "text" is used currently.
    #[serde(rename = "type")]
    #[allow(dead_code)]
    pub content_type: String,
    pub text: String,
}

impl NexusGateway {
    pub fn new(base_url: String, token: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
            token,
        }
    }

    /// Fetch the merged tool list from Nexus host.
    pub async fn fetch_tools(&self) -> anyhow::Result<Vec<HostToolEntry>> {
        let resp = self
            .client
            .get(format!("{}/api/v1/mcp/tools", self.base_url))
            .header("X-Nexus-Gateway-Token", &self.token)
            .send()
            .await?
            .error_for_status()?
            .json::<Vec<HostToolEntry>>()
            .await?;
        Ok(resp)
    }

    /// Forward a tool call to the Nexus host and return the result.
    pub async fn forward_call(
        &self,
        tool_name: &str,
        arguments: serde_json::Value,
    ) -> anyhow::Result<HostCallResponse> {
        let resp = self
            .client
            .post(format!("{}/api/v1/mcp/call", self.base_url))
            .header("X-Nexus-Gateway-Token", &self.token)
            .json(&HostCallRequest {
                tool_name: tool_name.to_string(),
                arguments,
            })
            .send()
            .await?
            .error_for_status()?
            .json::<HostCallResponse>()
            .await?;
        Ok(resp)
    }
}
