use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::extract::{Extension, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::AppState;

// ── Session Store ──────────────────────────────────────────────

struct Session {
    plugin_id: String,
    expires_at: Instant,
}

/// In-memory store for short-lived access tokens.
///
/// Plugins exchange their long-lived secret for an access token via
/// `POST /v1/auth/token`. The middleware validates requests against
/// this store instead of checking the secret directly.
pub struct SessionStore {
    sessions: Mutex<HashMap<String, Session>>,
    ttl: Duration,
}

impl SessionStore {
    pub fn new(ttl: Duration) -> Self {
        SessionStore {
            sessions: Mutex::new(HashMap::new()),
            ttl,
        }
    }

    /// Create a new session for a plugin. Returns the access token.
    pub fn create(&self, plugin_id: String) -> String {
        let token = uuid::Uuid::new_v4().to_string();
        let session = Session {
            plugin_id,
            expires_at: Instant::now() + self.ttl,
        };
        let mut sessions = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
        sessions.insert(token.clone(), session);
        token
    }

    /// Validate an access token. Returns the plugin ID if valid and not expired.
    pub fn validate(&self, token: &str) -> Option<String> {
        let sessions = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
        sessions.get(token).and_then(|s| {
            if Instant::now() < s.expires_at {
                Some(s.plugin_id.clone())
            } else {
                None
            }
        })
    }

    /// Remove expired sessions. Called lazily on token creation.
    pub fn cleanup(&self) {
        let mut sessions = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
        let now = Instant::now();
        sessions.retain(|_, s| now < s.expires_at);
    }

    /// TTL in seconds (for the `expires_in` response field).
    pub fn ttl_secs(&self) -> u64 {
        self.ttl.as_secs()
    }

    /// Revoke all sessions for a plugin (e.g. on plugin stop/remove).
    pub fn revoke_plugin(&self, plugin_id: &str) {
        let mut sessions = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
        sessions.retain(|_, s| s.plugin_id != plugin_id);
    }
}

// ── Handler ────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct TokenRequest {
    pub secret: String,
}

#[derive(Serialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub expires_in: u64,
    pub plugin_id: String,
}

/// Exchange a plugin secret for a short-lived access token.
///
/// `POST /v1/auth/token`
///
/// The plugin secret (`NEXUS_PLUGIN_SECRET`) is injected into each container
/// at start time. Plugins call this endpoint once (and again on expiry) to
/// get an access token for all subsequent API calls.
pub async fn create_token(
    State(state): State<AppState>,
    Extension(sessions): Extension<Arc<SessionStore>>,
    Json(req): Json<TokenRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let mgr = state.read().await;
    let plugin = mgr
        .storage
        .find_by_token(&req.secret)
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let plugin_id = plugin.manifest.id.clone();
    drop(mgr);

    // Cleanup expired sessions lazily (cheap, mutex is fast)
    sessions.cleanup();

    let access_token = sessions.create(plugin_id.clone());

    log::info!(
        "AUDIT plugin={} action=token_exchange ttl={}s",
        plugin_id,
        sessions.ttl_secs()
    );

    Ok(Json(TokenResponse {
        access_token,
        expires_in: sessions.ttl_secs(),
        plugin_id,
    }))
}

// ── Tests ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_and_validate() {
        let store = SessionStore::new(Duration::from_secs(60));
        let token = store.create("plug-a".into());
        assert_eq!(store.validate(&token), Some("plug-a".to_string()));
    }

    #[test]
    fn invalid_token_returns_none() {
        let store = SessionStore::new(Duration::from_secs(60));
        assert_eq!(store.validate("bogus"), None);
    }

    #[test]
    fn expired_token_returns_none() {
        let store = SessionStore::new(Duration::from_millis(1));
        let token = store.create("plug-a".into());
        std::thread::sleep(Duration::from_millis(10));
        assert_eq!(store.validate(&token), None);
    }

    #[test]
    fn each_create_returns_unique_token() {
        let store = SessionStore::new(Duration::from_secs(60));
        let t1 = store.create("plug-a".into());
        let t2 = store.create("plug-a".into());
        assert_ne!(t1, t2);
    }

    #[test]
    fn cleanup_removes_expired() {
        let store = SessionStore::new(Duration::from_millis(1));
        let _t1 = store.create("plug-a".into());
        let _t2 = store.create("plug-b".into());
        std::thread::sleep(Duration::from_millis(10));

        // Create a non-expired session
        let store2 = SessionStore::new(Duration::from_secs(60));
        // Can't change TTL per-session, so just verify cleanup on the short-lived store
        store.cleanup();

        let sessions = store.sessions.lock().unwrap();
        assert!(sessions.is_empty(), "expired sessions should be cleaned up");
        drop(sessions);

        // Verify non-expired store is fine
        let t3 = store2.create("plug-c".into());
        store2.cleanup();
        assert_eq!(store2.validate(&t3), Some("plug-c".to_string()));
    }

    #[test]
    fn revoke_plugin_removes_all_sessions() {
        let store = SessionStore::new(Duration::from_secs(60));
        let t1 = store.create("plug-a".into());
        let t2 = store.create("plug-a".into());
        let t3 = store.create("plug-b".into());

        store.revoke_plugin("plug-a");

        assert_eq!(store.validate(&t1), None);
        assert_eq!(store.validate(&t2), None);
        assert_eq!(store.validate(&t3), Some("plug-b".to_string()));
    }

    #[test]
    fn multiple_plugins_independent() {
        let store = SessionStore::new(Duration::from_secs(60));
        let ta = store.create("plug-a".into());
        let tb = store.create("plug-b".into());

        assert_eq!(store.validate(&ta), Some("plug-a".to_string()));
        assert_eq!(store.validate(&tb), Some("plug-b".to_string()));
    }
}
