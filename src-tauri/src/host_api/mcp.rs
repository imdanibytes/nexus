use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::{Arc, RwLock};

use crate::permissions::Permission;
use crate::AppState;

use super::auth::SessionStore;

// ---------------------------------------------------------------------------
// Types (used by nexus_mcp.rs and mcp_server.rs)
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Authenticated MCP session store
// ---------------------------------------------------------------------------

/// Tracks MCP session IDs that have been authenticated via gateway token.
/// Once a session authenticates on its first request, subsequent requests
/// with the same Mcp-Session-Id are allowed through without re-checking.
#[derive(Debug, Clone)]
pub struct McpSessionStore {
    authenticated: Arc<RwLock<HashSet<String>>>,
}

impl Default for McpSessionStore {
    fn default() -> Self {
        Self {
            authenticated: Arc::new(RwLock::new(HashSet::new())),
        }
    }
}

impl McpSessionStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn mark_authenticated(&self, session_id: &str) {
        if let Ok(mut set) = self.authenticated.write() {
            set.insert(session_id.to_string());
        }
    }

    pub fn is_authenticated(&self, session_id: &str) -> bool {
        self.authenticated
            .read()
            .map(|set| set.contains(session_id))
            .unwrap_or(false)
    }
}

// ---------------------------------------------------------------------------
// HTTP request logging middleware (applies to ALL routes on the Host API)
// ---------------------------------------------------------------------------

pub async fn http_request_logging(
    req: Request<Body>,
    next: Next,
) -> Response {
    let method = req.method().clone();
    let uri = req.uri().clone();
    let session_id = req
        .headers()
        .get("mcp-session-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("-")
        .to_string();
    let content_type = req
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("-")
        .to_string();
    let accept = req
        .headers()
        .get("accept")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("-")
        .to_string();

    log::info!(
        "HTTP ← {} {} | session={} type={} accept={}",
        method, uri, session_id, content_type, accept,
    );

    let resp = next.run(req).await;

    log::info!(
        "HTTP → {} {} | status={}",
        method, uri, resp.status(),
    );

    resp
}

// ---------------------------------------------------------------------------
// Gateway auth middleware (shared by native /mcp endpoint)
// ---------------------------------------------------------------------------

pub async fn gateway_auth_middleware(
    State(state): State<AppState>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let mcp_sessions = req
        .extensions()
        .get::<McpSessionStore>()
        .cloned()
        .unwrap_or_default();

    // If this request carries an Mcp-Session-Id that was already authenticated, let it through.
    if let Some(session_id) = req
        .headers()
        .get("mcp-session-id")
        .and_then(|v| v.to_str().ok())
    {
        if mcp_sessions.is_authenticated(session_id) {
            return Ok(next.run(req).await);
        }
    }

    // Try X-Nexus-Gateway-Token first (MCP gateway auth)
    if let Some(token) = req
        .headers()
        .get("X-Nexus-Gateway-Token")
        .and_then(|v| v.to_str().ok())
    {
        let mgr = state.read().await;
        if mgr.verify_gateway_token(token) {
            drop(mgr);
            let resp = next.run(req).await;

            // The response may contain a new Mcp-Session-Id — remember it as authenticated
            if let Some(session_id) = resp
                .headers()
                .get("mcp-session-id")
                .and_then(|v| v.to_str().ok())
            {
                mcp_sessions.mark_authenticated(session_id);
                log::info!("MCP session authenticated: {}", session_id);
            }

            return Ok(resp);
        }
    }

    // Fall back to Bearer token (plugin path — requires mcp:call permission)
    if let Some(bearer) = req
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
    {
        let sessions = req
            .extensions()
            .get::<Arc<SessionStore>>()
            .cloned()
            .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

        if let Some(plugin_id) = sessions.validate(bearer) {
            let mgr = state.read().await;
            if mgr
                .permissions
                .has_permission(&plugin_id, &Permission::McpCall)
            {
                drop(mgr);
                let resp = next.run(req).await;

                if let Some(session_id) = resp
                    .headers()
                    .get("mcp-session-id")
                    .and_then(|v| v.to_str().ok())
                {
                    mcp_sessions.mark_authenticated(session_id);
                    log::info!("MCP session authenticated (plugin): {}", session_id);
                }

                return Ok(resp);
            }
            log::warn!(
                "AUDIT plugin={} tried MCP access without mcp:call permission",
                plugin_id
            );
            return Err(StatusCode::FORBIDDEN);
        }
    }

    Err(StatusCode::UNAUTHORIZED)
}
