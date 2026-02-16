use std::sync::Arc;

use axum::extract::Extension;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::{Form, Json};

use super::store::OAuthStore;
use super::types::{TokenRequest, TokenResponse};

/// OAuth 2.1 Token Endpoint.
///
/// `POST /oauth/token` (application/x-www-form-urlencoded)
///
/// Supports two grant types:
/// - `authorization_code` — exchange auth code for tokens (with PKCE)
/// - `refresh_token` — rotate refresh token for new tokens
pub async fn token_exchange(
    Extension(store): Extension<Arc<OAuthStore>>,
    Form(req): Form<TokenRequest>,
) -> Response {
    match req.grant_type.as_str() {
        "authorization_code" => handle_authorization_code(store, req),
        "refresh_token" => handle_refresh_token(store, req),
        _ => oauth_error(
            StatusCode::BAD_REQUEST,
            "unsupported_grant_type",
            "Only authorization_code and refresh_token are supported",
        ),
    }
}

fn handle_authorization_code(store: Arc<OAuthStore>, req: TokenRequest) -> Response {
    let Some(code) = req.code.as_deref() else {
        return oauth_error(StatusCode::BAD_REQUEST, "invalid_request", "Missing 'code'");
    };
    let Some(code_verifier) = req.code_verifier.as_deref() else {
        return oauth_error(StatusCode::BAD_REQUEST, "invalid_request", "Missing 'code_verifier'");
    };
    let Some(client_id) = req.client_id.as_deref() else {
        return oauth_error(StatusCode::BAD_REQUEST, "invalid_request", "Missing 'client_id'");
    };
    let Some(redirect_uri) = req.redirect_uri.as_deref() else {
        return oauth_error(StatusCode::BAD_REQUEST, "invalid_request", "Missing 'redirect_uri'");
    };

    let (access, refresh) = match store.exchange_code(code, code_verifier, client_id, redirect_uri)
    {
        Ok(pair) => pair,
        Err(e) => {
            log::warn!("OAuth token exchange failed: {}", e);
            return oauth_error(StatusCode::BAD_REQUEST, e, "Token exchange failed");
        }
    };

    log::info!(
        "OAuth token issued: client={} grant=authorization_code refresh={}",
        access.client_name,
        refresh.is_some(),
    );

    Json(TokenResponse {
        access_token: access.token,
        token_type: "Bearer".into(),
        expires_in: 3600,
        refresh_token: refresh.map(|r| r.token),
    })
    .into_response()
}

fn handle_refresh_token(store: Arc<OAuthStore>, req: TokenRequest) -> Response {
    let Some(refresh_token) = req.refresh_token.as_deref() else {
        return oauth_error(StatusCode::BAD_REQUEST, "invalid_request", "Missing 'refresh_token'");
    };
    let Some(client_id) = req.client_id.as_deref() else {
        return oauth_error(StatusCode::BAD_REQUEST, "invalid_request", "Missing 'client_id'");
    };

    let (access, new_refresh) = match store.refresh(refresh_token, client_id) {
        Ok(pair) => pair,
        Err(e) => {
            log::warn!("OAuth refresh failed: {}", e);
            return oauth_error(StatusCode::BAD_REQUEST, e, "Refresh failed");
        }
    };

    log::info!(
        "OAuth token refreshed: client={} grant=refresh_token",
        access.client_name
    );

    Json(TokenResponse {
        access_token: access.token,
        token_type: "Bearer".into(),
        expires_in: 3600,
        refresh_token: Some(new_refresh.token),
    })
    .into_response()
}

fn oauth_error(status: StatusCode, error: &str, description: &str) -> Response {
    (
        status,
        Json(serde_json::json!({
            "error": error,
            "error_description": description
        })),
    )
        .into_response()
}
