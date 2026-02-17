use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Instant;

use crate::oauth::OAuthStore;
use crate::permissions::rar;
use crate::permissions::Permission;
use crate::AppState;


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

const MCP_SESSION_TTL_SECS: u64 = 24 * 60 * 60; // 24 hours
const MCP_SESSION_CAP: usize = 1000;

/// Tracks MCP session IDs that have been authenticated via gateway token.
/// Once a session authenticates on its first request, subsequent requests
/// with the same Mcp-Session-Id are allowed through without re-checking.
/// Sessions expire after 24 hours and the store is capped at 1000 entries.
#[derive(Debug, Clone)]
pub struct McpSessionStore {
    authenticated: Arc<RwLock<HashMap<String, Instant>>>,
}

impl Default for McpSessionStore {
    fn default() -> Self {
        Self {
            authenticated: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl McpSessionStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn mark_authenticated(&self, session_id: &str) {
        if let Ok(mut map) = self.authenticated.write() {
            let now = Instant::now();
            let ttl = std::time::Duration::from_secs(MCP_SESSION_TTL_SECS);

            // Evict expired entries
            map.retain(|_, ts| now.duration_since(*ts) < ttl);

            // Cap check — if still at cap after eviction, log and skip
            if map.len() >= MCP_SESSION_CAP {
                log::warn!(
                    "MCP session store at capacity ({}), new session not cached",
                    MCP_SESSION_CAP
                );
                return;
            }

            map.insert(session_id.to_string(), now);
        }
    }

    pub fn is_authenticated(&self, session_id: &str) -> bool {
        self.authenticated
            .read()
            .map(|map| {
                map.get(session_id).is_some_and(|ts| {
                    ts.elapsed() < std::time::Duration::from_secs(MCP_SESSION_TTL_SECS)
                })
            })
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
    let mcp_sessions = match req.extensions().get::<McpSessionStore>().cloned() {
        Some(store) => store,
        None => {
            log::warn!("McpSessionStore missing from extensions — session caching disabled");
            McpSessionStore::new()
        }
    };

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

    // Try Bearer token (OAuth 2.1 — both external AI clients and plugin tokens)
    if let Some(bearer) = req
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
    {
        if let Some(oauth_store) = req.extensions().get::<Arc<OAuthStore>>().cloned() {
            if let Some(access_token) = oauth_store.validate_access_token(bearer) {
                // Plugin tokens (client_credentials) require mcp:call permission
                if let Some(ref plugin_id) = access_token.plugin_id {
                    // Fast path: check authorization_details on the token
                    let has_mcp = rar::details_satisfy(&access_token.authorization_details, &Permission::McpCall);
                    if !has_mcp {
                        // Fallback: check PermissionStore
                        let mgr = state.read().await;
                        if !mgr.permissions.has_permission(plugin_id, &Permission::McpCall) {
                            log::warn!(
                                "AUDIT plugin={} tried MCP access without mcp:call permission",
                                plugin_id
                            );
                            return Err(StatusCode::FORBIDDEN);
                        }
                        drop(mgr);
                    }
                }

                log::info!("MCP authenticated via OAuth: client={}", access_token.client_name);
                let resp = next.run(req).await;
                if let Some(session_id) = resp
                    .headers()
                    .get("mcp-session-id")
                    .and_then(|v| v.to_str().ok())
                {
                    mcp_sessions.mark_authenticated(session_id);
                    log::info!("MCP session authenticated (OAuth): {}", session_id);
                }
                return Ok(resp);
            }
        }
    }

    // 401 with WWW-Authenticate header — tells MCP clients where to find OAuth metadata
    let resp = Response::builder()
        .status(StatusCode::UNAUTHORIZED)
        .header(
            "www-authenticate",
            "Bearer resource_metadata=\"http://127.0.0.1:9600/.well-known/oauth-protected-resource/mcp\"",
        )
        .body(Body::empty())
        .unwrap();
    Ok(resp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_store_mark_and_check() {
        let store = McpSessionStore::new();
        assert!(!store.is_authenticated("session-1"));
        store.mark_authenticated("session-1");
        assert!(store.is_authenticated("session-1"));
    }

    #[test]
    fn session_store_independent_sessions() {
        let store = McpSessionStore::new();
        store.mark_authenticated("session-1");
        assert!(!store.is_authenticated("session-2"));
    }

    #[test]
    fn session_store_cap_enforcement() {
        let store = McpSessionStore::new();
        // Fill to capacity
        for i in 0..MCP_SESSION_CAP {
            store.mark_authenticated(&format!("session-{}", i));
        }
        // All should be authenticated
        assert!(store.is_authenticated("session-0"));
        assert!(store.is_authenticated(&format!("session-{}", MCP_SESSION_CAP - 1)));

        // One more should be silently dropped (cap reached)
        store.mark_authenticated("overflow-session");
        assert!(!store.is_authenticated("overflow-session"));
    }
}
