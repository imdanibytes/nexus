use axum::Json;
use serde_json::{json, Value};

/// RFC 9728 — Protected Resource Metadata.
/// Tells MCP clients where to find the authorization server.
///
/// `GET /.well-known/oauth-protected-resource/mcp`
/// `GET /.well-known/oauth-protected-resource` (root fallback)
pub async fn protected_resource() -> Json<Value> {
    Json(json!({
        "resource": "http://127.0.0.1:9600/mcp",
        "authorization_servers": ["http://127.0.0.1:9600"],
        "bearer_methods_supported": ["header"],
        "resource_name": "Nexus MCP Server"
    }))
}

/// RFC 8414 — Authorization Server Metadata.
/// Describes available OAuth endpoints and capabilities.
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
        "authorization_details_types_supported": crate::permissions::rar::SUPPORTED_DETAIL_TYPES
    }))
}
