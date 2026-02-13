mod gateway;
mod server;

use anyhow::Result;
use rmcp::{ServiceExt, transport::stdio};
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
    let srv = server::NexusServer::new(gw);

    let service = srv.serve(stdio()).await.inspect_err(|e| {
        tracing::error!("serving error: {e:?}");
    })?;

    tracing::info!("nexus-mcp ready on stdio");
    service.waiting().await?;

    Ok(())
}
