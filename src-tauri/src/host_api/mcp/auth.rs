use axum::{
    body::Body,
    extract::{ConnectInfo, State},
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::{Arc, RwLock};
use std::time::Instant;

use crate::api_keys::ApiKeyStore;
use crate::oauth::validation::{validate_bearer, TokenValidation};
use crate::oauth::OAuthStore;
use crate::permissions::rar;
use crate::permissions::Permission;
use crate::AppState;

/// Check whether a socket address is a loopback (localhost) connection.
fn is_loopback(addr: &SocketAddr) -> bool {
    match addr.ip() {
        IpAddr::V4(ip) => ip == Ipv4Addr::LOCALHOST,
        IpAddr::V6(ip) => {
            ip == Ipv6Addr::LOCALHOST
                || ip == Ipv4Addr::LOCALHOST.to_ipv6_mapped()
        }
    }
}

// ---------------------------------------------------------------------------
// Authenticated MCP session store
// ---------------------------------------------------------------------------

const MCP_SESSION_TTL_SECS: u64 = 24 * 60 * 60; // 24 hours
const MCP_SESSION_CAP: usize = 1000;

/// Caches authenticated MCP session IDs to avoid re-validating credentials on
/// every request within a session.
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

            map.retain(|_, ts| now.duration_since(*ts) < ttl);

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
// HTTP request logging middleware
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

    if log::log_enabled!(log::Level::Debug) {
        for (name, value) in req.headers() {
            if let Ok(v) = value.to_str() {
                log::debug!("  header: {}: {}", name, v);
            }
        }
    }

    let resp = next.run(req).await;

    log::info!(
        "HTTP → {} {} | status={}",
        method, uri, resp.status(),
    );

    resp
}

// ---------------------------------------------------------------------------
// Gateway auth middleware
// ---------------------------------------------------------------------------

/// Authenticates incoming MCP requests using a layered strategy:
///
/// 1. **Session cache** — If the request carries an `Mcp-Session-Id` that was
///    previously authenticated, skip credential checks (24h TTL).
///
/// 2. **API key** — `Authorization: Bearer nxk_...` tokens. Restricted to
///    loopback connections as a defense-in-depth measure.
///
/// 3. **OAuth 2.0 Bearer** — Validated against [`OAuthStore`] per **RFC 6750 §2.1**.
///    Used by external AI clients and plugin `client_credentials` tokens (**RFC 6749 §4.4**).
///
/// 4. **Discovery challenge** — No credentials → 401 with `WWW-Authenticate: Bearer`
///    challenge per **RFC 7235 §3.1**, including `resource_metadata` (**RFC 9728 §2**)
///    to point the client at the authorization server.
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
    // The MCP Streamable HTTP transport uses session IDs to correlate requests.
    if let Some(session_id) = req
        .headers()
        .get("mcp-session-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
    {
        if mcp_sessions.is_authenticated(&session_id) {
            let resp = next.run(req).await;
            // Rewrite 401 → 404 per MCP spec for stale sessions (e.g. after host restart)
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

    // Extract Bearer token from the Authorization header.
    // RFC 7235 §2.1: auth-scheme comparison is case-insensitive.
    // RFC 6750 §2.1: `Authorization: Bearer <token>` is the standard format.
    let bearer_value = req
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| {
            if v.len() > 7 && v[..7].eq_ignore_ascii_case("bearer ") {
                Some(&v[7..])
            } else {
                None
            }
        })
        .map(|s| s.to_string());

    if let Some(ref token) = bearer_value {
        // API key auth: nxk_ prefix distinguishes API keys from OAuth tokens.
        if token.starts_with("nxk_") {
            let api_key_store = req.extensions().get::<ApiKeyStore>().cloned();
            let peer_addr = req.extensions().get::<ConnectInfo<SocketAddr>>();

            // API keys are limited to loopback (127.0.0.1) to prevent network exposure.
            if let Some(connect_info) = peer_addr {
                if !is_loopback(&connect_info.0) {
                    log::warn!(
                        "API key auth rejected: non-localhost peer {}",
                        connect_info.0
                    );
                    return Err(StatusCode::FORBIDDEN);
                }
            }

            if let Some(store) = api_key_store {
                if let Some(key) = store.validate(token) {
                    log::info!("MCP authenticated via API key: name={} prefix={}", key.name, key.prefix);
                    let resp = next.run(req).await;

                    if resp.status() == StatusCode::UNAUTHORIZED {
                        log::info!("MCP session stale after API key auth — rewriting 401 → 404");
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
                        log::info!("MCP session authenticated (API key): {}", session_id);
                    }

                    return Ok(resp);
                }
            }

            // nxk_ prefix but invalid key — respond per RFC 6750 §3.1.
            log::info!("MCP API key invalid — returning 401");
            let resp = Response::builder()
                .status(StatusCode::UNAUTHORIZED)
                .header(
                    "www-authenticate",
                    "Bearer realm=\"nexus-mcp\", error=\"invalid_token\", resource_metadata=\"http://127.0.0.1:9600/.well-known/oauth-protected-resource/mcp\"",
                )
                .body(Body::empty())
                .unwrap();
            return Ok(resp);
        }
    }

    // OAuth 2.0 Bearer token validation (RFC 6749 §7.1, RFC 6750 §2.1).
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
            // Internal plugin auth check: plugins require mcp:call permission.
            // Uses RFC 9396 (Authorization Details) if present on the token.
            if let Some(ref pid) = plugin_id {
                let has_blanket_mcp = rar::details_satisfy(&authorization_details, &Permission::McpCall);
                let has_any_mcp_access = authorization_details.iter().any(|d| {
                    d.detail_type == "nexus:mcp" && d.actions.iter().any(|a| a == "access")
                });
                if !has_blanket_mcp && !has_any_mcp_access {
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
            // RFC 6750 §3.1: error="invalid_token" hint for expired/revoked tokens.
            log::info!("MCP Bearer token invalid/expired — returning invalid_token hint");
            let resp = Response::builder()
                .status(StatusCode::UNAUTHORIZED)
                .header(
                    "www-authenticate",
                    "Bearer realm=\"nexus-mcp\", error=\"invalid_token\", resource_metadata=\"http://127.0.0.1:9600/.well-known/oauth-protected-resource/mcp\"",
                )
                .body(Body::empty())
                .unwrap();
            return Ok(resp);
        }
        TokenValidation::Missing => {}
    }

    // No credentials — RFC 7235 §3.1: server MUST respond with 401 challenge.
    // RFC 6750 §3: Challenge points to metadata for OAuth discovery.
    let resp = Response::builder()
        .status(StatusCode::UNAUTHORIZED)
        .header(
            "www-authenticate",
            "Bearer realm=\"nexus-mcp\", resource_metadata=\"http://127.0.0.1:9600/.well-known/oauth-protected-resource/mcp\"",
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

    use crate::api_keys::ApiKeyStore;
    use crate::permissions::{DefaultPermissionService, PermissionStore};
    use crate::runtime::mock::MockRuntime;
    use crate::plugin_manager::PluginManager;

    fn gateway_test_app(oauth_store: Arc<OAuthStore>, data_dir: &std::path::Path) -> Router {
        let perm_store = PermissionStore::load(data_dir).unwrap_or_default();
        let permissions: Arc<dyn crate::permissions::service::PermissionService> =
            Arc::new(DefaultPermissionService::new(perm_store));
        let mock = Arc::new(MockRuntime::new());
        let mgr = PluginManager::new(data_dir.to_path_buf(), mock, permissions, oauth_store.clone());
        let state: AppState = Arc::new(tokio::sync::RwLock::new(mgr));

        let mcp_sessions = McpSessionStore::new();
        let api_key_store = ApiKeyStore::load(data_dir);

        Router::new()
            .route("/mcp", get(|| async { "ok" }))
            .layer(axum_mw::from_fn_with_state(state.clone(), gateway_auth_middleware))
            .layer(Extension(oauth_store))
            .layer(Extension(mcp_sessions))
            .layer(Extension(api_key_store))
            .with_state(state)
    }

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
        for i in 0..MCP_SESSION_CAP {
            store.mark_authenticated(&format!("session-{}", i));
        }
        assert!(store.is_authenticated("session-0"));
        assert!(store.is_authenticated(&format!("session-{}", MCP_SESSION_CAP - 1)));
        store.mark_authenticated("overflow-session");
        assert!(!store.is_authenticated("overflow-session"));
    }

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
        assert!(www_auth.contains("realm=\"nexus-mcp\""));
        assert!(www_auth.contains("resource_metadata="));
    }
}
