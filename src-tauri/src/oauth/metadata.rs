//! OAuth 2.0 discovery metadata endpoints.
//!
//! Two well-known endpoints per the MCP authorization spec:
//!
//! 1. **Protected Resource Metadata** (RFC 9728 §3) — tells MCP clients which
//!    authorization server protects this resource and what scopes/RAR types are
//!    required to access it.
//!
//! 2. **Authorization Server Metadata** (RFC 8414 §2) — describes OAuth
//!    endpoints, supported grant types, PKCE methods, and registration.
//!
//! MCP clients discover auth requirements by fetching (1) first, then (2)
//! to get the actual endpoint URLs. The `resource_metadata` parameter
//! (RFC 9728 §2) is included in authorization requests to bind the token
//! to this specific resource.

use axum::Json;
use serde_json::{json, Value};

/// RFC 9728 §3 — Protected Resource Metadata.
///
/// Tells MCP clients where to find the authorization server and what
/// access requirements the resource imposes. MCP clients fetch this
/// endpoint first during discovery.
///
/// Fields:
/// - `resource` — canonical resource identifier (RFC 9728 §3, REQUIRED)
/// - `authorization_servers` — AS(s) that can issue tokens (REQUIRED)
/// - `bearer_methods_supported` — how tokens are presented; we only accept
///   `header` (RFC 6750 §2.1), not form-encoded or query string
/// - `scopes_supported` — resource-specific scope requirements (§3)
/// - `authorization_details_types_supported` — RAR types (RFC 9396) the
///   resource expects in tokens, mirrored from the AS for client convenience
/// - `resource_name` — human-readable name (OPTIONAL)
/// - `resource_documentation` — URL to human-readable docs (OPTIONAL)
///
/// `GET /.well-known/oauth-protected-resource/mcp`
/// `GET /.well-known/oauth-protected-resource` (root fallback)
pub async fn protected_resource() -> Json<Value> {
    Json(json!({
        "resource": "http://127.0.0.1:9600/mcp",
        "authorization_servers": ["http://127.0.0.1:9600"],
        "bearer_methods_supported": ["header"],
        "scopes_supported": ["mcp"],
        "authorization_details_types_supported": crate::permissions::rar::SUPPORTED_DETAIL_TYPES,
        "resource_name": "Nexus MCP Server",
        "resource_documentation": "https://github.com/imdanibytes/nexus"
    }))
}

/// RFC 8414 §2 — Authorization Server Metadata.
///
/// Describes OAuth endpoints, capabilities, and supported mechanisms.
/// MCP clients fetch this after discovering the AS URL from the protected
/// resource metadata above.
///
/// Fields:
/// - `issuer` — AS identifier, MUST match the AS URL (§2, REQUIRED)
/// - `authorization_endpoint` — for authorization code flow (RFC 6749 §3.1)
/// - `token_endpoint` — token exchange (RFC 6749 §3.2)
/// - `registration_endpoint` — dynamic client registration (RFC 7591)
/// - `response_types_supported` — only `code` (authorization code grant)
/// - `grant_types_supported` — auth code, refresh, and client_credentials
///   (for plugin machine-to-machine auth)
/// - `code_challenge_methods_supported` — only S256 (RFC 7636); `plain` is
///   intentionally excluded (PKCE downgrade prevention)
/// - `token_endpoint_auth_methods_supported` — `none` for public clients
///   (RFC 6749 §2.1), `client_secret_post` for confidential plugin clients
/// - `scopes_supported` — available scopes
/// - `authorization_details_types_supported` — RFC 9396 RAR types
/// - `service_documentation` — human-readable documentation (§2, OPTIONAL)
///
/// `GET /.well-known/oauth-authorization-server`
pub async fn authorization_server() -> Json<Value> {
    Json(json!({
        "issuer": "http://127.0.0.1:9600",
        "authorization_endpoint": "http://127.0.0.1:9600/oauth/authorize",
        "token_endpoint": "http://127.0.0.1:9600/oauth/token",
        "registration_endpoint": "http://127.0.0.1:9600/oauth/register",
        "response_types_supported": ["code"],
        "grant_types_supported": ["authorization_code", "refresh_token", "client_credentials"],
        "code_challenge_methods_supported": ["S256"],
        "token_endpoint_auth_methods_supported": ["none", "client_secret_post"],
        "scopes_supported": ["mcp"],
        "authorization_details_types_supported": crate::permissions::rar::SUPPORTED_DETAIL_TYPES,
        "service_documentation": "https://github.com/imdanibytes/nexus"
    }))
}
