use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};

use crate::permissions::checker::required_permission_for_endpoint;
use crate::AppState;

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

    let mgr = state.read().await;

    let plugin = mgr
        .storage
        .find_by_token(&token)
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let plugin_id = plugin.manifest.id.clone();
    let path = req.uri().path().to_string();
    let method = req.method().clone();

    // Check permission for this endpoint
    if let Some(required_perm) = required_permission_for_endpoint(&path) {
        if !mgr.permissions.has_permission(&plugin_id, &required_perm) {
            log::warn!(
                "AUDIT DENIED plugin={} method={} path={} reason=missing_permission",
                plugin_id, method, path
            );
            return Err(StatusCode::FORBIDDEN);
        }
    }

    req.extensions_mut()
        .insert(AuthenticatedPlugin { plugin_id: plugin_id.clone() });

    drop(mgr);

    let response = next.run(req).await;
    let status = response.status();

    log::info!(
        "AUDIT plugin={} method={} path={} status={}",
        plugin_id, method, path, status.as_u16()
    );

    Ok(response)
}
