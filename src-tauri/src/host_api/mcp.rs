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

use crate::oauth::validation::{validate_bearer, TokenValidation};
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
                    "MCP session store at capacity ({}), session {} not cached",
                    MCP_SESSION_CAP,
                    session_id
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

    pub fn remove(&self, session_id: &str) {
        if let Ok(mut map) = self.authenticated.write() {
            map.remove(session_id);
        }
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
        .map(|s| s.to_string())
    {
        if mcp_sessions.is_authenticated(&session_id) {
            let resp = next.run(req).await;
            // rmcp lost the session (e.g. in-memory store cleared on restart) but our
            // session cache still had it. Rewrite 401 → 404 per MCP spec.
            if resp.status() == StatusCode::UNAUTHORIZED {
                log::info!("MCP session {} stale in rmcp — evicting cache, rewriting 401 → 404", session_id);
                mcp_sessions.remove(&session_id);
                return Ok(Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(Body::empty())
                    .unwrap());
            }
            return Ok(resp);
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

            // rmcp returns 401 for unknown sessions (e.g. after host restart) instead
            // of 404 per MCP spec. Since our auth already passed, a downstream 401 means
            // stale session — rewrite to 404 so the client re-initializes.
            if resp.status() == StatusCode::UNAUTHORIZED {
                log::info!("MCP session stale after gateway auth — rewriting 401 → 404");
                return Ok(Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(Body::empty())
                    .unwrap());
            }

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
    let oauth_store = req.extensions().get::<Arc<OAuthStore>>().cloned();
    let bearer_validation = oauth_store
        .as_ref()
        .map(|store| validate_bearer(req.headers(), store))
        .unwrap_or(TokenValidation::Missing);

    match bearer_validation {
        TokenValidation::Valid {
            plugin_id,
            authorization_details,
            client_name,
            ..
        } => {
            // Plugin tokens (client_credentials) require either blanket mcp:call
            // or at least one mcp:{target} permission to access the MCP endpoint.
            if let Some(ref pid) = plugin_id {
                // Fast path: check authorization_details on the token
                let has_blanket_mcp = rar::details_satisfy(&authorization_details, &Permission::McpCall);
                let has_any_mcp_access = authorization_details.iter().any(|d| {
                    d.detail_type == "nexus:mcp" && d.actions.iter().any(|a| a == "access")
                });
                if !has_blanket_mcp && !has_any_mcp_access {
                    // Fallback: check PermissionStore for McpCall or any McpAccess
                    let mgr = state.read().await;
                    let has_perm = mgr.permissions.has_permission(pid, &Permission::McpCall)
                        || mgr.permissions.get_grants(pid).iter().any(|g| {
                            matches!(&g.permission, Permission::McpAccess(_))
                                && g.state == crate::permissions::PermissionState::Active
                        });
                    if !has_perm {
                        log::warn!(
                            "AUDIT plugin={} tried MCP access without mcp:call or mcp:* permission",
                            pid
                        );
                        return Err(StatusCode::FORBIDDEN);
                    }
                    drop(mgr);
                }
            }

            log::info!("MCP authenticated via OAuth: client={}", client_name);
            let resp = next.run(req).await;

            // rmcp returns 401 for unknown sessions (e.g. after host restart) instead
            // of 404 per MCP spec. Since OAuth auth already passed, a downstream 401
            // means stale session — rewrite to 404 so the client re-initializes.
            if resp.status() == StatusCode::UNAUTHORIZED {
                log::info!("MCP session stale after OAuth auth — rewriting 401 → 404");
                return Ok(Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(Body::empty())
                    .unwrap());
            }

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
        TokenValidation::Invalid => {
            // Bearer token was provided but failed validation — tell the client to refresh
            // (RFC 6750 §3.1: error="invalid_token" signals expired/revoked tokens)
            log::info!("MCP Bearer token invalid/expired — returning invalid_token hint");
            let resp = Response::builder()
                .status(StatusCode::UNAUTHORIZED)
                .header(
                    "www-authenticate",
                    "Bearer error=\"invalid_token\", resource_metadata=\"http://127.0.0.1:9600/.well-known/oauth-protected-resource/mcp\"",
                )
                .body(Body::empty())
                .unwrap();
            return Ok(resp);
        }
        TokenValidation::Missing => {
            // Fall through to discovery challenge below
        }
    }

    // No auth provided at all — return discovery challenge
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
    use axum::{routing::get, Extension, Router, middleware as axum_mw};
    use tower::ServiceExt;

    use crate::permissions::{DefaultPermissionService, PermissionStore};
    use crate::runtime::mock::MockRuntime;
    use crate::plugin_manager::PluginManager;

    /// Build a minimal test router with the gateway auth middleware.
    fn gateway_test_app(oauth_store: Arc<OAuthStore>, data_dir: &std::path::Path) -> Router {
        let perm_store = PermissionStore::load(data_dir).unwrap_or_default();
        let permissions: Arc<dyn crate::permissions::service::PermissionService> =
            Arc::new(DefaultPermissionService::new(perm_store));
        let mock = Arc::new(MockRuntime::new());
        let mgr = PluginManager::new(data_dir.to_path_buf(), mock, permissions, oauth_store.clone());
        let state: AppState = Arc::new(tokio::sync::RwLock::new(mgr));

        let mcp_sessions = McpSessionStore::new();

        Router::new()
            .route("/mcp", get(|| async { "ok" }))
            .layer(axum_mw::from_fn_with_state(state.clone(), gateway_auth_middleware))
            .layer(Extension(oauth_store))
            .layer(Extension(mcp_sessions))
            .with_state(state)
    }

    // =====================================================================
    // Session store
    // =====================================================================

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

    // =====================================================================
    // Gateway auth — WWW-Authenticate differentiation (RFC 6750 §3.1)
    // =====================================================================

    #[tokio::test]
    async fn no_auth_returns_discovery_challenge() {
        let tmp = tempfile::tempdir().unwrap();
        let oauth_store = Arc::new(OAuthStore::load(tmp.path()));
        let app = gateway_test_app(oauth_store, tmp.path());

        let req = Request::builder()
            .uri("/mcp")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();

        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
        let www_auth = resp
            .headers()
            .get("www-authenticate")
            .unwrap()
            .to_str()
            .unwrap();
        assert!(
            www_auth.contains("resource_metadata="),
            "should include resource_metadata for discovery"
        );
        assert!(
            !www_auth.contains("error="),
            "no auth provided → no error hint (pure discovery challenge)"
        );
    }

    #[tokio::test]
    async fn expired_bearer_returns_invalid_token() {
        let tmp = tempfile::tempdir().unwrap();
        let oauth_store = Arc::new(OAuthStore::load(tmp.path()));
        let app = gateway_test_app(oauth_store, tmp.path());

        // Send a Bearer token that doesn't exist in the store (simulates expired/invalid)
        let req = Request::builder()
            .uri("/mcp")
            .header("authorization", "Bearer this-token-does-not-exist")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();

        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
        let www_auth = resp
            .headers()
            .get("www-authenticate")
            .unwrap()
            .to_str()
            .unwrap();
        assert!(
            www_auth.contains("error=\"invalid_token\""),
            "expired/invalid Bearer → must include error=\"invalid_token\" per RFC 6750"
        );
        assert!(
            www_auth.contains("resource_metadata="),
            "should still include resource_metadata for re-discovery"
        );
    }

    #[tokio::test]
    async fn valid_bearer_passes_through() {
        let tmp = tempfile::tempdir().unwrap();
        let oauth_store = Arc::new(OAuthStore::load(tmp.path()));

        // Create a valid access token (non-plugin, MCP scope)
        let token = oauth_store.create_access_token(
            "test-client".into(),
            "Test Client".into(),
            vec!["mcp".into()],
            "http://127.0.0.1:9600/mcp".into(),
            None,
            vec![],
        );

        let app = gateway_test_app(oauth_store, tmp.path());

        let req = Request::builder()
            .uri("/mcp")
            .header("authorization", format!("Bearer {}", token.token))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();

        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "valid Bearer should pass through to the handler"
        );
    }

    // =====================================================================
    // Stale session 401 → 404 rewrite (rmcp spec compliance)
    // =====================================================================

    /// Build a test router whose handler returns 401 (simulates rmcp rejecting
    /// unknown sessions after host restart).
    fn gateway_test_app_stale_session(oauth_store: Arc<OAuthStore>, data_dir: &std::path::Path) -> Router {
        let perm_store = PermissionStore::load(data_dir).unwrap_or_default();
        let permissions: Arc<dyn crate::permissions::service::PermissionService> =
            Arc::new(DefaultPermissionService::new(perm_store));
        let mock = Arc::new(MockRuntime::new());
        let mgr = PluginManager::new(data_dir.to_path_buf(), mock, permissions, oauth_store.clone());
        let state: AppState = Arc::new(tokio::sync::RwLock::new(mgr));

        let mcp_sessions = McpSessionStore::new();

        Router::new()
            .route("/mcp", get(|| async {
                // Simulate rmcp returning 401 for an unknown session
                Response::builder()
                    .status(StatusCode::UNAUTHORIZED)
                    .body(Body::from("Unauthorized: Session not found"))
                    .unwrap()
            }))
            .layer(axum_mw::from_fn_with_state(state.clone(), gateway_auth_middleware))
            .layer(Extension(oauth_store))
            .layer(Extension(mcp_sessions))
            .with_state(state)
    }

    #[tokio::test]
    async fn stale_session_bearer_auth_rewrites_401_to_404() {
        let tmp = tempfile::tempdir().unwrap();
        let oauth_store = Arc::new(OAuthStore::load(tmp.path()));

        let token = oauth_store.create_access_token(
            "test-client".into(),
            "Test Client".into(),
            vec!["mcp".into()],
            "http://127.0.0.1:9600/mcp".into(),
            None,
            vec![],
        );

        let app = gateway_test_app_stale_session(oauth_store, tmp.path());

        // Request with valid Bearer + stale Mcp-Session-Id
        let req = Request::builder()
            .uri("/mcp")
            .header("authorization", format!("Bearer {}", token.token))
            .header("mcp-session-id", "stale-session-from-before-restart")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();

        assert_eq!(
            resp.status(),
            StatusCode::NOT_FOUND,
            "auth succeeded but rmcp rejected session → must rewrite to 404 per MCP spec"
        );
    }

    #[tokio::test]
    async fn stale_session_cached_rewrites_401_to_404() {
        let tmp = tempfile::tempdir().unwrap();
        let oauth_store = Arc::new(OAuthStore::load(tmp.path()));

        let perm_store = PermissionStore::load(tmp.path()).unwrap_or_default();
        let permissions: Arc<dyn crate::permissions::service::PermissionService> =
            Arc::new(DefaultPermissionService::new(perm_store));
        let mock = Arc::new(MockRuntime::new());
        let mgr = PluginManager::new(tmp.path().to_path_buf(), mock, permissions, oauth_store.clone());
        let state: AppState = Arc::new(tokio::sync::RwLock::new(mgr));

        let mcp_sessions = McpSessionStore::new();
        // Pre-populate session cache (simulates session that was valid before restart)
        mcp_sessions.mark_authenticated("cached-but-stale");

        let app = Router::new()
            .route("/mcp", get(|| async {
                Response::builder()
                    .status(StatusCode::UNAUTHORIZED)
                    .body(Body::from("Unauthorized: Session not found"))
                    .unwrap()
            }))
            .layer(axum_mw::from_fn_with_state(state.clone(), gateway_auth_middleware))
            .layer(Extension(oauth_store))
            .layer(Extension(mcp_sessions.clone()))
            .with_state(state);

        let req = Request::builder()
            .uri("/mcp")
            .header("mcp-session-id", "cached-but-stale")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();

        assert_eq!(
            resp.status(),
            StatusCode::NOT_FOUND,
            "cached session rejected by rmcp → must rewrite to 404"
        );
        // Session should be evicted from cache
        assert!(
            !mcp_sessions.is_authenticated("cached-but-stale"),
            "stale session should be evicted from cache"
        );
    }

    #[test]
    fn session_store_remove() {
        let store = McpSessionStore::new();
        store.mark_authenticated("session-to-remove");
        assert!(store.is_authenticated("session-to-remove"));
        store.remove("session-to-remove");
        assert!(!store.is_authenticated("session-to-remove"));
    }
}
