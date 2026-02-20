//! Plugin auth lifecycle service.
//!
//! Encapsulates all OAuth operations for plugin lifecycle management.
//! PluginManager delegates to this instead of reaching into OAuthStore directly.

use std::sync::Arc;

use crate::permissions::rar;
use crate::permissions::service::PermissionService;

use super::store::OAuthStore;

pub struct PluginAuthService {
    oauth_store: Arc<OAuthStore>,
    permissions: Arc<dyn PermissionService>,
}

impl PluginAuthService {
    pub fn new(oauth_store: Arc<OAuthStore>, permissions: Arc<dyn PermissionService>) -> Self {
        Self {
            oauth_store,
            permissions,
        }
    }

    /// Register a new plugin OAuth client. Returns `(client_id, plaintext_secret)`.
    pub fn register(&self, plugin_id: &str, plugin_name: &str) -> (String, String) {
        let (client, secret) = self.oauth_store.register_plugin_client(plugin_id, plugin_name);
        log::info!(
            "auth:lifecycle plugin={} action=register client_id={}",
            plugin_id,
            client.client_id
        );
        (client.client_id, secret)
    }

    /// Prepare for plugin start: rotate secret, revoke old tokens, recompute
    /// authorization_details. Handles missing-client fallback (pre-OAuth installs).
    ///
    /// Returns `(client_id, new_secret)`.
    pub fn prepare_start(
        &self,
        plugin_id: &str,
        plugin_name: &str,
        oauth_client_id: &str,
    ) -> (String, String) {
        let (client_id, new_secret) = match self.oauth_store.rotate_plugin_secret(oauth_client_id) {
            Some(secret) => {
                log::info!(
                    "auth:lifecycle plugin={} action=rotate_secret client_id={}",
                    plugin_id,
                    oauth_client_id
                );
                (oauth_client_id.to_string(), secret)
            }
            None => {
                // OAuth client doesn't exist (pre-OAuth install or data loss) — register fresh
                log::warn!(
                    "auth:lifecycle plugin={} action=fallback_register reason=missing_client old_client_id={}",
                    plugin_id,
                    oauth_client_id
                );
                let (client, secret) =
                    self.oauth_store.register_plugin_client(plugin_id, plugin_name);
                (client.client_id, secret)
            }
        };

        // Revoke any tokens from the previous run
        self.oauth_store.revoke_plugin_tokens(oauth_client_id);
        log::info!(
            "auth:lifecycle plugin={} action=revoke_tokens client_id={}",
            plugin_id,
            oauth_client_id
        );

        // Re-compute authorization_details from current permissions
        self.refresh_auth_details(plugin_id, &client_id);

        (client_id, new_secret)
    }

    /// Plugin stopped — revoke all tokens.
    pub fn on_stop(&self, plugin_id: &str, oauth_client_id: &str) {
        self.oauth_store.revoke_plugin_tokens(oauth_client_id);
        log::info!(
            "auth:lifecycle plugin={} action=stop_revoke client_id={}",
            plugin_id,
            oauth_client_id
        );
    }

    /// Plugin removed — delete client entirely.
    pub fn on_remove(&self, plugin_id: &str, oauth_client_id: &str) {
        self.oauth_store.remove_plugin_client(oauth_client_id);
        log::info!(
            "auth:lifecycle plugin={} action=remove_client client_id={}",
            plugin_id,
            oauth_client_id
        );
    }

    /// Recompute authorization_details from current permissions.
    /// Called after permission changes at runtime.
    pub fn refresh_auth_details(&self, plugin_id: &str, oauth_client_id: &str) {
        let grants = self.permissions.get_grants(plugin_id);
        let details = rar::build_authorization_details(&grants);
        self.oauth_store
            .set_plugin_auth_details(oauth_client_id, details);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::permissions::{DefaultPermissionService, Permission, PermissionStore};

    fn test_service(dir: &std::path::Path) -> PluginAuthService {
        let oauth_store = Arc::new(OAuthStore::load(dir));
        let perm_store = PermissionStore::load(dir).unwrap_or_default();
        let permissions: Arc<dyn PermissionService> =
            Arc::new(DefaultPermissionService::new(perm_store));
        PluginAuthService::new(oauth_store, permissions)
    }

    #[test]
    fn register_creates_client() {
        let dir = tempfile::tempdir().unwrap();
        let svc = test_service(dir.path());

        let (client_id, secret) = svc.register("com.test.plugin", "Test Plugin");
        assert!(!client_id.is_empty());
        assert!(!secret.is_empty());

        // Client should exist in the store
        assert!(svc.oauth_store.get_client(&client_id).is_some());
    }

    #[test]
    fn prepare_start_rotates_secret() {
        let dir = tempfile::tempdir().unwrap();
        let svc = test_service(dir.path());

        let (client_id, secret1) = svc.register("com.test.plugin", "Test Plugin");
        let (returned_id, secret2) = svc.prepare_start("com.test.plugin", "Test Plugin", &client_id);

        assert_eq!(returned_id, client_id);
        assert_ne!(secret1, secret2);
    }

    #[test]
    fn prepare_start_fallback_on_missing_client() {
        let dir = tempfile::tempdir().unwrap();
        let svc = test_service(dir.path());

        // No client registered — should fall back to registering
        let (client_id, secret) = svc.prepare_start("com.test.plugin", "Test Plugin", "nonexistent-id");
        assert!(!client_id.is_empty());
        assert!(!secret.is_empty());
        assert!(svc.oauth_store.get_client(&client_id).is_some());
    }

    #[test]
    fn on_stop_revokes_tokens() {
        let dir = tempfile::tempdir().unwrap();
        let svc = test_service(dir.path());

        let (client_id, secret) = svc.register("com.test.plugin", "Test Plugin");
        let (access, _) = svc
            .oauth_store
            .issue_client_credentials(&client_id, &secret, "".into(), vec![])
            .unwrap();

        assert!(svc.oauth_store.validate_access_token(&access.token).is_some());
        svc.on_stop("com.test.plugin", &client_id);
        assert!(svc.oauth_store.validate_access_token(&access.token).is_none());

        // Client should still exist
        assert!(svc.oauth_store.get_client(&client_id).is_some());
    }

    #[test]
    fn on_remove_deletes_client() {
        let dir = tempfile::tempdir().unwrap();
        let svc = test_service(dir.path());

        let (client_id, _) = svc.register("com.test.plugin", "Test Plugin");
        assert!(svc.oauth_store.get_client(&client_id).is_some());

        svc.on_remove("com.test.plugin", &client_id);
        assert!(svc.oauth_store.get_client(&client_id).is_none());
    }

    #[test]
    fn refresh_auth_details_sets_details() {
        let dir = tempfile::tempdir().unwrap();
        let oauth_store = Arc::new(OAuthStore::load(dir.path()));
        let perm_store = PermissionStore::load(dir.path()).unwrap_or_default();
        let permissions: Arc<dyn PermissionService> =
            Arc::new(DefaultPermissionService::new(perm_store));

        // Grant a permission
        permissions
            .grant("com.test.plugin", Permission::SystemInfo, None)
            .unwrap();

        let svc = PluginAuthService::new(oauth_store.clone(), permissions);
        let (client_id, _) = svc.register("com.test.plugin", "Test Plugin");

        svc.refresh_auth_details("com.test.plugin", &client_id);

        let details = oauth_store.get_plugin_auth_details(&client_id);
        assert!(!details.is_empty());
        assert_eq!(details[0].detail_type, "nexus:system");
    }

    // =====================================================================
    // Edge cases
    // =====================================================================

    #[test]
    fn prepare_start_revokes_old_tokens() {
        let dir = tempfile::tempdir().unwrap();
        let svc = test_service(dir.path());

        let (client_id, secret) = svc.register("com.test.plugin", "Test Plugin");

        // Issue a token using the original secret
        let (access, _) = svc
            .oauth_store
            .issue_client_credentials(&client_id, &secret, "".into(), vec![])
            .unwrap();
        assert!(svc.oauth_store.validate_access_token(&access.token).is_some());

        // prepare_start should revoke that token
        let (_, _new_secret) = svc.prepare_start("com.test.plugin", "Test Plugin", &client_id);
        assert!(
            svc.oauth_store.validate_access_token(&access.token).is_none(),
            "old token should be revoked after prepare_start"
        );
    }

    #[test]
    fn prepare_start_old_secret_invalid() {
        let dir = tempfile::tempdir().unwrap();
        let svc = test_service(dir.path());

        let (client_id, old_secret) = svc.register("com.test.plugin", "Test Plugin");
        let (_, new_secret) = svc.prepare_start("com.test.plugin", "Test Plugin", &client_id);

        // Old secret should fail
        assert!(
            svc.oauth_store
                .issue_client_credentials(&client_id, &old_secret, "".into(), vec![])
                .is_err(),
            "old secret should be rejected after rotation"
        );
        // New secret should work
        assert!(
            svc.oauth_store
                .issue_client_credentials(&client_id, &new_secret, "".into(), vec![])
                .is_ok(),
            "new secret should be accepted"
        );
    }

    #[test]
    fn prepare_start_refreshes_auth_details() {
        let dir = tempfile::tempdir().unwrap();
        let oauth_store = Arc::new(OAuthStore::load(dir.path()));
        let perm_store = PermissionStore::load(dir.path()).unwrap_or_default();
        let permissions: Arc<dyn PermissionService> =
            Arc::new(DefaultPermissionService::new(perm_store));

        permissions
            .grant("com.test.plugin", Permission::FilesystemRead, None)
            .unwrap();

        let svc = PluginAuthService::new(oauth_store.clone(), permissions);
        let (client_id, _) = svc.register("com.test.plugin", "Test Plugin");

        // Before prepare_start, no auth details
        assert!(oauth_store.get_plugin_auth_details(&client_id).is_empty());

        svc.prepare_start("com.test.plugin", "Test Plugin", &client_id);

        // After prepare_start, auth details should be populated
        let details = oauth_store.get_plugin_auth_details(&client_id);
        assert!(!details.is_empty(), "prepare_start should refresh auth details");
    }

    #[test]
    fn refresh_auth_details_nonexistent_client_is_noop() {
        let dir = tempfile::tempdir().unwrap();
        let svc = test_service(dir.path());

        // Should not panic — just sets details for a client that doesn't exist
        svc.refresh_auth_details("com.test.plugin", "nonexistent-client-id");

        // Verify it stored something (the store accepts any client_id)
        let details = svc.oauth_store.get_plugin_auth_details("nonexistent-client-id");
        // With no permissions granted, details should be empty
        assert!(details.is_empty());
    }

    #[test]
    fn on_remove_then_on_stop_does_not_panic() {
        let dir = tempfile::tempdir().unwrap();
        let svc = test_service(dir.path());

        let (client_id, _) = svc.register("com.test.plugin", "Test Plugin");
        svc.on_remove("com.test.plugin", &client_id);
        // Calling on_stop after remove shouldn't panic
        svc.on_stop("com.test.plugin", &client_id);
    }

    #[test]
    fn register_same_plugin_returns_same_client() {
        let dir = tempfile::tempdir().unwrap();
        let svc = test_service(dir.path());

        let (id1, _) = svc.register("com.test.plugin", "Test Plugin");
        let (id2, _) = svc.register("com.test.plugin", "Test Plugin");

        assert_eq!(id1, id2, "re-registering same plugin should return same client_id");
    }

    #[test]
    fn prepare_start_fallback_returns_valid_credentials() {
        let dir = tempfile::tempdir().unwrap();
        let svc = test_service(dir.path());

        // No pre-existing client — fallback registration
        let (client_id, secret) =
            svc.prepare_start("com.test.plugin", "Test Plugin", "nonexistent-id");

        // Should be able to authenticate with the returned credentials
        let result = svc
            .oauth_store
            .issue_client_credentials(&client_id, &secret, "".into(), vec![]);
        assert!(result.is_ok(), "fallback credentials should be functional");
    }
}
