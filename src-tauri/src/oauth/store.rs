use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::Utc;
use sha2::{Digest, Sha256};

use super::types::*;

const AUTH_CODE_TTL: Duration = Duration::from_secs(10 * 60); // 10 minutes
const ACCESS_TOKEN_TTL: Duration = Duration::from_secs(60 * 60); // 1 hour
const REFRESH_TOKEN_DAYS: i64 = 30;

// ---------------------------------------------------------------------------
// OAuthStore — the single source of truth for all OAuth state
// ---------------------------------------------------------------------------

pub struct OAuthStore {
    data_dir: PathBuf,
    clients: Mutex<HashMap<String, OAuthClient>>,
    auth_codes: Mutex<HashMap<String, AuthorizationCode>>,
    access_tokens: Mutex<HashMap<String, AccessToken>>,
    refresh_tokens: Mutex<HashMap<String, RefreshToken>>,
}

impl OAuthStore {
    /// Load persisted clients and refresh tokens from disk, or create empty store.
    pub fn load(data_dir: &Path) -> Self {
        let clients_path = data_dir.join("oauth_clients.json");
        let refresh_path = data_dir.join("oauth_refresh.json");

        let clients: HashMap<String, OAuthClient> =
            std::fs::read_to_string(&clients_path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default();

        let refresh_tokens: HashMap<String, RefreshToken> =
            std::fs::read_to_string(&refresh_path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default();

        let now = Utc::now();
        let refresh_tokens: HashMap<String, RefreshToken> = refresh_tokens
            .into_iter()
            .filter(|(_, t)| t.expires_at > now)
            .collect();

        log::info!(
            "OAuth store loaded: {} clients, {} refresh tokens",
            clients.len(),
            refresh_tokens.len()
        );

        Self {
            data_dir: data_dir.to_path_buf(),
            clients: Mutex::new(clients),
            auth_codes: Mutex::new(HashMap::new()),
            access_tokens: Mutex::new(HashMap::new()),
            refresh_tokens: Mutex::new(refresh_tokens),
        }
    }

    // ── Client Registration ──────────────────────────────────────

    /// Register a new OAuth client. Idempotent by `client_name` — if a client
    /// with the same name already exists, returns it (updating redirect URIs).
    pub fn register_client(&self, req: RegistrationRequest) -> OAuthClient {
        let mut clients = self.clients.lock().unwrap_or_else(|e| e.into_inner());

        // Idempotent: return existing client with same name, updating redirect URIs
        if let Some(existing) = clients.values_mut().find(|c| c.client_name == req.client_name) {
            existing.redirect_uris = req.redirect_uris.into_iter().map(normalize_redirect_uri).collect();
            let result = existing.clone();
            drop(clients);
            self.save_clients();
            return result;
        }

        let client = OAuthClient {
            client_id: uuid::Uuid::new_v4().to_string(),
            client_name: req.client_name,
            redirect_uris: req.redirect_uris.into_iter().map(normalize_redirect_uri).collect(),
            grant_types: req.grant_types,
            token_endpoint_auth_method: req.token_endpoint_auth_method,
            registered_at: Utc::now(),
            approved: false,
        };

        clients.insert(client.client_id.clone(), client.clone());
        drop(clients);
        self.save_clients();
        client
    }

    pub fn get_client(&self, client_id: &str) -> Option<OAuthClient> {
        let clients = self.clients.lock().unwrap_or_else(|e| e.into_inner());
        clients.get(client_id).cloned()
    }

    pub fn is_client_approved(&self, client_id: &str) -> bool {
        let clients = self.clients.lock().unwrap_or_else(|e| e.into_inner());
        clients.get(client_id).map(|c| c.approved).unwrap_or(false)
    }

    pub fn approve_client(&self, client_id: &str) {
        let mut clients = self.clients.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(client) = clients.get_mut(client_id) {
            client.approved = true;
        }
        drop(clients);
        self.save_clients();
    }

    pub fn list_clients(&self) -> Vec<OAuthClient> {
        let clients = self.clients.lock().unwrap_or_else(|e| e.into_inner());
        clients.values().cloned().collect()
    }

    // ── Authorization Codes ──────────────────────────────────────

    pub fn create_authorization_code(
        &self,
        client_id: String,
        redirect_uri: String,
        code_challenge: String,
        scopes: Vec<String>,
        resource: String,
        state: String,
    ) -> String {
        self.create_authorization_code_inner(client_id, redirect_uri, code_challenge, scopes, resource, state, false)
    }

    /// Like `create_authorization_code`, but the resulting token exchange
    /// will NOT issue a refresh token (used for "Allow for 1 hour" consent).
    pub fn create_authorization_code_once(
        &self,
        client_id: String,
        redirect_uri: String,
        code_challenge: String,
        scopes: Vec<String>,
        resource: String,
        state: String,
    ) -> String {
        self.create_authorization_code_inner(client_id, redirect_uri, code_challenge, scopes, resource, state, true)
    }

    #[allow(clippy::too_many_arguments)]
    fn create_authorization_code_inner(
        &self,
        client_id: String,
        redirect_uri: String,
        code_challenge: String,
        scopes: Vec<String>,
        resource: String,
        state: String,
        no_refresh: bool,
    ) -> String {
        let code = uuid::Uuid::new_v4().to_string();
        let auth_code = AuthorizationCode {
            code: code.clone(),
            client_id,
            redirect_uri,
            code_challenge,
            scopes,
            resource,
            state,
            expires_at: Instant::now() + AUTH_CODE_TTL,
            used: false,
            no_refresh,
        };
        let mut codes = self.auth_codes.lock().unwrap_or_else(|e| e.into_inner());
        // Lazy cleanup
        let now = Instant::now();
        codes.retain(|_, c| now < c.expires_at && !c.used);
        codes.insert(code.clone(), auth_code);
        code
    }

    /// Exchange an authorization code for tokens. Validates PKCE, single-use,
    /// client_id, and redirect_uri.
    ///
    /// Returns `(access_token, Option<refresh_token>)`. The refresh token is
    /// `None` when the auth code was created via "Allow for 1 hour" consent.
    pub fn exchange_code(
        &self,
        code: &str,
        code_verifier: &str,
        client_id: &str,
        redirect_uri: &str,
    ) -> Result<(AccessToken, Option<RefreshToken>), &'static str> {
        let mut codes = self.auth_codes.lock().unwrap_or_else(|e| e.into_inner());
        let auth_code = codes.get_mut(code).ok_or("invalid_grant")?;

        if auth_code.used {
            return Err("invalid_grant");
        }
        if Instant::now() >= auth_code.expires_at {
            return Err("invalid_grant");
        }
        if auth_code.client_id != client_id {
            return Err("invalid_grant");
        }
        if normalize_redirect_uri(auth_code.redirect_uri.clone()) != normalize_redirect_uri(redirect_uri.to_string()) {
            return Err("invalid_grant");
        }

        // PKCE validation: base64url(sha256(code_verifier)) must match code_challenge
        if !verify_pkce(code_verifier, &auth_code.code_challenge) {
            return Err("invalid_grant");
        }

        auth_code.used = true;
        let no_refresh = auth_code.no_refresh;

        let scopes = auth_code.scopes.clone();
        let resource = auth_code.resource.clone();
        let client_name = {
            let clients = self.clients.lock().unwrap_or_else(|e| e.into_inner());
            clients
                .get(client_id)
                .map(|c| c.client_name.clone())
                .unwrap_or_default()
        };

        drop(codes);

        let access = self.create_access_token(client_id.to_string(), client_name, scopes.clone(), resource.clone());
        let refresh = if no_refresh {
            None
        } else {
            Some(self.create_refresh_token(client_id.to_string(), scopes, resource))
        };

        Ok((access, refresh))
    }

    // ── Access Tokens ────────────────────────────────────────────

    fn create_access_token(
        &self,
        client_id: String,
        client_name: String,
        scopes: Vec<String>,
        resource: String,
    ) -> AccessToken {
        let token = AccessToken {
            token: uuid::Uuid::new_v4().to_string(),
            client_id,
            client_name,
            scopes,
            resource,
            expires_at: Instant::now() + ACCESS_TOKEN_TTL,
        };
        let mut tokens = self.access_tokens.lock().unwrap_or_else(|e| e.into_inner());
        // Lazy cleanup
        let now = Instant::now();
        tokens.retain(|_, t| now < t.expires_at);
        tokens.insert(token.token.clone(), token.clone());
        token
    }

    /// Validate an access token. Returns token info if valid.
    pub fn validate_access_token(&self, token: &str) -> Option<AccessToken> {
        let tokens = self.access_tokens.lock().unwrap_or_else(|e| e.into_inner());
        tokens.get(token).and_then(|t| {
            if Instant::now() < t.expires_at {
                Some(t.clone())
            } else {
                None
            }
        })
    }

    // ── Refresh Tokens ───────────────────────────────────────────

    fn create_refresh_token(
        &self,
        client_id: String,
        scopes: Vec<String>,
        resource: String,
    ) -> RefreshToken {
        let token = RefreshToken {
            token: uuid::Uuid::new_v4().to_string(),
            client_id,
            scopes,
            resource,
            expires_at: Utc::now() + chrono::Duration::days(REFRESH_TOKEN_DAYS),
        };
        let mut tokens = self.refresh_tokens.lock().unwrap_or_else(|e| e.into_inner());
        tokens.insert(token.token.clone(), token.clone());
        drop(tokens);
        self.save_refresh_tokens();
        token
    }

    /// Refresh an access token. Rotates the refresh token (old one is invalidated).
    pub fn refresh(
        &self,
        refresh_token: &str,
        client_id: &str,
    ) -> Result<(AccessToken, RefreshToken), &'static str> {
        let mut tokens = self.refresh_tokens.lock().unwrap_or_else(|e| e.into_inner());
        let old = tokens.remove(refresh_token).ok_or("invalid_grant")?;

        if old.client_id != client_id {
            return Err("invalid_grant");
        }
        if old.expires_at < Utc::now() {
            return Err("invalid_grant");
        }

        let scopes = old.scopes;
        let resource = old.resource;
        drop(tokens);

        let client_name = {
            let clients = self.clients.lock().unwrap_or_else(|e| e.into_inner());
            clients
                .get(client_id)
                .map(|c| c.client_name.clone())
                .unwrap_or_default()
        };

        let access = self.create_access_token(client_id.to_string(), client_name, scopes.clone(), resource.clone());
        let refresh = self.create_refresh_token(client_id.to_string(), scopes, resource);

        self.save_refresh_tokens();
        Ok((access, refresh))
    }

    // ── Revocation ───────────────────────────────────────────────

    /// Revoke all tokens for a client and remove it from the store entirely.
    /// The client will need to re-register and go through consent again.
    pub fn revoke_client(&self, client_id: &str) {
        {
            let mut tokens = self.access_tokens.lock().unwrap_or_else(|e| e.into_inner());
            tokens.retain(|_, t| t.client_id != client_id);
        }
        {
            let mut tokens = self.refresh_tokens.lock().unwrap_or_else(|e| e.into_inner());
            tokens.retain(|_, t| t.client_id != client_id);
        }
        {
            let mut codes = self.auth_codes.lock().unwrap_or_else(|e| e.into_inner());
            codes.retain(|_, c| c.client_id != client_id);
        }
        {
            let mut clients = self.clients.lock().unwrap_or_else(|e| e.into_inner());
            clients.remove(client_id);
        }
        self.save_clients();
        self.save_refresh_tokens();
    }

    // ── Test Helpers ─────────────────────────────────────────────

    /// Force an auth code to expire (test-only).
    #[cfg(test)]
    pub fn expire_auth_code(&self, code: &str) {
        let mut codes = self.auth_codes.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(c) = codes.get_mut(code) {
            c.expires_at = Instant::now() - Duration::from_secs(1);
        }
    }

    /// Force an access token to expire (test-only).
    #[cfg(test)]
    pub fn expire_access_token(&self, token: &str) {
        let mut tokens = self.access_tokens.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(t) = tokens.get_mut(token) {
            t.expires_at = Instant::now() - Duration::from_secs(1);
        }
    }

    // ── Persistence ──────────────────────────────────────────────

    fn save_clients(&self) {
        let clients = self.clients.lock().unwrap_or_else(|e| e.into_inner());
        let json = serde_json::to_string_pretty(&*clients).unwrap_or_default();
        let path = self.data_dir.join("oauth_clients.json");
        if let Err(e) = std::fs::write(&path, json) {
            log::error!("Failed to save OAuth clients: {}", e);
        }
    }

    fn save_refresh_tokens(&self) {
        let tokens = self.refresh_tokens.lock().unwrap_or_else(|e| e.into_inner());
        let json = serde_json::to_string_pretty(&*tokens).unwrap_or_default();
        let path = self.data_dir.join("oauth_refresh.json");
        if let Err(e) = std::fs::write(&path, json) {
            log::error!("Failed to save OAuth refresh tokens: {}", e);
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// PKCE S256 verification: base64url(sha256(verifier)) == challenge
fn verify_pkce(code_verifier: &str, code_challenge: &str) -> bool {
    let mut hasher = Sha256::new();
    hasher.update(code_verifier.as_bytes());
    let computed = URL_SAFE_NO_PAD.encode(hasher.finalize());
    computed == code_challenge
}

/// Normalize localhost variants in redirect URIs.
/// Claude Code sometimes uses `localhost` vs `127.0.0.1` inconsistently.
pub(crate) fn normalize_redirect_uri(uri: String) -> String {
    uri.replace("://localhost:", "://127.0.0.1:")
        .replace("://localhost/", "://127.0.0.1/")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_store() -> (OAuthStore, TempDir) {
        let dir = TempDir::new().unwrap();
        let store = OAuthStore::load(dir.path());
        (store, dir)
    }

    /// Helper: generate a PKCE verifier/challenge pair.
    fn pkce_pair(verifier: &str) -> (&str, String) {
        let mut hasher = Sha256::new();
        hasher.update(verifier.as_bytes());
        (verifier, URL_SAFE_NO_PAD.encode(hasher.finalize()))
    }

    /// Helper: register a standard test client.
    fn register_test_client(store: &OAuthStore, name: &str) -> OAuthClient {
        store.register_client(RegistrationRequest {
            client_name: name.into(),
            redirect_uris: vec!["http://127.0.0.1:3000/callback".into()],
            grant_types: vec!["authorization_code".into()],
            token_endpoint_auth_method: "none".into(),
        })
    }

    /// Helper: create an auth code for a client with a given PKCE challenge.
    fn create_code(store: &OAuthStore, client_id: &str, challenge: &str) -> String {
        store.create_authorization_code(
            client_id.into(),
            "http://127.0.0.1:3000/callback".into(),
            challenge.into(),
            vec!["mcp".into()],
            "http://127.0.0.1:9600/mcp".into(),
            "test-state".into(),
        )
    }

    // =====================================================================
    // Client Registration
    // =====================================================================

    #[test]
    fn register_and_lookup_client() {
        let (store, _dir) = test_store();
        let client = register_test_client(&store, "Test Client");

        assert_eq!(client.client_name, "Test Client");
        assert!(!client.approved);

        let found = store.get_client(&client.client_id).unwrap();
        assert_eq!(found.client_id, client.client_id);
    }

    #[test]
    fn approved_client_is_idempotent() {
        let (store, _dir) = test_store();
        let client = register_test_client(&store, "Claude Code");
        store.approve_client(&client.client_id);

        // Second registration with same name returns existing client
        let client2 = store.register_client(RegistrationRequest {
            client_name: "Claude Code".into(),
            redirect_uris: vec!["http://127.0.0.1:9999/callback".into()],
            grant_types: vec!["authorization_code".into()],
            token_endpoint_auth_method: "none".into(),
        });

        assert_eq!(client.client_id, client2.client_id);
    }

    #[test]
    fn unapproved_client_is_idempotent() {
        let (store, _dir) = test_store();
        let c1 = register_test_client(&store, "Pending App");
        // Same name → returns existing client regardless of approval state
        let c2 = register_test_client(&store, "Pending App");
        assert_eq!(c1.client_id, c2.client_id);
    }

    #[test]
    fn get_nonexistent_client_returns_none() {
        let (store, _dir) = test_store();
        assert!(store.get_client("does-not-exist").is_none());
    }

    #[test]
    fn is_client_approved_default_false() {
        let (store, _dir) = test_store();
        let client = register_test_client(&store, "New App");
        assert!(!store.is_client_approved(&client.client_id));
    }

    #[test]
    fn approve_then_check() {
        let (store, _dir) = test_store();
        let client = register_test_client(&store, "Approvable");
        store.approve_client(&client.client_id);
        assert!(store.is_client_approved(&client.client_id));
    }

    #[test]
    fn approve_nonexistent_client_is_noop() {
        let (store, _dir) = test_store();
        // Should not panic
        store.approve_client("ghost-id");
        assert!(!store.is_client_approved("ghost-id"));
    }

    #[test]
    fn list_clients_returns_all() {
        let (store, _dir) = test_store();
        register_test_client(&store, "App A");
        register_test_client(&store, "App B");
        register_test_client(&store, "App C");
        assert_eq!(store.list_clients().len(), 3);
    }

    #[test]
    fn multiple_redirect_uris_stored() {
        let (store, _dir) = test_store();
        let client = store.register_client(RegistrationRequest {
            client_name: "Multi-URI".into(),
            redirect_uris: vec![
                "http://127.0.0.1:3000/callback".into(),
                "http://127.0.0.1:8080/oauth/redirect".into(),
            ],
            grant_types: vec!["authorization_code".into()],
            token_endpoint_auth_method: "none".into(),
        });
        let found = store.get_client(&client.client_id).unwrap();
        assert_eq!(found.redirect_uris.len(), 2);
    }

    // =====================================================================
    // PKCE Verification
    // =====================================================================

    #[test]
    fn pkce_s256_valid() {
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        let (_, challenge) = pkce_pair(verifier);
        assert!(verify_pkce(verifier, &challenge));
    }

    #[test]
    fn pkce_wrong_verifier_rejected() {
        let (_, challenge) = pkce_pair("correct-verifier-at-least-43-characters-long-for-test");
        assert!(!verify_pkce("wrong-verifier-at-least-43-characters", &challenge));
    }

    #[test]
    fn pkce_empty_verifier_rejected() {
        let (_, challenge) = pkce_pair("real-verifier-that-is-long-enough-for-oauth-pkce");
        assert!(!verify_pkce("", &challenge));
    }

    #[test]
    fn pkce_challenge_not_plain_verifier() {
        // Verify the challenge is NOT the plain verifier (it must be hashed)
        let verifier = "my-verifier-string-at-least-43-chars-for-oauth-pkce-spec";
        assert!(!verify_pkce(verifier, verifier));
    }

    // =====================================================================
    // Authorization Code Exchange
    // =====================================================================

    #[test]
    fn full_authorization_code_flow() {
        let (store, _dir) = test_store();
        let client = register_test_client(&store, "Test");
        let (verifier, challenge) = pkce_pair("test-verifier-that-is-at-least-43-characters-long-for-oauth");
        let code = create_code(&store, &client.client_id, &challenge);

        let (access, refresh) = store
            .exchange_code(&code, verifier, &client.client_id, "http://127.0.0.1:3000/callback")
            .unwrap();

        assert!(!access.token.is_empty());
        assert_eq!(access.client_id, client.client_id);
        assert!(store.validate_access_token(&access.token).is_some());

        // Code is single-use
        assert!(store
            .exchange_code(&code, verifier, &client.client_id, "http://127.0.0.1:3000/callback")
            .is_err());

        // Refresh works
        let refresh = refresh.expect("full auth should include refresh token");
        let (access2, _) = store.refresh(&refresh.token, &client.client_id).unwrap();
        assert!(store.validate_access_token(&access2.token).is_some());

        // Old refresh token invalidated
        assert!(store.refresh(&refresh.token, &client.client_id).is_err());
    }

    #[test]
    fn exchange_code_wrong_client_id() {
        let (store, _dir) = test_store();
        let client = register_test_client(&store, "Real Client");
        let _imposter = register_test_client(&store, "Imposter");
        let (verifier, challenge) = pkce_pair("a]verifier-string-long-enough-to-satisfy-pkce-min-length");
        let code = create_code(&store, &client.client_id, &challenge);

        let result = store.exchange_code(
            &code,
            verifier,
            &_imposter.client_id,
            "http://127.0.0.1:3000/callback",
        );
        assert_eq!(result.unwrap_err(), "invalid_grant");
    }

    #[test]
    fn exchange_code_wrong_redirect_uri() {
        let (store, _dir) = test_store();
        let client = register_test_client(&store, "URI Test");
        let (verifier, challenge) = pkce_pair("verifier-for-redirect-uri-mismatch-test-at-least-43-chars");
        let code = create_code(&store, &client.client_id, &challenge);

        let result = store.exchange_code(
            &code,
            verifier,
            &client.client_id,
            "http://127.0.0.1:9999/evil-callback",
        );
        assert_eq!(result.unwrap_err(), "invalid_grant");
    }

    #[test]
    fn exchange_code_wrong_pkce_verifier() {
        let (store, _dir) = test_store();
        let client = register_test_client(&store, "PKCE Fail");
        let (_, challenge) = pkce_pair("the-real-verifier-that-nobody-else-knows-at-least-43-ch");
        let code = create_code(&store, &client.client_id, &challenge);

        let result = store.exchange_code(
            &code,
            "completely-wrong-verifier-that-is-long-enough-though",
            &client.client_id,
            "http://127.0.0.1:3000/callback",
        );
        assert_eq!(result.unwrap_err(), "invalid_grant");
    }

    #[test]
    fn exchange_code_replay_attack() {
        let (store, _dir) = test_store();
        let client = register_test_client(&store, "Replay Target");
        let (verifier, challenge) = pkce_pair("verifier-for-replay-attack-test-must-be-43-chars-long");
        let code = create_code(&store, &client.client_id, &challenge);

        // First exchange succeeds
        let result = store.exchange_code(&code, verifier, &client.client_id, "http://127.0.0.1:3000/callback");
        assert!(result.is_ok());

        // Replay of the same code must fail
        let replay = store.exchange_code(&code, verifier, &client.client_id, "http://127.0.0.1:3000/callback");
        assert_eq!(replay.unwrap_err(), "invalid_grant");
    }

    #[test]
    fn exchange_nonexistent_code() {
        let (store, _dir) = test_store();
        let client = register_test_client(&store, "No Code");
        let result = store.exchange_code(
            "made-up-code",
            "irrelevant-verifier",
            &client.client_id,
            "http://127.0.0.1:3000/callback",
        );
        assert_eq!(result.unwrap_err(), "invalid_grant");
    }

    #[test]
    fn exchange_code_localhost_redirect_normalization() {
        // Client registers with localhost, code created with localhost,
        // but exchange uses 127.0.0.1 — should still match.
        let (store, _dir) = test_store();
        let client = store.register_client(RegistrationRequest {
            client_name: "Localhost Client".into(),
            redirect_uris: vec!["http://localhost:3000/callback".into()],
            grant_types: vec!["authorization_code".into()],
            token_endpoint_auth_method: "none".into(),
        });
        let (verifier, challenge) = pkce_pair("localhost-normalization-test-verifier-at-least-43-chars");
        let code = store.create_authorization_code(
            client.client_id.clone(),
            "http://localhost:3000/callback".into(),
            challenge,
            vec!["mcp".into()],
            "http://127.0.0.1:9600/mcp".into(),
            "state".into(),
        );

        // Exchange with 127.0.0.1 variant
        let result = store.exchange_code(
            &code,
            verifier,
            &client.client_id,
            "http://127.0.0.1:3000/callback",
        );
        assert!(result.is_ok());
    }

    #[test]
    fn multiple_auth_codes_for_same_client() {
        let (store, _dir) = test_store();
        let client = register_test_client(&store, "Multi-Code");
        let (v1, c1) = pkce_pair("first-verifier-that-is-at-least-43-characters-long-aaa");
        let (v2, c2) = pkce_pair("second-verifier-that-is-at-least-43-characters-long-bb");

        let code1 = create_code(&store, &client.client_id, &c1);
        let code2 = create_code(&store, &client.client_id, &c2);

        // Both codes should be independently valid
        assert!(store.exchange_code(&code1, v1, &client.client_id, "http://127.0.0.1:3000/callback").is_ok());
        assert!(store.exchange_code(&code2, v2, &client.client_id, "http://127.0.0.1:3000/callback").is_ok());
    }

    // =====================================================================
    // Access Tokens
    // =====================================================================

    #[test]
    fn access_token_contains_correct_metadata() {
        let (store, _dir) = test_store();
        let client = register_test_client(&store, "Meta Client");
        let (verifier, challenge) = pkce_pair("verifier-for-metadata-test-must-be-at-least-43-chars-l");
        let code = create_code(&store, &client.client_id, &challenge);

        let (access, _) = store
            .exchange_code(&code, verifier, &client.client_id, "http://127.0.0.1:3000/callback")
            .unwrap();

        assert_eq!(access.client_id, client.client_id);
        assert_eq!(access.client_name, "Meta Client");
        assert_eq!(access.scopes, vec!["mcp"]);
        assert_eq!(access.resource, "http://127.0.0.1:9600/mcp");
    }

    #[test]
    fn nonexistent_access_token_returns_none() {
        let (store, _dir) = test_store();
        assert!(store.validate_access_token("not-a-real-token").is_none());
    }

    #[test]
    fn each_exchange_produces_unique_tokens() {
        let (store, _dir) = test_store();
        let client = register_test_client(&store, "Unique Tokens");

        let (v1, c1) = pkce_pair("first-unique-verifier-at-least-43-characters-long-aaa");
        let (v2, c2) = pkce_pair("second-unique-verifier-at-least-43-characters-long-bb");

        let code1 = create_code(&store, &client.client_id, &c1);
        let code2 = create_code(&store, &client.client_id, &c2);

        let (a1, r1) = store.exchange_code(&code1, v1, &client.client_id, "http://127.0.0.1:3000/callback").unwrap();
        let (a2, r2) = store.exchange_code(&code2, v2, &client.client_id, "http://127.0.0.1:3000/callback").unwrap();

        assert_ne!(a1.token, a2.token);
        assert_ne!(r1.unwrap().token, r2.unwrap().token);
    }

    // =====================================================================
    // Refresh Tokens
    // =====================================================================

    #[test]
    fn refresh_rotates_token() {
        let (store, _dir) = test_store();
        let client = register_test_client(&store, "Rotate Test");
        let (verifier, challenge) = pkce_pair("verifier-for-rotation-test-must-be-at-least-43-chars-l");
        let code = create_code(&store, &client.client_id, &challenge);

        let (_, refresh1) = store
            .exchange_code(&code, verifier, &client.client_id, "http://127.0.0.1:3000/callback")
            .unwrap();
        let refresh1 = refresh1.unwrap();

        let (new_access, refresh2) = store.refresh(&refresh1.token, &client.client_id).unwrap();

        // New access token is valid
        assert!(store.validate_access_token(&new_access.token).is_some());
        // New refresh token is different from old one
        assert_ne!(refresh1.token, refresh2.token);
        // Old refresh token is dead
        assert!(store.refresh(&refresh1.token, &client.client_id).is_err());
        // New refresh token works
        assert!(store.refresh(&refresh2.token, &client.client_id).is_ok());
    }

    #[test]
    fn refresh_wrong_client_id() {
        let (store, _dir) = test_store();
        let client = register_test_client(&store, "Owner");
        let thief = register_test_client(&store, "Thief");
        let (verifier, challenge) = pkce_pair("verifier-for-wrong-client-refresh-test-43-chars-long!");
        let code = create_code(&store, &client.client_id, &challenge);

        let (_, refresh) = store
            .exchange_code(&code, verifier, &client.client_id, "http://127.0.0.1:3000/callback")
            .unwrap();
        let refresh = refresh.unwrap();

        // Another client trying to use the refresh token
        let result = store.refresh(&refresh.token, &thief.client_id);
        assert_eq!(result.unwrap_err(), "invalid_grant");
    }

    #[test]
    fn refresh_nonexistent_token() {
        let (store, _dir) = test_store();
        let client = register_test_client(&store, "Ghost Refresh");
        let result = store.refresh("not-a-real-refresh-token", &client.client_id);
        assert_eq!(result.unwrap_err(), "invalid_grant");
    }

    #[test]
    fn refresh_preserves_scopes_and_resource() {
        let (store, _dir) = test_store();
        let client = register_test_client(&store, "Scope Check");
        let (verifier, challenge) = pkce_pair("verifier-for-scope-preservation-test-at-least-43-chars");

        // Create code with specific scopes
        let code = store.create_authorization_code(
            client.client_id.clone(),
            "http://127.0.0.1:3000/callback".into(),
            challenge,
            vec!["mcp".into(), "read".into()],
            "http://127.0.0.1:9600/mcp".into(),
            "state".into(),
        );

        let (access1, refresh) = store
            .exchange_code(&code, verifier, &client.client_id, "http://127.0.0.1:3000/callback")
            .unwrap();
        let refresh = refresh.unwrap();

        let (access2, _) = store.refresh(&refresh.token, &client.client_id).unwrap();

        // Scopes and resource should carry through the refresh
        assert_eq!(access1.scopes, access2.scopes);
        assert_eq!(access1.resource, access2.resource);
        assert_eq!(access2.scopes, vec!["mcp", "read"]);
    }

    // =====================================================================
    // Revocation
    // =====================================================================

    #[test]
    fn revoke_invalidates_access_tokens() {
        let (store, _dir) = test_store();
        let client = register_test_client(&store, "Revoke Me");
        let (verifier, challenge) = pkce_pair("verifier-for-revocation-test-must-be-at-least-43-chars");
        let code = create_code(&store, &client.client_id, &challenge);

        let (access, _) = store
            .exchange_code(&code, verifier, &client.client_id, "http://127.0.0.1:3000/callback")
            .unwrap();

        assert!(store.validate_access_token(&access.token).is_some());
        store.revoke_client(&client.client_id);
        assert!(store.validate_access_token(&access.token).is_none());
    }

    #[test]
    fn revoke_invalidates_refresh_tokens() {
        let (store, _dir) = test_store();
        let client = register_test_client(&store, "Revoke Refresh");
        let (verifier, challenge) = pkce_pair("verifier-for-refresh-revocation-test-at-least-43-chars");
        let code = create_code(&store, &client.client_id, &challenge);

        let (_, refresh) = store
            .exchange_code(&code, verifier, &client.client_id, "http://127.0.0.1:3000/callback")
            .unwrap();
        let refresh = refresh.unwrap();

        store.revoke_client(&client.client_id);
        assert!(store.refresh(&refresh.token, &client.client_id).is_err());
    }

    #[test]
    fn revoke_removes_client() {
        let (store, _dir) = test_store();
        let client = register_test_client(&store, "Remove Me");
        store.approve_client(&client.client_id);
        assert!(store.get_client(&client.client_id).is_some());

        store.revoke_client(&client.client_id);
        assert!(store.get_client(&client.client_id).is_none());
    }

    #[test]
    fn revoke_does_not_affect_other_clients() {
        let (store, _dir) = test_store();
        let alice = register_test_client(&store, "Alice");
        let bob = register_test_client(&store, "Bob");

        let (v_a, c_a) = pkce_pair("alice-verifier-string-that-is-at-least-43-characters!!");
        let (v_b, c_b) = pkce_pair("bob-verifier-string-that-is-at-least-43-characters!!!");

        let code_a = create_code(&store, &alice.client_id, &c_a);
        let code_b = create_code(&store, &bob.client_id, &c_b);

        let (access_a, _) = store.exchange_code(&code_a, v_a, &alice.client_id, "http://127.0.0.1:3000/callback").unwrap();
        let (access_b, _) = store.exchange_code(&code_b, v_b, &bob.client_id, "http://127.0.0.1:3000/callback").unwrap();

        // Revoke Alice
        store.revoke_client(&alice.client_id);

        // Alice's token dead, Bob's still valid
        assert!(store.validate_access_token(&access_a.token).is_none());
        assert!(store.validate_access_token(&access_b.token).is_some());
    }

    // =====================================================================
    // Persistence
    // =====================================================================

    #[test]
    fn persistence_clients_roundtrip() {
        let dir = TempDir::new().unwrap();

        let store = OAuthStore::load(dir.path());
        let client = register_test_client(&store, "Persistent Client");
        store.approve_client(&client.client_id);
        drop(store);

        let store2 = OAuthStore::load(dir.path());
        let found = store2.get_client(&client.client_id).unwrap();
        assert_eq!(found.client_name, "Persistent Client");
        assert!(found.approved);
    }

    #[test]
    fn persistence_refresh_tokens_survive_reload() {
        let dir = TempDir::new().unwrap();

        let (client_id, refresh_token) = {
            let store = OAuthStore::load(dir.path());
            let client = register_test_client(&store, "Refresh Persist");
            let (verifier, challenge) = pkce_pair("verifier-for-persistence-test-at-least-43-characters!!");
            let code = create_code(&store, &client.client_id, &challenge);
            let (_, refresh) = store
                .exchange_code(&code, verifier, &client.client_id, "http://127.0.0.1:3000/callback")
                .unwrap();
            (client.client_id.clone(), refresh.unwrap().token.clone())
        };
        // store dropped — only disk state remains

        let store2 = OAuthStore::load(dir.path());
        // Refresh token should still work after reload
        let result = store2.refresh(&refresh_token, &client_id);
        assert!(result.is_ok());
    }

    #[test]
    fn persistence_expired_refresh_tokens_pruned_on_load() {
        let dir = TempDir::new().unwrap();

        // Write a refresh token file with an already-expired token
        let mut tokens = HashMap::new();
        tokens.insert(
            "expired-token".to_string(),
            RefreshToken {
                token: "expired-token".into(),
                client_id: "some-client".into(),
                scopes: vec!["mcp".into()],
                resource: "http://127.0.0.1:9600/mcp".into(),
                expires_at: Utc::now() - chrono::Duration::days(1),
            },
        );
        let json = serde_json::to_string_pretty(&tokens).unwrap();
        std::fs::write(dir.path().join("oauth_refresh.json"), json).unwrap();

        let store = OAuthStore::load(dir.path());
        // Expired token should have been pruned during load
        let result = store.refresh("expired-token", "some-client");
        assert!(result.is_err());
    }

    #[test]
    fn persistence_multiple_clients_survive_reload() {
        let dir = TempDir::new().unwrap();

        let store = OAuthStore::load(dir.path());
        let a = register_test_client(&store, "Client A");
        let b = register_test_client(&store, "Client B");
        store.approve_client(&a.client_id);
        // b stays unapproved
        drop(store);

        let store2 = OAuthStore::load(dir.path());
        assert!(store2.is_client_approved(&a.client_id));
        assert!(!store2.is_client_approved(&b.client_id));
        assert_eq!(store2.list_clients().len(), 2);
    }

    #[test]
    fn persistence_revoked_client_removed() {
        let dir = TempDir::new().unwrap();

        let store = OAuthStore::load(dir.path());
        let client = register_test_client(&store, "Will Revoke");
        store.approve_client(&client.client_id);
        store.revoke_client(&client.client_id);
        drop(store);

        let store2 = OAuthStore::load(dir.path());
        // Client is fully removed, not just unapproved
        assert!(store2.get_client(&client.client_id).is_none());
    }

    // =====================================================================
    // Localhost Normalization
    // =====================================================================

    #[test]
    fn localhost_to_127() {
        assert_eq!(
            normalize_redirect_uri("http://localhost:3000/callback".into()),
            "http://127.0.0.1:3000/callback"
        );
    }

    #[test]
    fn already_127_unchanged() {
        assert_eq!(
            normalize_redirect_uri("http://127.0.0.1:3000/callback".into()),
            "http://127.0.0.1:3000/callback"
        );
    }

    #[test]
    fn localhost_path_without_port() {
        assert_eq!(
            normalize_redirect_uri("http://localhost/callback".into()),
            "http://127.0.0.1/callback"
        );
    }

    #[test]
    fn non_localhost_uri_unchanged() {
        let uri = "https://example.com:8080/callback".to_string();
        assert_eq!(normalize_redirect_uri(uri.clone()), uri);
    }

    // =====================================================================
    // Edge Cases & Security
    // =====================================================================

    #[test]
    fn empty_data_dir_loads_cleanly() {
        let dir = TempDir::new().unwrap();
        let store = OAuthStore::load(dir.path());
        assert_eq!(store.list_clients().len(), 0);
        assert!(store.validate_access_token("anything").is_none());
    }

    #[test]
    fn corrupt_clients_file_loads_empty() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("oauth_clients.json"), "not json!!!").unwrap();
        let store = OAuthStore::load(dir.path());
        assert_eq!(store.list_clients().len(), 0);
    }

    #[test]
    fn corrupt_refresh_file_loads_empty() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("oauth_refresh.json"), "{{{bad").unwrap();
        let store = OAuthStore::load(dir.path());
        // Should not crash, just start with empty refresh tokens
        assert!(store.refresh("any", "any").is_err());
    }

    #[test]
    fn concurrent_registrations_dont_overwrite() {
        let (store, _dir) = test_store();
        // Register several clients rapidly
        let clients: Vec<_> = (0..10)
            .map(|i| register_test_client(&store, &format!("Client {}", i)))
            .collect();

        // All should have unique IDs
        let ids: std::collections::HashSet<_> = clients.iter().map(|c| c.client_id.clone()).collect();
        assert_eq!(ids.len(), 10);
        assert_eq!(store.list_clients().len(), 10);
    }

    #[test]
    fn allow_once_issues_no_refresh_token() {
        let (store, _dir) = test_store();
        let client = register_test_client(&store, "Once Only");
        let (verifier, challenge) = pkce_pair("verifier-for-allow-once-no-refresh-test-at-least-43-ch");
        let code = store.create_authorization_code_once(
            client.client_id.clone(),
            "http://127.0.0.1:3000/callback".into(),
            challenge,
            vec!["mcp".into()],
            "http://127.0.0.1:9600/mcp".into(),
            "state".into(),
        );

        let (access, refresh) = store
            .exchange_code(&code, verifier, &client.client_id, "http://127.0.0.1:3000/callback")
            .unwrap();

        // Access token should work
        assert!(store.validate_access_token(&access.token).is_some());
        // No refresh token issued
        assert!(refresh.is_none());
    }

    #[test]
    fn allow_always_issues_refresh_token() {
        let (store, _dir) = test_store();
        let client = register_test_client(&store, "Always");
        let (verifier, challenge) = pkce_pair("verifier-for-allow-always-refresh-test-at-least-43-chs");
        let code = create_code(&store, &client.client_id, &challenge);

        let (_, refresh) = store
            .exchange_code(&code, verifier, &client.client_id, "http://127.0.0.1:3000/callback")
            .unwrap();

        assert!(refresh.is_some());
    }

    #[test]
    fn expired_auth_code_rejected() {
        let (store, _dir) = test_store();
        let client = register_test_client(&store, "Expiry Test");
        let (verifier, challenge) = pkce_pair("verifier-for-expired-auth-code-test-at-least-43-chars!");
        let code = create_code(&store, &client.client_id, &challenge);

        store.expire_auth_code(&code);

        let result = store.exchange_code(&code, verifier, &client.client_id, "http://127.0.0.1:3000/callback");
        assert_eq!(result.unwrap_err(), "invalid_grant");
    }

    #[test]
    fn expired_access_token_rejected() {
        let (store, _dir) = test_store();
        let client = register_test_client(&store, "Token Expiry");
        let (verifier, challenge) = pkce_pair("verifier-for-expired-access-token-test-at-least-43-ch!");
        let code = create_code(&store, &client.client_id, &challenge);

        let (access, _) = store
            .exchange_code(&code, verifier, &client.client_id, "http://127.0.0.1:3000/callback")
            .unwrap();

        assert!(store.validate_access_token(&access.token).is_some());
        store.expire_access_token(&access.token);
        assert!(store.validate_access_token(&access.token).is_none());
    }

    #[test]
    fn auth_code_cleanup_on_create() {
        let (store, _dir) = test_store();
        let client = register_test_client(&store, "Cleanup Test");
        let (v1, c1) = pkce_pair("first-code-verifier-at-least-43-characters-long-aaaaa");

        let code1 = create_code(&store, &client.client_id, &c1);

        // Use the first code (marks it as used)
        store.exchange_code(&code1, v1, &client.client_id, "http://127.0.0.1:3000/callback").unwrap();

        // Creating a new code should trigger cleanup of used codes
        let (_, c2) = pkce_pair("second-code-verifier-at-least-43-characters-long-bbbb");
        let _code2 = create_code(&store, &client.client_id, &c2);

        // Used code should definitely be gone (cleaned up)
        assert!(store.exchange_code(&code1, v1, &client.client_id, "http://127.0.0.1:3000/callback").is_err());
    }
}
