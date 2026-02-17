use std::sync::Arc;

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};

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
    let auth_header = req
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));

    let token = match auth_header {
        Some(t) => t.to_string(),
        None => return Err(StatusCode::UNAUTHORIZED),
    };

    // SECURITY NOTE (accepted risk): Token validated once at middleware entry.
    // In-flight requests continue if token is revoked mid-processing. Narrow
    // TOCTOU window acceptable for localhost threat model. Tokens are opaque
    // UUIDs with 1-hour TTL; revocation is best-effort for in-flight requests.
    let oauth_store = req
        .extensions()
        .get::<Arc<OAuthStore>>()
        .cloned()
        .expect("OAuthStore must be in request extensions");

    let access_token = oauth_store
        .validate_access_token(&token)
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let plugin_id = access_token
        .plugin_id
        .ok_or(StatusCode::FORBIDDEN)?;

    let path = req.uri().path().to_string();
    let method = req.method().clone();

    // Check permission for this endpoint: token fast path → PermissionStore fallback
    if let Some(required_perm) = required_permission_for_endpoint(&path, &method) {
        // Fast path: check authorization_details on the token (zero store lookups)
        if !rar::details_satisfy(&access_token.authorization_details, &required_perm) {
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
