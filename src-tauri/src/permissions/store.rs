use super::types::{GrantedPermission, Permission, PermissionState};
use crate::error::NexusResult;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, serde::Serialize, serde::Deserialize, Default)]
pub struct PermissionStore {
    grants: HashMap<String, Vec<GrantedPermission>>,
    #[serde(skip)]
    path: PathBuf,
}

impl PermissionStore {
    pub fn load(data_dir: &std::path::Path) -> NexusResult<Self> {
        let path = data_dir.join("permissions.json");
        if path.exists() {
            let data = std::fs::read_to_string(&path)?;
            let mut store: PermissionStore = serde_json::from_str(&data)?;
            store.path = path;

            // One-time migration: reconcile legacy `revoked_at` with the new `state` field.
            // Old JSON has `state` defaulting to Active even when `revoked_at` is set.
            let mut migrated = false;
            for grants in store.grants.values_mut() {
                for grant in grants.iter_mut() {
                    if grant.revoked_at.is_some() && grant.state == PermissionState::Active {
                        grant.state = PermissionState::Revoked;
                        migrated = true;
                    }
                }
            }
            if migrated {
                store.save()?;
                log::info!("Migrated permissions.json to three-state model");
            }

            Ok(store)
        } else {
            Ok(PermissionStore {
                grants: HashMap::new(),
                path,
            })
        }
    }

    pub fn save(&self) -> NexusResult<()> {
        let data = serde_json::to_string_pretty(self)?;
        crate::util::atomic_write(&self.path, data.as_bytes())?;
        Ok(())
    }

    pub fn grant(
        &mut self,
        plugin_id: &str,
        permission: Permission,
        approved_scopes: Option<Vec<String>>,
    ) -> NexusResult<()> {
        let entry = self.grants.entry(plugin_id.to_string()).or_default();

        if let Some(existing) = entry.iter_mut().find(|g| g.permission == permission) {
            // If Deferred or Revoked, transition to Active
            if existing.state != PermissionState::Active {
                existing.state = PermissionState::Active;
                existing.revoked_at = None;
                self.save()?;
            }
        } else {
            entry.push(GrantedPermission {
                plugin_id: plugin_id.to_string(),
                permission,
                granted_at: chrono::Utc::now(),
                approved_scopes,
                state: PermissionState::Active,
                revoked_at: None,
            });
            self.save()?;
        }
        Ok(())
    }

    /// Soft-revoke: marks the permission as revoked but preserves the grant
    /// (including approved scopes) so it can be restored later.
    pub fn revoke(&mut self, plugin_id: &str, permission: &Permission) -> NexusResult<()> {
        if let Some(entry) = self.grants.get_mut(plugin_id) {
            if let Some(grant) = entry.iter_mut().find(|g| &g.permission == permission) {
                grant.state = PermissionState::Revoked;
                grant.revoked_at = Some(chrono::Utc::now());
                self.save()?;
            }
        }
        Ok(())
    }

    /// Restore a previously revoked permission.
    pub fn unrevoke(&mut self, plugin_id: &str, permission: &Permission) -> NexusResult<()> {
        if let Some(entry) = self.grants.get_mut(plugin_id) {
            if let Some(grant) = entry.iter_mut().find(|g| &g.permission == permission) {
                grant.state = PermissionState::Active;
                grant.revoked_at = None;
                self.save()?;
            }
        }
        Ok(())
    }

    pub fn revoke_all(&mut self, plugin_id: &str) -> NexusResult<()> {
        self.grants.remove(plugin_id);
        self.save()?;
        Ok(())
    }

    /// Returns true only for Active permissions.
    pub fn has_permission(&self, plugin_id: &str, permission: &Permission) -> bool {
        self.grants
            .get(plugin_id)
            .is_some_and(|grants| {
                grants.iter().any(|g| &g.permission == permission && g.state == PermissionState::Active)
            })
    }

    /// Returns all grants (active, deferred, and revoked).
    pub fn get_grants(&self, plugin_id: &str) -> Vec<GrantedPermission> {
        self.grants.get(plugin_id).cloned().unwrap_or_default()
    }

    /// Returns the permission state for a (plugin_id, permission) pair,
    /// or None if no grant exists at all.
    pub fn get_state(&self, plugin_id: &str, permission: &Permission) -> Option<PermissionState> {
        self.grants.get(plugin_id).and_then(|grants| {
            grants
                .iter()
                .find(|g| &g.permission == permission)
                .map(|g| g.state)
        })
    }

    /// Create a grant in Deferred state. Same scope defaults as `grant()` but the
    /// permission won't be active until the user approves at first use (JIT).
    pub fn defer(
        &mut self,
        plugin_id: &str,
        permission: Permission,
        approved_scopes: Option<Vec<String>>,
    ) -> NexusResult<()> {
        let entry = self.grants.entry(plugin_id.to_string()).or_default();

        if entry.iter().any(|g| g.permission == permission) {
            // Already exists — don't create a duplicate, don't change state
            return Ok(());
        }

        entry.push(GrantedPermission {
            plugin_id: plugin_id.to_string(),
            permission,
            granted_at: chrono::Utc::now(),
            approved_scopes,
            state: PermissionState::Deferred,
            revoked_at: None,
        });
        self.save()?;
        Ok(())
    }

    /// Transition a Deferred permission to Active. Idempotent — no-op if already Active.
    pub fn activate(&mut self, plugin_id: &str, permission: &Permission) -> NexusResult<()> {
        if let Some(entry) = self.grants.get_mut(plugin_id) {
            if let Some(grant) = entry.iter_mut().find(|g| &g.permission == permission) {
                if grant.state == PermissionState::Deferred {
                    grant.state = PermissionState::Active;
                    self.save()?;
                }
            }
        }
        Ok(())
    }

    pub fn get_approved_scopes(&self, plugin_id: &str, permission: &Permission) -> Option<Vec<String>> {
        self.grants.get(plugin_id).and_then(|grants| {
            grants
                .iter()
                .find(|g| &g.permission == permission)
                .and_then(|g| g.approved_scopes.clone())
        })
    }

    /// Add a scope value to the approved_scopes list for a specific permission grant.
    ///
    /// No-op when the grant has `approved_scopes: None` (unrestricted) — adding
    /// a scope to an unrestricted grant would accidentally restrict it.
    pub fn add_approved_scope(
        &mut self,
        plugin_id: &str,
        permission: &Permission,
        scope: String,
    ) -> NexusResult<()> {
        if let Some(grants) = self.grants.get_mut(plugin_id) {
            if let Some(grant) = grants.iter_mut().find(|g| &g.permission == permission) {
                if let Some(ref mut scopes) = grant.approved_scopes {
                    if !scopes.contains(&scope) {
                        scopes.push(scope);
                        self.save()?;
                    }
                }
                // None = unrestricted — don't touch it
            }
        }
        Ok(())
    }

    /// Remove a scope value from the approved_scopes list for a specific permission grant.
    pub fn remove_approved_scope(
        &mut self,
        plugin_id: &str,
        permission: &Permission,
        scope: &str,
    ) -> NexusResult<()> {
        if let Some(grants) = self.grants.get_mut(plugin_id) {
            if let Some(grant) = grants.iter_mut().find(|g| &g.permission == permission) {
                if let Some(ref mut scopes) = grant.approved_scopes {
                    scopes.retain(|s| s != scope);
                    self.save()?;
                }
            }
        }
        Ok(())
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_store() -> (PermissionStore, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let store = PermissionStore::load(dir.path()).unwrap();
        (store, dir)
    }

    #[test]
    fn grant_and_check() {
        let (mut store, _dir) = temp_store();
        store.grant("plug-a", Permission::SystemInfo, None).unwrap();
        assert!(store.has_permission("plug-a", &Permission::SystemInfo));
        assert_eq!(store.get_state("plug-a", &Permission::SystemInfo), Some(PermissionState::Active));
    }

    #[test]
    fn missing_permission_returns_false() {
        let (store, _dir) = temp_store();
        assert!(!store.has_permission("plug-a", &Permission::SystemInfo));
        assert_eq!(store.get_state("plug-a", &Permission::SystemInfo), None);
    }

    #[test]
    fn grant_is_idempotent() {
        let (mut store, _dir) = temp_store();
        store.grant("plug-a", Permission::SystemInfo, None).unwrap();
        store.grant("plug-a", Permission::SystemInfo, None).unwrap();
        let grants = store.get_grants("plug-a");
        assert_eq!(grants.len(), 1, "duplicate grant created");
    }

    #[test]
    fn revoke_removes_permission() {
        let (mut store, _dir) = temp_store();
        store.grant("plug-a", Permission::SystemInfo, None).unwrap();
        store.revoke("plug-a", &Permission::SystemInfo).unwrap();
        assert!(!store.has_permission("plug-a", &Permission::SystemInfo));
        assert_eq!(store.get_state("plug-a", &Permission::SystemInfo), Some(PermissionState::Revoked));
    }

    #[test]
    fn revoke_nonexistent_is_noop() {
        let (mut store, _dir) = temp_store();
        store.revoke("plug-a", &Permission::SystemInfo).unwrap();
    }

    #[test]
    fn revoke_all_clears_plugin() {
        let (mut store, _dir) = temp_store();
        store.grant("plug-a", Permission::SystemInfo, None).unwrap();
        store.grant("plug-a", Permission::FilesystemRead, Some(vec![])).unwrap();
        store.revoke_all("plug-a").unwrap();
        assert!(store.get_grants("plug-a").is_empty());
    }

    #[test]
    fn plugins_are_isolated() {
        let (mut store, _dir) = temp_store();
        store.grant("plug-a", Permission::SystemInfo, None).unwrap();
        store.grant("plug-b", Permission::FilesystemRead, None).unwrap();

        assert!(store.has_permission("plug-a", &Permission::SystemInfo));
        assert!(!store.has_permission("plug-a", &Permission::FilesystemRead));
        assert!(!store.has_permission("plug-b", &Permission::SystemInfo));
        assert!(store.has_permission("plug-b", &Permission::FilesystemRead));
    }

    #[test]
    fn scopes_none_means_unrestricted() {
        let (mut store, _dir) = temp_store();
        store.grant("plug-a", Permission::FilesystemRead, None).unwrap();
        assert_eq!(store.get_approved_scopes("plug-a", &Permission::FilesystemRead), None);
    }

    #[test]
    fn scopes_empty_means_restricted() {
        let (mut store, _dir) = temp_store();
        store.grant("plug-a", Permission::FilesystemRead, Some(vec![])).unwrap();
        assert_eq!(
            store.get_approved_scopes("plug-a", &Permission::FilesystemRead),
            Some(vec![])
        );
    }

    #[test]
    fn add_scope_to_restricted() {
        let (mut store, _dir) = temp_store();
        store.grant("plug-a", Permission::FilesystemRead, Some(vec![])).unwrap();
        store.add_approved_scope("plug-a", &Permission::FilesystemRead, "/home/user".into()).unwrap();

        let scopes = store.get_approved_scopes("plug-a", &Permission::FilesystemRead);
        assert_eq!(scopes, Some(vec!["/home/user".to_string()]));
    }

    #[test]
    fn add_scope_is_idempotent() {
        let (mut store, _dir) = temp_store();
        store.grant("plug-a", Permission::FilesystemRead, Some(vec![])).unwrap();
        store.add_approved_scope("plug-a", &Permission::FilesystemRead, "/home".into()).unwrap();
        store.add_approved_scope("plug-a", &Permission::FilesystemRead, "/home".into()).unwrap();

        let scopes = store.get_approved_scopes("plug-a", &Permission::FilesystemRead).unwrap();
        assert_eq!(scopes.len(), 1);
    }

    #[test]
    fn add_scope_to_unrestricted_is_noop() {
        let (mut store, _dir) = temp_store();
        store.grant("plug-a", Permission::FilesystemRead, None).unwrap();
        store.add_approved_scope("plug-a", &Permission::FilesystemRead, "/home".into()).unwrap();

        // Should still be None (unrestricted), not Some(["/home"])
        assert_eq!(store.get_approved_scopes("plug-a", &Permission::FilesystemRead), None);
    }

    #[test]
    fn remove_scope() {
        let (mut store, _dir) = temp_store();
        store.grant("plug-a", Permission::FilesystemRead, Some(vec!["/a".into(), "/b".into()])).unwrap();
        store.remove_approved_scope("plug-a", &Permission::FilesystemRead, "/a").unwrap();

        let scopes = store.get_approved_scopes("plug-a", &Permission::FilesystemRead).unwrap();
        assert_eq!(scopes, vec!["/b".to_string()]);
    }

    #[test]
    fn extension_permissions_work() {
        let (mut store, _dir) = temp_store();
        let perm = Permission::Extension("ext:git-ops:status".into());
        store.grant("plug-a", perm.clone(), Some(vec![])).unwrap();

        assert!(store.has_permission("plug-a", &perm));
        assert!(!store.has_permission("plug-a", &Permission::Extension("ext:git-ops:commit".into())));
    }

    #[test]
    fn persistence_roundtrip() {
        let dir = tempfile::tempdir().unwrap();

        // Write
        {
            let mut store = PermissionStore::load(dir.path()).unwrap();
            store.grant("plug-a", Permission::SystemInfo, None).unwrap();
            store.grant("plug-a", Permission::FilesystemRead, Some(vec!["/foo".into()])).unwrap();
        }

        // Read back
        {
            let store = PermissionStore::load(dir.path()).unwrap();
            assert!(store.has_permission("plug-a", &Permission::SystemInfo));
            assert_eq!(
                store.get_approved_scopes("plug-a", &Permission::FilesystemRead),
                Some(vec!["/foo".to_string()])
            );
        }
    }

    // --- Three-state permission model tests ---

    #[test]
    fn defer_creates_deferred_grant() {
        let (mut store, _dir) = temp_store();
        store.defer("plug-a", Permission::SystemInfo, None).unwrap();

        assert!(!store.has_permission("plug-a", &Permission::SystemInfo));
        assert_eq!(store.get_state("plug-a", &Permission::SystemInfo), Some(PermissionState::Deferred));
        assert_eq!(store.get_grants("plug-a").len(), 1);
    }

    #[test]
    fn defer_is_idempotent() {
        let (mut store, _dir) = temp_store();
        store.defer("plug-a", Permission::SystemInfo, None).unwrap();
        store.defer("plug-a", Permission::SystemInfo, None).unwrap();
        assert_eq!(store.get_grants("plug-a").len(), 1);
    }

    #[test]
    fn defer_does_not_overwrite_existing() {
        let (mut store, _dir) = temp_store();
        store.grant("plug-a", Permission::SystemInfo, None).unwrap();
        store.defer("plug-a", Permission::SystemInfo, None).unwrap();
        // Should still be Active — defer doesn't downgrade
        assert_eq!(store.get_state("plug-a", &Permission::SystemInfo), Some(PermissionState::Active));
    }

    #[test]
    fn activate_transitions_deferred_to_active() {
        let (mut store, _dir) = temp_store();
        store.defer("plug-a", Permission::SystemInfo, None).unwrap();
        assert!(!store.has_permission("plug-a", &Permission::SystemInfo));

        store.activate("plug-a", &Permission::SystemInfo).unwrap();
        assert!(store.has_permission("plug-a", &Permission::SystemInfo));
        assert_eq!(store.get_state("plug-a", &Permission::SystemInfo), Some(PermissionState::Active));
    }

    #[test]
    fn activate_is_idempotent_on_active() {
        let (mut store, _dir) = temp_store();
        store.grant("plug-a", Permission::SystemInfo, None).unwrap();
        store.activate("plug-a", &Permission::SystemInfo).unwrap();
        assert_eq!(store.get_state("plug-a", &Permission::SystemInfo), Some(PermissionState::Active));
    }

    #[test]
    fn activate_does_not_affect_revoked() {
        let (mut store, _dir) = temp_store();
        store.grant("plug-a", Permission::SystemInfo, None).unwrap();
        store.revoke("plug-a", &Permission::SystemInfo).unwrap();
        store.activate("plug-a", &Permission::SystemInfo).unwrap();
        // activate only works on Deferred, not Revoked
        assert_eq!(store.get_state("plug-a", &Permission::SystemInfo), Some(PermissionState::Revoked));
    }

    #[test]
    fn deferred_to_revoke() {
        let (mut store, _dir) = temp_store();
        store.defer("plug-a", Permission::SystemInfo, None).unwrap();
        store.revoke("plug-a", &Permission::SystemInfo).unwrap();
        assert_eq!(store.get_state("plug-a", &Permission::SystemInfo), Some(PermissionState::Revoked));
        assert!(!store.has_permission("plug-a", &Permission::SystemInfo));
    }

    #[test]
    fn grant_restores_deferred() {
        let (mut store, _dir) = temp_store();
        store.defer("plug-a", Permission::SystemInfo, None).unwrap();
        store.grant("plug-a", Permission::SystemInfo, None).unwrap();
        assert_eq!(store.get_state("plug-a", &Permission::SystemInfo), Some(PermissionState::Active));
    }

    #[test]
    fn grant_restores_revoked() {
        let (mut store, _dir) = temp_store();
        store.grant("plug-a", Permission::SystemInfo, None).unwrap();
        store.revoke("plug-a", &Permission::SystemInfo).unwrap();
        store.grant("plug-a", Permission::SystemInfo, None).unwrap();
        assert_eq!(store.get_state("plug-a", &Permission::SystemInfo), Some(PermissionState::Active));
    }

    #[test]
    fn full_lifecycle_grant_revoke_unrevoke() {
        let (mut store, _dir) = temp_store();
        store.grant("plug-a", Permission::FilesystemRead, Some(vec!["/a".into()])).unwrap();
        assert_eq!(store.get_state("plug-a", &Permission::FilesystemRead), Some(PermissionState::Active));

        store.revoke("plug-a", &Permission::FilesystemRead).unwrap();
        assert_eq!(store.get_state("plug-a", &Permission::FilesystemRead), Some(PermissionState::Revoked));
        assert!(!store.has_permission("plug-a", &Permission::FilesystemRead));
        // Scopes preserved during revoke
        assert_eq!(store.get_approved_scopes("plug-a", &Permission::FilesystemRead), Some(vec!["/a".to_string()]));

        store.unrevoke("plug-a", &Permission::FilesystemRead).unwrap();
        assert_eq!(store.get_state("plug-a", &Permission::FilesystemRead), Some(PermissionState::Active));
        assert!(store.has_permission("plug-a", &Permission::FilesystemRead));
        assert_eq!(store.get_approved_scopes("plug-a", &Permission::FilesystemRead), Some(vec!["/a".to_string()]));
    }

    #[test]
    fn deferred_with_scopes() {
        let (mut store, _dir) = temp_store();
        store.defer("plug-a", Permission::FilesystemRead, Some(vec![])).unwrap();
        assert_eq!(store.get_approved_scopes("plug-a", &Permission::FilesystemRead), Some(vec![]));

        store.activate("plug-a", &Permission::FilesystemRead).unwrap();
        assert_eq!(store.get_approved_scopes("plug-a", &Permission::FilesystemRead), Some(vec![]));
    }

    #[test]
    fn migration_from_legacy_format() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("permissions.json");

        // Write legacy format: revoked_at set but no state field
        let legacy_json = serde_json::json!({
            "grants": {
                "plug-a": [
                    {
                        "plugin_id": "plug-a",
                        "permission": "system:info",
                        "granted_at": "2024-01-01T00:00:00Z",
                        "approved_scopes": null,
                        "revoked_at": "2024-06-01T00:00:00Z"
                    },
                    {
                        "plugin_id": "plug-a",
                        "permission": "filesystem:read",
                        "granted_at": "2024-01-01T00:00:00Z",
                        "approved_scopes": ["/foo"],
                        "revoked_at": null
                    }
                ]
            }
        });
        std::fs::write(&path, serde_json::to_string_pretty(&legacy_json).unwrap()).unwrap();

        let store = PermissionStore::load(dir.path()).unwrap();

        // revoked_at was set → state should be Revoked after migration
        assert_eq!(store.get_state("plug-a", &Permission::SystemInfo), Some(PermissionState::Revoked));
        assert!(!store.has_permission("plug-a", &Permission::SystemInfo));

        // No revoked_at → state defaults to Active (no migration needed)
        assert_eq!(store.get_state("plug-a", &Permission::FilesystemRead), Some(PermissionState::Active));
        assert!(store.has_permission("plug-a", &Permission::FilesystemRead));

        // Verify the migration was persisted
        let store2 = PermissionStore::load(dir.path()).unwrap();
        assert_eq!(store2.get_state("plug-a", &Permission::SystemInfo), Some(PermissionState::Revoked));
    }

    #[test]
    fn state_persists_through_roundtrip() {
        let dir = tempfile::tempdir().unwrap();

        {
            let mut store = PermissionStore::load(dir.path()).unwrap();
            store.grant("plug-a", Permission::SystemInfo, None).unwrap();
            store.defer("plug-a", Permission::FilesystemRead, Some(vec![])).unwrap();
            store.grant("plug-a", Permission::NetworkLocal, None).unwrap();
            store.revoke("plug-a", &Permission::NetworkLocal).unwrap();
        }

        {
            let store = PermissionStore::load(dir.path()).unwrap();
            assert_eq!(store.get_state("plug-a", &Permission::SystemInfo), Some(PermissionState::Active));
            assert_eq!(store.get_state("plug-a", &Permission::FilesystemRead), Some(PermissionState::Deferred));
            assert_eq!(store.get_state("plug-a", &Permission::NetworkLocal), Some(PermissionState::Revoked));
        }
    }
}
