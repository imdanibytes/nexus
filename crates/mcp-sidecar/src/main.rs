mod gateway;
mod server;

use anyhow::Result;
use futures_util::StreamExt;
use rmcp::{Peer, RoleServer, ServiceExt, transport::stdio};
use tracing_subscriber::{self, EnvFilter};

#[tokio::main]
async fn main() -> Result<()> {
    // Logging goes to stderr — Claude Desktop captures it for diagnostics.
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    tracing::info!("nexus-mcp starting");

    // Resolve the Tauri app data directory.
    // Identifier from tauri.conf.json: "com.nexus-dashboard.app"
    // macOS:   ~/Library/Application Support/com.nexus-dashboard.app/
    // Linux:   ~/.local/share/com.nexus-dashboard.app/
    // Windows: %APPDATA%/com.nexus-dashboard.app/
    // Token: prefer NEXUS_GATEWAY_TOKEN env var (set by Claude Desktop config),
    // fall back to reading from the Nexus data directory.
    let token = if let Ok(t) = std::env::var("NEXUS_GATEWAY_TOKEN") {
        tracing::info!("Using gateway token from NEXUS_GATEWAY_TOKEN env var");
        t.trim().to_string()
    } else {
        let data_dir = dirs::data_dir()
            .expect("cannot determine platform data directory")
            .join("com.nexus-dashboard.app");
        let token_path = data_dir.join("mcp_gateway_token");
        let t = std::fs::read_to_string(&token_path)
            .map_err(|e| {
                anyhow::anyhow!(
                    "Cannot read gateway token at {}: {}. Is Nexus running?",
                    token_path.display(),
                    e
                )
            })?;
        tracing::info!("Loaded gateway token from {}", token_path.display());
        t.trim().to_string()
    };

    // API URL: prefer env var, fall back to default.
    let base_url = std::env::var("NEXUS_API_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:9600".to_string());

    let gw = gateway::NexusGateway::new(base_url, token);
    let events_url = gw.events_url();
    let events_token = gw.token().to_string();
    let srv = server::NexusServer::new(gw);

    let service = srv.serve(stdio()).await.inspect_err(|e| {
        tracing::error!("serving error: {e:?}");
    })?;

    tracing::info!("nexus-mcp ready on stdio");

    // Clone the peer handle so the SSE listener can send notifications
    let peer = service.peer().clone();
    tokio::spawn(sse_listener(peer, events_url, events_token));

    service.waiting().await?;

    Ok(())
}

/// Connects to the Nexus host SSE endpoint and forwards `tools_changed` events
/// as MCP `notifications/tools/list_changed` to the connected client.
/// Reconnects with exponential backoff on disconnect.
async fn sse_listener(peer: Peer<RoleServer>, url: String, token: String) {
    let client = reqwest::Client::new();
    let mut backoff = std::time::Duration::from_secs(1);
    let max_backoff = std::time::Duration::from_secs(30);

    loop {
        tracing::info!("Connecting to SSE endpoint: {}", url);
        match connect_sse(&client, &peer, &url, &token).await {
            Ok(()) => {
                tracing::info!("SSE stream ended cleanly");
                backoff = std::time::Duration::from_secs(1);
            }
            Err(e) => {
                tracing::warn!("SSE connection error: {e}, reconnecting in {backoff:?}");
            }
        }
        tokio::time::sleep(backoff).await;
        backoff = (backoff * 2).min(max_backoff);
    }
}

async fn connect_sse(
    client: &reqwest::Client,
    peer: &Peer<RoleServer>,
    url: &str,
    token: &str,
) -> Result<()> {
    let resp = client
        .get(url)
        .header("X-Nexus-Gateway-Token", token)
        .header("Accept", "text/event-stream")
        .send()
        .await?
        .error_for_status()?;

    let mut stream = resp.bytes_stream();
    let mut buf = String::new();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        buf.push_str(&String::from_utf8_lossy(&chunk));

        // Process complete SSE frames (delimited by double newline)
        while let Some(pos) = buf.find("\n\n") {
            let frame = buf[..pos].to_string();
            buf = buf[pos + 2..].to_string();

            for line in frame.lines() {
                if line.starts_with("event: tools_changed")
                    || line.starts_with("event:tools_changed")
                {
                    tracing::info!("Received tools_changed event, notifying client");
                    if let Err(e) = peer.notify_tool_list_changed().await {
                        tracing::error!("Failed to notify client: {e}");
                    }
                    break;
                }
            }
        }
    }

    Ok(())
}
