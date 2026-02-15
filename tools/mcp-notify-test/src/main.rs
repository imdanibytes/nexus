use rmcp::{
    ErrorData as McpError, Peer, RoleServer, ServerHandler,
    model::*,
    service::RequestContext,
    transport::stdio,
    ServiceExt,
};
use serde_json::Map;
use std::borrow::Cow;
use std::sync::Arc;

struct NotifyTestServer;

impl ServerHandler for NotifyTestServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .enable_logging()
                .build(),
            server_info: Implementation {
                name: "mcp-notify-test".into(),
                version: "0.1.0".into(),
                title: None,
                description: None,
                icons: None,
                website_url: None,
            },
            instructions: Some(
                "Test server for MCP notifications. Call 'send_notification' to trigger a notification back to you.".to_string(),
            ),
        }
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        let tools = vec![
            Tool {
                name: Cow::Borrowed("send_notification"),
                title: None,
                description: Some(Cow::Borrowed(
                    "Sends a test notification back to the client via MCP logging",
                )),
                input_schema: Arc::new({
                    let mut map = Map::new();
                    map.insert("type".into(), serde_json::json!("object"));
                    map.insert(
                        "properties".into(),
                        serde_json::json!({
                            "message": {
                                "type": "string",
                                "description": "The notification message to send"
                            },
                            "level": {
                                "type": "string",
                                "enum": ["debug", "info", "warning", "error"],
                                "description": "Log level (default: info)"
                            }
                        }),
                    );
                    map.insert(
                        "required".into(),
                        serde_json::json!(["message"]),
                    );
                    map
                }),
                output_schema: None,
                annotations: None,
                execution: None,
                icons: None,
                meta: None,
            },
            Tool {
                name: Cow::Borrowed("send_delayed_notification"),
                title: None,
                description: Some(Cow::Borrowed(
                    "Returns immediately, then sends a notification after a delay",
                )),
                input_schema: Arc::new({
                    let mut map = Map::new();
                    map.insert("type".into(), serde_json::json!("object"));
                    map.insert(
                        "properties".into(),
                        serde_json::json!({
                            "message": {
                                "type": "string",
                                "description": "The notification message to send after delay"
                            },
                            "delay_secs": {
                                "type": "integer",
                                "description": "Seconds to wait before sending (default: 3)"
                            }
                        }),
                    );
                    map.insert(
                        "required".into(),
                        serde_json::json!(["message"]),
                    );
                    map
                }),
                output_schema: None,
                annotations: None,
                execution: None,
                icons: None,
                meta: None,
            },
        ];

        Ok(ListToolsResult {
            tools,
            next_cursor: None,
            meta: None,
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let args = request.arguments.unwrap_or_default();
        let message = args
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("Hello from MCP notification!")
            .to_string();

        let level = args
            .get("level")
            .and_then(|v| v.as_str())
            .unwrap_or("info");

        let logging_level = match level {
            "debug" => LoggingLevel::Debug,
            "warning" => LoggingLevel::Warning,
            "error" => LoggingLevel::Error,
            _ => LoggingLevel::Info,
        };

        match request.name.as_ref() {
            "send_notification" => {
                // Send the notification immediately
                let peer: Peer<RoleServer> = context.peer.clone();
                let result = peer
                    .notify_logging_message(LoggingMessageNotificationParam {
                        level: logging_level,
                        logger: Some("nexus".to_string()),
                        data: serde_json::json!(message),
                    })
                    .await;

                match result {
                    Ok(()) => Ok(CallToolResult::success(vec![Content::text(
                        format!("Notification sent: \"{}\" (level: {})", message, level),
                    )])),
                    Err(e) => Ok(CallToolResult::success(vec![Content::text(
                        format!("Failed to send notification: {}", e),
                    )])),
                }
            }
            "send_delayed_notification" => {
                let delay = args
                    .get("delay_secs")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(3);

                let peer: Peer<RoleServer> = context.peer.clone();
                let msg_clone = message.clone();

                // Spawn background task to send notification after delay
                tokio::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
                    let _ = peer
                        .notify_logging_message(LoggingMessageNotificationParam {
                            level: logging_level,
                            logger: Some("nexus".to_string()),
                            data: serde_json::json!(msg_clone),
                        })
                        .await;
                });

                Ok(CallToolResult::success(vec![Content::text(
                    format!(
                        "OK — notification \"{}\" will be sent in {} seconds",
                        message, delay
                    ),
                )]))
            }
            _ => Err(McpError::method_not_found::<CallToolRequestMethod>()),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    let server = NotifyTestServer;
    let service = server.serve(stdio()).await?;
    service.waiting().await?;

    Ok(())
}
