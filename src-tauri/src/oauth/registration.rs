use std::sync::Arc;

use axum::extract::Extension;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;

use super::store::OAuthStore;
use super::types::{RegistrationRequest, RegistrationResponse};

/// RFC 7591 — Dynamic Client Registration.
///
/// `POST /oauth/register`
///
/// MCP clients call this to register themselves before starting the
/// authorization flow. Public clients only (no client_secret issued).
pub async fn register_client(
    Extension(store): Extension<Arc<OAuthStore>>,
    Json(req): Json<RegistrationRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    if req.client_name.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    if req.redirect_uris.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Only public clients (no client_secret support)
    if req.token_endpoint_auth_method != "none" {
        return Err(StatusCode::BAD_REQUEST);
    }

    let client = store.register_client(req);

    log::info!(
        "OAuth client registered: name={} id={}",
        client.client_name,
        client.client_id
    );

    let response = RegistrationResponse {
        client_id: client.client_id,
        client_name: client.client_name,
        redirect_uris: client.redirect_uris,
        grant_types: client.grant_types,
        token_endpoint_auth_method: client.token_endpoint_auth_method,
    };

    Ok((StatusCode::CREATED, Json(response)))
}
