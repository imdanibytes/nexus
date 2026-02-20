use std::sync::Arc;

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};

use crate::oauth::validation::{validate_bearer, TokenValidation};
use crate::oauth::OAuthStore;
use crate::permissions::checker::required_permission_for_endpoint;
use crate::permissions::rar;
use crate::permissions::PermissionState;
use crate::AppState;

use super::approval::{ApprovalBridge, ApprovalRequest};

#[derive(Clone, Debug)]
pub struct AuthenticatedPlugin {
    pub plugin_id: String,
}

pub async fn auth_middleware(
    State(state): State<AppState>,
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // SECURITY NOTE (accepted risk): Token validated once at middleware entry.
    // In-flight requests continue if token is revoked mid-processing. Narrow
    // TOCTOU window acceptable for localhost threat model. Tokens are opaque
    // UUIDs with 1-hour TTL; revocation is best-effort for in-flight requests.
    let oauth_store = req
        .extensions()
        .get::<Arc<OAuthStore>>()
        .cloned()
        .expect("OAuthStore must be in request extensions");

    let validation = validate_bearer(req.headers(), &oauth_store);
    let (plugin_id, authorization_details) = match validation {
        TokenValidation::Valid {
            plugin_id: Some(pid),
            authorization_details,
            ..
        } => (pid, authorization_details),
        TokenValidation::Valid { plugin_id: None, .. } => return Err(StatusCode::FORBIDDEN),
        TokenValidation::Invalid => return Err(StatusCode::UNAUTHORIZED),
        TokenValidation::Missing => return Err(StatusCode::UNAUTHORIZED),
    };

    let path = req.uri().path().to_string();
    let method = req.method().clone();

    // Check permission for this endpoint: token fast path → PermissionStore fallback
    if let Some(required_perm) = required_permission_for_endpoint(&path, &method) {
        // Fast path: check authorization_details on the token (zero store lookups)
        if !rar::details_satisfy(&authorization_details, &required_perm) {
            // Fallback: check PermissionStore (handles stale tokens, Deferred, etc.)
            let mgr = state.read().await;
            let perm_state = mgr.permissions.get_state(&plugin_id, &required_perm);
            drop(mgr);

            match perm_state {
                Some(PermissionState::Active) => {
                    // Stale token — permission is active in store, proceed
                }
                Some(PermissionState::Deferred) => {
                    // JIT approval for deferred built-in permissions
                    let bridge = req
                        .extensions()
                        .get::<Arc<ApprovalBridge>>()
                        .cloned()
                        .expect("ApprovalBridge must be in request extensions");

                    let plugin_name = {
                        let mgr = state.read().await;
                        mgr.storage
                            .get(&plugin_id)
                            .map(|p| p.manifest.name.clone())
                            .unwrap_or_else(|| plugin_id.clone())
                    };

                    let mut context = std::collections::HashMap::new();
                    context.insert("permission".to_string(), required_perm.as_str().to_string());
                    context.insert("description".to_string(), required_perm.description().to_string());

                    let request = ApprovalRequest {
                        id: uuid::Uuid::new_v4().to_string(),
                        plugin_id: plugin_id.clone(),
                        plugin_name,
                        category: "deferred_permission".to_string(),
                        permission: required_perm.as_str().to_string(),
                        context,
                    };

                    match bridge.request_approval(request).await {
                        super::approval::ApprovalDecision::Approve => {
                            let mgr = state.read().await;
                            let _ = mgr.permissions.activate(&plugin_id, &required_perm);
                        }
                        super::approval::ApprovalDecision::ApproveOnce => {
                            // Don't persist, just continue this request
                        }
                        super::approval::ApprovalDecision::Deny => {
                            let mgr = state.read().await;
                            let _ = mgr.permissions.revoke(&plugin_id, &required_perm);
                            log::warn!(
                                "AUDIT DENIED plugin={} method={} path={} reason=deferred_denied",
                                plugin_id, method, path
                            );
                            return Err(StatusCode::FORBIDDEN);
                        }
                    }
                }
                Some(PermissionState::Revoked) | None => {
                    log::warn!(
                        "AUDIT DENIED plugin={} method={} path={} reason=missing_permission",
                        plugin_id, method, path
                    );
                    return Err(StatusCode::FORBIDDEN);
                }
            }
        }
    }

    req.extensions_mut()
        .insert(AuthenticatedPlugin { plugin_id: plugin_id.clone() });

    let response = next.run(req).await;
    let status = response.status();

    log::info!(
        "AUDIT plugin={} method={} path={} status={}",
        plugin_id, method, path, status.as_u16()
    );

    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{routing::get, Extension, Router, middleware as axum_mw};
    use tower::ServiceExt;

    use crate::permissions::{DefaultPermissionService, Permission, PermissionStore};
    use crate::runtime::mock::MockRuntime;
    use crate::plugin_manager::PluginManager;

    const PLUGIN_ID: &str = "com.test.plugin";

    /// Build an AppState + OAuthStore for tests. Returns (state, oauth_store).
    fn test_state(
        data_dir: &std::path::Path,
        permissions: Arc<dyn crate::permissions::service::PermissionService>,
    ) -> (AppState, Arc<OAuthStore>) {
        let oauth_store = Arc::new(OAuthStore::load(data_dir));
        let mock = Arc::new(MockRuntime::new());
        let mgr = PluginManager::new(
            data_dir.to_path_buf(),
            mock,
            permissions,
            oauth_store.clone(),
        );
        let state: AppState = Arc::new(tokio::sync::RwLock::new(mgr));
        (state, oauth_store)
    }

    /// Create a plugin Bearer token (client_credentials flow).
    fn plugin_token(
        oauth_store: &OAuthStore,
        details: Vec<crate::permissions::rar::AuthorizationDetail>,
    ) -> String {
        let (client, secret) = oauth_store.register_plugin_client(PLUGIN_ID, "Test Plugin");
        oauth_store.set_plugin_auth_details(&client.client_id, details.clone());
        let (access, _) = oauth_store
            .issue_client_credentials(&client.client_id, &secret, "".into(), details)
            .unwrap();
        access.token
    }

    /// Create a non-plugin Bearer token (no plugin_id).
    fn non_plugin_token(oauth_store: &OAuthStore) -> String {
        let token = oauth_store.create_access_token(
            "external-client".into(),
            "External".into(),
            vec![],
            "".into(),
            None,
            vec![],
        );
        token.token
    }

    /// Build a test router with auth_middleware on the given path.
    fn test_app(state: AppState, oauth_store: Arc<OAuthStore>, path: &str) -> Router {
        Router::new()
            .route(path, get(|| async { "ok" }))
            .layer(axum_mw::from_fn_with_state(state.clone(), auth_middleware))
            .layer(Extension(oauth_store))
            .with_state(state)
    }

    // =====================================================================
    // Token validation
    // =====================================================================

    #[tokio::test]
    async fn missing_token_returns_401() {
        let tmp = tempfile::tempdir().unwrap();
        let perms: Arc<dyn crate::permissions::service::PermissionService> =
            Arc::new(DefaultPermissionService::new(PermissionStore::load(tmp.path()).unwrap()));
        let (state, oauth_store) = test_state(tmp.path(), perms);
        let app = test_app(state, oauth_store, "/v1/settings");

        let req = Request::get("/v1/settings").body(Body::empty()).unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn invalid_token_returns_401() {
        let tmp = tempfile::tempdir().unwrap();
        let perms: Arc<dyn crate::permissions::service::PermissionService> =
            Arc::new(DefaultPermissionService::new(PermissionStore::load(tmp.path()).unwrap()));
        let (state, oauth_store) = test_state(tmp.path(), perms);
        let app = test_app(state, oauth_store, "/v1/settings");

        let req = Request::get("/v1/settings")
            .header("authorization", "Bearer bogus-token")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn expired_token_returns_401() {
        let tmp = tempfile::tempdir().unwrap();
        let perms: Arc<dyn crate::permissions::service::PermissionService> =
            Arc::new(DefaultPermissionService::new(PermissionStore::load(tmp.path()).unwrap()));
        let (state, oauth_store) = test_state(tmp.path(), perms);

        let token = plugin_token(&oauth_store, vec![]);
        oauth_store.expire_access_token(&token);

        let app = test_app(state, oauth_store, "/v1/settings");
        let req = Request::get("/v1/settings")
            .header("authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn non_plugin_token_returns_403() {
        let tmp = tempfile::tempdir().unwrap();
        let perms: Arc<dyn crate::permissions::service::PermissionService> =
            Arc::new(DefaultPermissionService::new(PermissionStore::load(tmp.path()).unwrap()));
        let (state, oauth_store) = test_state(tmp.path(), perms);

        let token = non_plugin_token(&oauth_store);
        let app = test_app(state, oauth_store, "/v1/settings");

        let req = Request::get("/v1/settings")
            .header("authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    // =====================================================================
    // Endpoints with no required permission (auth-only)
    // =====================================================================

    #[tokio::test]
    async fn auth_only_endpoint_passes_with_valid_plugin_token() {
        let tmp = tempfile::tempdir().unwrap();
        let perms: Arc<dyn crate::permissions::service::PermissionService> =
            Arc::new(DefaultPermissionService::new(PermissionStore::load(tmp.path()).unwrap()));
        let (state, oauth_store) = test_state(tmp.path(), perms);

        let token = plugin_token(&oauth_store, vec![]);
        // /v1/settings requires no permission, only auth
        let app = test_app(state, oauth_store, "/v1/settings");

        let req = Request::get("/v1/settings")
            .header("authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    // =====================================================================
    // RAR fast path: permission satisfied by token's authorization_details
    // =====================================================================

    #[tokio::test]
    async fn rar_fast_path_allows_request() {
        let tmp = tempfile::tempdir().unwrap();
        let perms: Arc<dyn crate::permissions::service::PermissionService> =
            Arc::new(DefaultPermissionService::new(PermissionStore::load(tmp.path()).unwrap()));
        let (state, oauth_store) = test_state(tmp.path(), perms);

        // Build token with SystemInfo in authorization_details
        let details = rar::build_authorization_details(&[
            crate::permissions::GrantedPermission {
                plugin_id: PLUGIN_ID.to_string(),
                permission: Permission::SystemInfo,
                granted_at: chrono::Utc::now(),
                approved_scopes: None,
                state: PermissionState::Active,
                revoked_at: None,
            },
        ]);
        let token = plugin_token(&oauth_store, details);

        let app = test_app(state, oauth_store, "/v1/system/info");
        let req = Request::get("/v1/system/info")
            .header("authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    // =====================================================================
    // Store fallback: stale token, but Active in PermissionStore
    // =====================================================================

    #[tokio::test]
    async fn stale_token_active_in_store_passes() {
        let tmp = tempfile::tempdir().unwrap();
        let perms: Arc<dyn crate::permissions::service::PermissionService> =
            Arc::new(DefaultPermissionService::new(PermissionStore::load(tmp.path()).unwrap()));
        // Grant permission in store, but token has no RAR (stale scenario)
        perms.grant(PLUGIN_ID, Permission::SystemInfo, None).unwrap();

        let (state, oauth_store) = test_state(tmp.path(), perms);
        let token = plugin_token(&oauth_store, vec![]); // no RAR on token

        let app = test_app(state, oauth_store, "/v1/system/info");
        let req = Request::get("/v1/system/info")
            .header("authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    // =====================================================================
    // Store fallback: Revoked → 403
    // =====================================================================

    #[tokio::test]
    async fn revoked_permission_returns_403() {
        let tmp = tempfile::tempdir().unwrap();
        let perms: Arc<dyn crate::permissions::service::PermissionService> =
            Arc::new(DefaultPermissionService::new(PermissionStore::load(tmp.path()).unwrap()));
        perms.grant(PLUGIN_ID, Permission::SystemInfo, None).unwrap();
        perms.revoke(PLUGIN_ID, &Permission::SystemInfo).unwrap();

        let (state, oauth_store) = test_state(tmp.path(), perms);
        let token = plugin_token(&oauth_store, vec![]);

        let app = test_app(state, oauth_store, "/v1/system/info");
        let req = Request::get("/v1/system/info")
            .header("authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    // =====================================================================
    // Store fallback: no grant at all → 403
    // =====================================================================

    #[tokio::test]
    async fn no_permission_grant_returns_403() {
        let tmp = tempfile::tempdir().unwrap();
        let perms: Arc<dyn crate::permissions::service::PermissionService> =
            Arc::new(DefaultPermissionService::new(PermissionStore::load(tmp.path()).unwrap()));

        let (state, oauth_store) = test_state(tmp.path(), perms);
        let token = plugin_token(&oauth_store, vec![]);

        let app = test_app(state, oauth_store, "/v1/system/info");
        let req = Request::get("/v1/system/info")
            .header("authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    // =====================================================================
    // Different permission types route correctly
    // =====================================================================

    #[tokio::test]
    async fn fs_read_permission_checked() {
        let tmp = tempfile::tempdir().unwrap();
        let perms: Arc<dyn crate::permissions::service::PermissionService> =
            Arc::new(DefaultPermissionService::new(PermissionStore::load(tmp.path()).unwrap()));
        // Grant SystemInfo but NOT FilesystemRead
        perms.grant(PLUGIN_ID, Permission::SystemInfo, None).unwrap();

        let (state, oauth_store) = test_state(tmp.path(), perms);
        let token = plugin_token(&oauth_store, vec![]);

        let app = test_app(state, oauth_store, "/v1/fs/read");
        let req = Request::get("/v1/fs/read")
            .header("authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::FORBIDDEN,
            "SystemInfo grant should not satisfy FilesystemRead"
        );
    }

    #[tokio::test]
    async fn correct_permission_for_endpoint_passes() {
        let tmp = tempfile::tempdir().unwrap();
        let perms: Arc<dyn crate::permissions::service::PermissionService> =
            Arc::new(DefaultPermissionService::new(PermissionStore::load(tmp.path()).unwrap()));
        perms.grant(PLUGIN_ID, Permission::FilesystemRead, None).unwrap();

        let (state, oauth_store) = test_state(tmp.path(), perms);
        let token = plugin_token(&oauth_store, vec![]);

        let app = test_app(state, oauth_store, "/v1/fs/read");
        let req = Request::get("/v1/fs/read")
            .header("authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
