use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Instant;

// ---------------------------------------------------------------------------
// Client registration (RFC 7591)
// ---------------------------------------------------------------------------

/// A registered OAuth 2.1 client (e.g. "Claude Code", "Cursor").
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthClient {
    pub client_id: String,
    pub client_name: String,
    pub redirect_uris: Vec<String>,
    pub grant_types: Vec<String>,
    pub token_endpoint_auth_method: String,
    pub registered_at: DateTime<Utc>,
    /// Whether the user explicitly approved this client (skip consent on reconnect).
    #[serde(default)]
    pub approved: bool,
}

/// Inbound registration request body.
#[derive(Debug, Deserialize)]
pub struct RegistrationRequest {
    pub client_name: String,
    pub redirect_uris: Vec<String>,
    #[serde(default = "default_grant_types")]
    pub grant_types: Vec<String>,
    #[serde(default = "default_auth_method")]
    pub token_endpoint_auth_method: String,
}

fn default_grant_types() -> Vec<String> {
    vec!["authorization_code".into()]
}

fn default_auth_method() -> String {
    "none".into()
}

/// Registration response (mirrors request + server-assigned fields).
#[derive(Debug, Serialize)]
pub struct RegistrationResponse {
    pub client_id: String,
    pub client_name: String,
    pub redirect_uris: Vec<String>,
    pub grant_types: Vec<String>,
    pub token_endpoint_auth_method: String,
}

// ---------------------------------------------------------------------------
// Frontend-facing client info (serializable subset of OAuthClient)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct OAuthClientInfo {
    pub client_id: String,
    pub client_name: String,
    pub registered_at: DateTime<Utc>,
    pub approved: bool,
}

impl From<&OAuthClient> for OAuthClientInfo {
    fn from(c: &OAuthClient) -> Self {
        Self {
            client_id: c.client_id.clone(),
            client_name: c.client_name.clone(),
            registered_at: c.registered_at,
            approved: c.approved,
        }
    }
}

// ---------------------------------------------------------------------------
// Authorization code (short-lived, in-memory only)
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct AuthorizationCode {
    pub code: String,
    pub client_id: String,
    pub redirect_uri: String,
    pub code_challenge: String,
    pub scopes: Vec<String>,
    pub resource: String,
    pub state: String,
    pub expires_at: Instant,
    pub used: bool,
    /// When true, token exchange will not issue a refresh token.
    pub no_refresh: bool,
}

// ---------------------------------------------------------------------------
// Tokens
// ---------------------------------------------------------------------------

/// In-memory access token (lost on restart — clients refresh or re-auth).
#[derive(Debug, Clone)]
pub struct AccessToken {
    pub token: String,
    pub client_id: String,
    pub client_name: String,
    pub scopes: Vec<String>,
    pub resource: String,
    pub expires_at: Instant,
}

/// Persistent refresh token (survives restarts).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshToken {
    pub token: String,
    pub client_id: String,
    pub scopes: Vec<String>,
    pub resource: String,
    pub expires_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Token endpoint request (application/x-www-form-urlencoded)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct TokenRequest {
    pub grant_type: String,
    // authorization_code fields
    pub code: Option<String>,
    pub code_verifier: Option<String>,
    pub redirect_uri: Option<String>,
    pub client_id: Option<String>,
    pub resource: Option<String>,
    // refresh_token fields
    pub refresh_token: Option<String>,
}

/// Token endpoint response.
#[derive(Debug, Serialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
}

// ---------------------------------------------------------------------------
// Authorization request query params
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct AuthorizeParams {
    pub response_type: String,
    pub client_id: String,
    pub redirect_uri: String,
    pub code_challenge: String,
    pub code_challenge_method: String,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub scope: String,
    #[serde(default)]
    pub resource: String,
}
