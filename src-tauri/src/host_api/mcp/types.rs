use serde::{Deserialize, Serialize};

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
