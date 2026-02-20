//! Component tests for the auth lifecycle.
//!
//! These tests exercise the full stack: PluginAuthService → OAuthStore →
//! PermissionService → RAR → token validation, verifying that the pieces
//! fit together correctly across a plugin's lifecycle.

use std::sync::Arc;

use nexus_lib::oauth::store::OAuthStore;
use nexus_lib::oauth::PluginAuthService;
use nexus_lib::oauth::validation::{validate_bearer, TokenValidation};
use nexus_lib::permissions::rar;
use nexus_lib::permissions::service::{DefaultPermissionService, PermissionService};
use nexus_lib::permissions::{Permission, PermissionStore};

fn setup(dir: &std::path::Path) -> (PluginAuthService, Arc<OAuthStore>, Arc<dyn PermissionService>) {
    let oauth_store = Arc::new(OAuthStore::load(dir));
    let perm_store = PermissionStore::load(dir).unwrap_or_default();
    let permissions: Arc<dyn PermissionService> =
        Arc::new(DefaultPermissionService::new(perm_store));
    let svc = PluginAuthService::new(oauth_store.clone(), permissions.clone());
    (svc, oauth_store, permissions)
}

fn bearer_header(token: &str) -> axum::http::HeaderMap {
    let mut headers = axum::http::HeaderMap::new();
    headers.insert(
        "authorization",
        format!("Bearer {}", token).parse().unwrap(),
    );
    headers
}

// =========================================================================
// Full lifecycle: install → start → authenticate → stop → verify
// =========================================================================

#[test]
fn full_plugin_lifecycle() {
    let dir = tempfile::tempdir().unwrap();
    let (svc, oauth_store, _perms) = setup(dir.path());

    // 1. Install — register OAuth client
    let (client_id, _install_secret) = svc.register("com.test.lifecycle", "Lifecycle Plugin");

    // 2. Start — rotate secret, revoke old tokens
    let (active_id, start_secret) =
        svc.prepare_start("com.test.lifecycle", "Lifecycle Plugin", &client_id);
    assert_eq!(active_id, client_id);

    // 3. Authenticate — plugin does client_credentials
    let (access, _refresh) = oauth_store
        .issue_client_credentials(&active_id, &start_secret, "".into(), vec![])
        .unwrap();

    // Token should be valid
    let headers = bearer_header(&access.token);
    match validate_bearer(&headers, &oauth_store) {
        TokenValidation::Valid { plugin_id, .. } => {
            assert_eq!(plugin_id, Some("com.test.lifecycle".to_string()));
        }
        other => panic!("expected Valid, got {:?}", std::mem::discriminant(&other)),
    }

    // 4. Stop — tokens revoked
    svc.on_stop("com.test.lifecycle", &active_id);

    // Token should now be invalid
    match validate_bearer(&headers, &oauth_store) {
        TokenValidation::Invalid => {} // expected
        other => panic!("expected Invalid after stop, got {:?}", std::mem::discriminant(&other)),
    }

    // Client should still exist (stopped, not removed)
    assert!(oauth_store.get_client(&active_id).is_some());

    // 5. Start again — new secret
    let (active_id_2, new_secret) =
        svc.prepare_start("com.test.lifecycle", "Lifecycle Plugin", &active_id);
    assert_eq!(active_id_2, active_id, "client_id should be stable across restarts");

    // Can authenticate with new secret
    let (access_2, _) = oauth_store
        .issue_client_credentials(&active_id_2, &new_secret, "".into(), vec![])
        .unwrap();
    assert!(oauth_store.validate_access_token(&access_2.token).is_some());

    // 6. Remove — client deleted entirely
    svc.on_remove("com.test.lifecycle", &active_id);
    assert!(oauth_store.get_client(&active_id).is_none());
    assert!(oauth_store.validate_access_token(&access_2.token).is_none());
}

// =========================================================================
// Secret rotation invalidates old credentials
// =========================================================================

#[test]
fn secret_rotation_invalidates_old_secret() {
    let dir = tempfile::tempdir().unwrap();
    let (svc, oauth_store, _) = setup(dir.path());

    let (client_id, secret_v1) = svc.register("com.test.rotate", "Rotate Plugin");

    // Issue token with v1 secret
    let (token_v1, _) = oauth_store
        .issue_client_credentials(&client_id, &secret_v1, "".into(), vec![])
        .unwrap();

    // Rotate via prepare_start
    let (_, secret_v2) = svc.prepare_start("com.test.rotate", "Rotate Plugin", &client_id);

    // Old secret fails
    assert!(
        oauth_store
            .issue_client_credentials(&client_id, &secret_v1, "".into(), vec![])
            .is_err()
    );
    // Old token is revoked
    assert!(oauth_store.validate_access_token(&token_v1.token).is_none());

    // New secret works
    let (token_v2, _) = oauth_store
        .issue_client_credentials(&client_id, &secret_v2, "".into(), vec![])
        .unwrap();
    assert!(oauth_store.validate_access_token(&token_v2.token).is_some());
}

// =========================================================================
// Permission grant → RAR → token carries permission on the wire
// =========================================================================

#[test]
fn permission_grant_flows_to_token_rar() {
    let dir = tempfile::tempdir().unwrap();
    let (svc, oauth_store, perms) = setup(dir.path());

    let (client_id, _) = svc.register("com.test.rar", "RAR Plugin");

    // Grant permissions
    perms
        .grant("com.test.rar", Permission::SystemInfo, None)
        .unwrap();
    perms
        .grant("com.test.rar", Permission::FilesystemRead, Some(vec!["/home".into()]))
        .unwrap();

    // Refresh auth details to push grants into the OAuth layer
    svc.refresh_auth_details("com.test.rar", &client_id);

    // Verify the stored details
    let details = oauth_store.get_plugin_auth_details(&client_id);
    assert!(details.len() >= 2, "should have at least 2 authorization details");

    // RAR should satisfy SystemInfo
    assert!(
        rar::details_satisfy(&details, &Permission::SystemInfo),
        "token RAR should satisfy SystemInfo"
    );
    // RAR should satisfy FilesystemRead
    assert!(
        rar::details_satisfy(&details, &Permission::FilesystemRead),
        "token RAR should satisfy FilesystemRead"
    );
    // RAR should NOT satisfy an ungranted permission
    assert!(
        !rar::details_satisfy(&details, &Permission::ProcessExec),
        "token RAR should not satisfy ProcessExec"
    );
}

// =========================================================================
// Permission revoke → RAR refresh → token no longer satisfies
// =========================================================================

#[test]
fn permission_revoke_removes_from_rar() {
    let dir = tempfile::tempdir().unwrap();
    let (svc, oauth_store, perms) = setup(dir.path());

    let (client_id, _) = svc.register("com.test.revoke-rar", "Revoke Plugin");

    perms
        .grant("com.test.revoke-rar", Permission::SystemInfo, None)
        .unwrap();
    svc.refresh_auth_details("com.test.revoke-rar", &client_id);

    // Should satisfy before revoke
    let details_before = oauth_store.get_plugin_auth_details(&client_id);
    assert!(rar::details_satisfy(&details_before, &Permission::SystemInfo));

    // Revoke and refresh
    perms.revoke("com.test.revoke-rar", &Permission::SystemInfo).unwrap();
    svc.refresh_auth_details("com.test.revoke-rar", &client_id);

    // Should no longer satisfy
    let details_after = oauth_store.get_plugin_auth_details(&client_id);
    assert!(
        !rar::details_satisfy(&details_after, &Permission::SystemInfo),
        "revoked permission should not appear in RAR"
    );
}

// =========================================================================
// Deferred permission is not in RAR until activated
// =========================================================================

#[test]
fn deferred_permission_not_in_rar() {
    let dir = tempfile::tempdir().unwrap();
    let (svc, oauth_store, perms) = setup(dir.path());

    let (client_id, _) = svc.register("com.test.deferred", "Deferred Plugin");

    perms
        .defer("com.test.deferred", Permission::ProcessExec, None)
        .unwrap();
    svc.refresh_auth_details("com.test.deferred", &client_id);

    // Deferred should NOT be in RAR
    let details = oauth_store.get_plugin_auth_details(&client_id);
    assert!(
        !rar::details_satisfy(&details, &Permission::ProcessExec),
        "deferred permission should not be in RAR"
    );

    // Activate it
    perms.activate("com.test.deferred", &Permission::ProcessExec).unwrap();
    svc.refresh_auth_details("com.test.deferred", &client_id);

    // Now it should be in RAR
    let details_after = oauth_store.get_plugin_auth_details(&client_id);
    assert!(
        rar::details_satisfy(&details_after, &Permission::ProcessExec),
        "activated permission should be in RAR"
    );
}

// =========================================================================
// Multiple plugins are fully isolated
// =========================================================================

#[test]
fn plugin_isolation() {
    let dir = tempfile::tempdir().unwrap();
    let (svc, oauth_store, perms) = setup(dir.path());

    let (id_a, _) = svc.register("com.test.plugin-a", "Plugin A");
    let (id_b, _) = svc.register("com.test.plugin-b", "Plugin B");

    assert_ne!(id_a, id_b);

    // Grant different permissions
    perms.grant("com.test.plugin-a", Permission::SystemInfo, None).unwrap();
    perms.grant("com.test.plugin-b", Permission::FilesystemRead, None).unwrap();

    svc.refresh_auth_details("com.test.plugin-a", &id_a);
    svc.refresh_auth_details("com.test.plugin-b", &id_b);

    // A's token
    let (_, start_secret_a) = svc.prepare_start("com.test.plugin-a", "Plugin A", &id_a);
    let (token_a, _) = oauth_store
        .issue_client_credentials(&id_a, &start_secret_a, "".into(),
            oauth_store.get_plugin_auth_details(&id_a))
        .unwrap();

    // B's token
    let (_, start_secret_b) = svc.prepare_start("com.test.plugin-b", "Plugin B", &id_b);
    let (token_b, _) = oauth_store
        .issue_client_credentials(&id_b, &start_secret_b, "".into(),
            oauth_store.get_plugin_auth_details(&id_b))
        .unwrap();

    // Validate A's token resolves to plugin-a
    let headers_a = bearer_header(&token_a.token);
    match validate_bearer(&headers_a, &oauth_store) {
        TokenValidation::Valid { plugin_id, authorization_details, .. } => {
            assert_eq!(plugin_id, Some("com.test.plugin-a".to_string()));
            assert!(rar::details_satisfy(&authorization_details, &Permission::SystemInfo));
            assert!(!rar::details_satisfy(&authorization_details, &Permission::FilesystemRead));
        }
        _ => panic!("expected Valid for plugin-a"),
    }

    // Validate B's token resolves to plugin-b
    let headers_b = bearer_header(&token_b.token);
    match validate_bearer(&headers_b, &oauth_store) {
        TokenValidation::Valid { plugin_id, authorization_details, .. } => {
            assert_eq!(plugin_id, Some("com.test.plugin-b".to_string()));
            assert!(!rar::details_satisfy(&authorization_details, &Permission::SystemInfo));
            assert!(rar::details_satisfy(&authorization_details, &Permission::FilesystemRead));
        }
        _ => panic!("expected Valid for plugin-b"),
    }

    // Stop A — B should be unaffected
    svc.on_stop("com.test.plugin-a", &id_a);
    assert!(oauth_store.validate_access_token(&token_a.token).is_none(), "A's token should be revoked");
    assert!(oauth_store.validate_access_token(&token_b.token).is_some(), "B's token should still be valid");
}

// =========================================================================
// Fallback registration after data loss
// =========================================================================

#[test]
fn data_loss_recovery_via_prepare_start() {
    let dir = tempfile::tempdir().unwrap();
    let (svc, oauth_store, _) = setup(dir.path());

    // Simulate: plugin was installed with a client_id that no longer exists
    // (e.g., OAuth data was deleted but plugin storage still references it)
    let stale_client_id = "stale-client-from-before-data-loss";

    let (recovered_id, secret) =
        svc.prepare_start("com.test.recovery", "Recovery Plugin", stale_client_id);

    // Should have created a new client
    assert_ne!(recovered_id, stale_client_id);
    assert!(oauth_store.get_client(&recovered_id).is_some());

    // Should be able to authenticate
    let (access, _) = oauth_store
        .issue_client_credentials(&recovered_id, &secret, "".into(), vec![])
        .unwrap();
    assert!(oauth_store.validate_access_token(&access.token).is_some());
}
