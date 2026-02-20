//! API key model for Bearer token authentication (RFC 6750).
//!
//! Keys use a `nxk_` prefix to distinguish them from OAuth access tokens in the
//! `Authorization: Bearer` header (RFC 6750 §2.1). The prefix enables O(1) routing
//! in the gateway auth middleware without hitting the OAuth store.
//!
//! Raw key material is shown once at generation and never stored in this struct —
//! only the SHA-256 digest is persisted, following OWASP credential storage guidelines.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A stored API key record. Contains metadata and the key hash — never raw key material.
///
/// The raw key format is `nxk_` + 40 Base62 characters (total 44 chars, ~238 bits entropy).
/// See [`super::store`] for generation and validation logic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    /// Unique identifier (UUID v4).
    pub id: String,
    /// Human-readable label (e.g. "Default", "Claude Code").
    pub name: String,
    /// SHA-256 hex digest of the full raw key. Used for validation via constant-time
    /// comparison (see [`super::store::ApiKeyStore::validate`]).
    pub key_hash: String,
    /// First 8 characters of the raw key (e.g. "nxk_Ab12") for display in the UI.
    /// Safe to expose — insufficient to reconstruct the key.
    pub prefix: String,
    /// When the key was created.
    pub created_at: DateTime<Utc>,
    /// Updated on each successful validation. `None` if never used.
    pub last_used_at: Option<DateTime<Utc>>,
}
