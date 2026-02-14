use super::McpWrapError;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;

/// A tool discovered from an MCP server via JSON-RPC.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredTool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

/// Discover tools from an MCP server by running the given command and
/// performing the JSON-RPC initialize → notifications/initialized → tools/list handshake.
pub async fn discover_tools(command: &str) -> Result<Vec<DiscoveredTool>, McpWrapError> {
    let parts: Vec<&str> = command.split_whitespace().collect();
    if parts.is_empty() {
        return Err(McpWrapError::Other("Empty command".to_string()));
    }

    let binary = parts[0];
    if binary != "npx" && binary != "node" {
        return Err(McpWrapError::UnsupportedRuntime(binary.to_string()));
    }

    let mut child = Command::new(binary)
        .args(&parts[1..])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::inherit())
        .spawn()?;

    let stdin = child.stdin.take().ok_or_else(|| {
        McpWrapError::Other("Failed to open stdin".to_string())
    })?;
    let stdout = child.stdout.take().ok_or_else(|| {
        McpWrapError::Other("Failed to open stdout".to_string())
    })?;

    let mut writer = stdin;
    let mut reader = BufReader::new(stdout);

    // Send initialize request
    let init_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": { "name": "nexus-mcp-wrap", "version": "0.1.0" }
        }
    });
    let mut msg = serde_json::to_string(&init_req)?;
    msg.push('\n');
    writer.write_all(msg.as_bytes()).await?;
    writer.flush().await?;

    let result = tokio::time::timeout(
        std::time::Duration::from_secs(15),
        run_handshake(&mut reader, &mut writer),
    )
    .await;

    // Kill the process regardless of outcome
    let _ = child.kill().await;

    match result {
        Ok(Ok(tools)) => Ok(tools),
        Ok(Err(e)) => Err(e),
        Err(_) => Err(McpWrapError::Timeout),
    }
}

/// Runs the JSON-RPC handshake after the initialize request has been sent.
/// Reads responses line-by-line, sends notifications/initialized + tools/list
/// after receiving the init response, then returns the discovered tools.
async fn run_handshake(
    reader: &mut BufReader<tokio::process::ChildStdout>,
    writer: &mut tokio::process::ChildStdin,
) -> Result<Vec<DiscoveredTool>, McpWrapError> {
    let mut line = String::new();
    let mut phase = Phase::Init;

    loop {
        line.clear();
        let n = reader.read_line(&mut line).await?;
        if n == 0 {
            return Err(McpWrapError::ServerExited(-1));
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let msg: serde_json::Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(_) => continue, // Ignore non-JSON output (server logs, etc.)
        };

        match phase {
            Phase::Init => {
                // Wait for initialize response (id=1)
                if msg.get("id") == Some(&serde_json::json!(1)) && msg.get("result").is_some() {
                    phase = Phase::Tools;

                    // Send notifications/initialized
                    let notif = serde_json::json!({
                        "jsonrpc": "2.0",
                        "method": "notifications/initialized"
                    });
                    let mut notif_msg = serde_json::to_string(&notif)?;
                    notif_msg.push('\n');
                    writer.write_all(notif_msg.as_bytes()).await?;

                    // Send tools/list
                    let tools_req = serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": 2,
                        "method": "tools/list",
                        "params": {}
                    });
                    let mut tools_msg = serde_json::to_string(&tools_req)?;
                    tools_msg.push('\n');
                    writer.write_all(tools_msg.as_bytes()).await?;
                    writer.flush().await?;
                }
            }
            Phase::Tools => {
                // Wait for tools/list response (id=2)
                if msg.get("id") == Some(&serde_json::json!(2)) {
                    if let Some(result) = msg.get("result") {
                        let raw_tools = result
                            .get("tools")
                            .and_then(|t| t.as_array())
                            .cloned()
                            .unwrap_or_default();

                        let tools = raw_tools
                            .into_iter()
                            .filter_map(|t| {
                                let name = t.get("name")?.as_str()?.to_string();
                                let description = t
                                    .get("description")
                                    .and_then(|d| d.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                let input_schema = t
                                    .get("inputSchema")
                                    .cloned()
                                    .unwrap_or_else(|| serde_json::json!({"type": "object", "properties": {}}));

                                Some(DiscoveredTool {
                                    name,
                                    description,
                                    input_schema,
                                })
                            })
                            .collect();

                        return Ok(tools);
                    }
                }
            }
        }
    }
}

enum Phase {
    Init,
    Tools,
}
