//! OAuth 2.1 Authorization Server for Nexus.
//!
//! Implements the MCP 2025-06-18 auth spec:
//! - RFC 9728 Protected Resource Metadata
//! - RFC 8414 Authorization Server Metadata
//! - RFC 7591 Dynamic Client Registration
//! - Authorization Code + PKCE flow
//!
//! Generic OAuth infrastructure — not MCP-specific. The MCP gateway is the
//! first consumer; plugins can migrate to client_credentials in the future.

pub mod authorize;
pub mod metadata;
pub mod plugin_auth;
pub mod registration;
pub mod store;
pub mod token;
pub mod types;
pub mod validation;

pub use plugin_auth::PluginAuthService;
pub use store::OAuthStore;
