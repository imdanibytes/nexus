//! First-class API key authentication for the MCP gateway.
//!
//! Provides `Authorization: Bearer nxk_...` authentication as an alternative to the
//! full OAuth 2.0 flow (RFC 6749). API keys are localhost-only, long-lived credentials
//! designed for local AI clients (Claude Code, Cursor, etc.) where browser-based OAuth
//! is unnecessarily friction-heavy.
//!
//! # Relevant RFCs
//!
//! - **RFC 6750** — Bearer Token Usage (token format and `Authorization` header)
//! - **RFC 7235** — HTTP Authentication (challenge/response framework)
//! - **RFC 9728** — OAuth 2.0 Protected Resource Metadata (`resource_metadata` parameter)

pub mod store;
pub mod types;

pub use store::ApiKeyStore;
pub use types::ApiKey;
