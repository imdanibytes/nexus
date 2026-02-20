use std::sync::Arc;

use axum::extract::Extension;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::{Form, Json};
use chrono::Utc;

use crate::permissions::rar::AuthorizationDetail;

use super::store::OAuthStore;
use super::types::{AccessToken, TokenRequest, TokenResponse};

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
        "client_credentials" => handle_client_credentials(store, req),
        _ => oauth_error(
            StatusCode::BAD_REQUEST,
            "unsupported_grant_type",
            "Unsupported grant type",
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

    let ttl = expires_in(&access);
    Json(TokenResponse {
        access_token: access.token,
        token_type: "Bearer".into(),
        expires_in: ttl,
        refresh_token: refresh.map(|r| r.token),
        authorization_details: None,
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

    let details = if access.authorization_details.is_empty() {
        None
    } else {
        Some(access.authorization_details.clone())
    };

    let ttl = expires_in(&access);
    Json(TokenResponse {
        access_token: access.token,
        token_type: "Bearer".into(),
        expires_in: ttl,
        refresh_token: Some(new_refresh.token),
        authorization_details: details,
    })
    .into_response()
}

fn handle_client_credentials(store: Arc<OAuthStore>, req: TokenRequest) -> Response {
    let Some(client_id) = req.client_id.as_deref() else {
        return oauth_error(StatusCode::BAD_REQUEST, "invalid_request", "Missing 'client_id'");
    };
    let Some(client_secret) = req.client_secret.as_deref() else {
        return oauth_error(StatusCode::BAD_REQUEST, "invalid_request", "Missing 'client_secret'");
    };

    let resource = req.resource.unwrap_or_default();

    // Parse RFC 9396 authorization_details from the form body (JSON string)
    let auth_details: Vec<AuthorizationDetail> = req
        .authorization_details
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();

    let (access, refresh) = match store.issue_client_credentials(client_id, client_secret, resource, auth_details) {
        Ok(pair) => pair,
        Err(e) => {
            log::warn!("OAuth client_credentials failed: {}", e);
            return oauth_error(StatusCode::UNAUTHORIZED, e, "Client authentication failed");
        }
    };

    log::info!(
        "OAuth token issued: client={} grant=client_credentials auth_details={}",
        access.client_name,
        access.authorization_details.len(),
    );

    let details = if access.authorization_details.is_empty() {
        None
    } else {
        Some(access.authorization_details.clone())
    };

    let ttl = expires_in(&access);
    Json(TokenResponse {
        access_token: access.token,
        token_type: "Bearer".into(),
        expires_in: ttl,
        refresh_token: Some(refresh.token),
        authorization_details: details,
    })
    .into_response()
}

/// Compute the remaining lifetime of an access token in seconds.
fn expires_in(token: &AccessToken) -> u64 {
    (token.expires_at - Utc::now()).num_seconds().max(0) as u64
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
