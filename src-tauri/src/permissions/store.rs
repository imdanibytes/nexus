use super::types::{GrantedPermission, Permission};
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
        std::fs::write(&self.path, data)?;
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
            // If it was revoked, restore it instead of creating a duplicate
            if existing.revoked_at.is_some() {
                existing.revoked_at = None;
                self.save()?;
            }
        } else {
            entry.push(GrantedPermission {
                plugin_id: plugin_id.to_string(),
                permission,
                granted_at: chrono::Utc::now(),
                approved_scopes,
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

    /// Returns true only for active (non-revoked) permissions.
    pub fn has_permission(&self, plugin_id: &str, permission: &Permission) -> bool {
        self.grants
            .get(plugin_id)
            .is_some_and(|grants| {
                grants.iter().any(|g| &g.permission == permission && g.revoked_at.is_none())
            })
    }

    /// Returns all grants (both active and revoked).
    pub fn get_grants(&self, plugin_id: &str) -> Vec<GrantedPermission> {
        self.grants.get(plugin_id).cloned().unwrap_or_default()
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

    // --- Backward-compatible aliases for filesystem code ---

    /// Alias for `get_approved_scopes` — used by filesystem handlers.
    pub fn get_approved_paths(&self, plugin_id: &str, permission: &Permission) -> Option<Vec<String>> {
        self.get_approved_scopes(plugin_id, permission)
    }

    /// Alias for `add_approved_scope` — used by filesystem handlers.
    pub fn add_approved_path(
        &mut self,
        plugin_id: &str,
        permission: &Permission,
        path: String,
    ) -> NexusResult<()> {
        self.add_approved_scope(plugin_id, permission, path)
    }

    /// Alias for `remove_approved_scope` — used by filesystem handlers.
    pub fn remove_approved_path(
        &mut self,
        plugin_id: &str,
        permission: &Permission,
        path: &str,
    ) -> NexusResult<()> {
        self.remove_approved_scope(plugin_id, permission, path)
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
    }

    #[test]
    fn missing_permission_returns_false() {
        let (store, _dir) = temp_store();
        assert!(!store.has_permission("plug-a", &Permission::SystemInfo));
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
}
