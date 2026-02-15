use super::store::PermissionStore;
use super::types::{GrantedPermission, Permission, PermissionState};
use crate::error::NexusResult;

/// Trait for permission operations with interior mutability.
///
/// All methods take `&self` — implementations use internal locking.
/// This lets callers downgrade from `state.write().await` to `state.read().await`
/// when they only need to touch permissions, since the trait handles its own
/// synchronization internally.
pub trait PermissionService: Send + Sync {
    // Read operations
    fn has_permission(&self, plugin_id: &str, permission: &Permission) -> bool;
    fn get_grants(&self, plugin_id: &str) -> Vec<GrantedPermission>;
    fn get_state(&self, plugin_id: &str, permission: &Permission) -> Option<PermissionState>;
    fn get_approved_scopes(
        &self,
        plugin_id: &str,
        permission: &Permission,
    ) -> Option<Vec<String>>;

    // Write operations (interior mutability — &self, not &mut self)
    fn grant(
        &self,
        plugin_id: &str,
        permission: Permission,
        approved_scopes: Option<Vec<String>>,
    ) -> NexusResult<()>;
    fn revoke(&self, plugin_id: &str, permission: &Permission) -> NexusResult<()>;
    fn unrevoke(&self, plugin_id: &str, permission: &Permission) -> NexusResult<()>;
    fn revoke_all(&self, plugin_id: &str) -> NexusResult<()>;
    fn defer(
        &self,
        plugin_id: &str,
        permission: Permission,
        approved_scopes: Option<Vec<String>>,
    ) -> NexusResult<()>;
    fn activate(&self, plugin_id: &str, permission: &Permission) -> NexusResult<()>;
    fn add_approved_scope(
        &self,
        plugin_id: &str,
        permission: &Permission,
        scope: String,
    ) -> NexusResult<()>;
    fn remove_approved_scope(
        &self,
        plugin_id: &str,
        permission: &Permission,
        scope: &str,
    ) -> NexusResult<()>;

    // Aliases (default implementations)
    fn get_approved_paths(
        &self,
        plugin_id: &str,
        permission: &Permission,
    ) -> Option<Vec<String>> {
        self.get_approved_scopes(plugin_id, permission)
    }
}

// ---------------------------------------------------------------------------
// DefaultPermissionService — wraps PermissionStore with interior mutability
// ---------------------------------------------------------------------------

pub struct DefaultPermissionService {
    inner: std::sync::RwLock<PermissionStore>,
}

impl DefaultPermissionService {
    pub fn new(store: PermissionStore) -> Self {
        Self {
            inner: std::sync::RwLock::new(store),
        }
    }
}

impl PermissionService for DefaultPermissionService {
    fn has_permission(&self, plugin_id: &str, permission: &Permission) -> bool {
        self.inner.read().unwrap().has_permission(plugin_id, permission)
    }

    fn get_grants(&self, plugin_id: &str) -> Vec<GrantedPermission> {
        self.inner.read().unwrap().get_grants(plugin_id)
    }

    fn get_state(&self, plugin_id: &str, permission: &Permission) -> Option<PermissionState> {
        self.inner.read().unwrap().get_state(plugin_id, permission)
    }

    fn get_approved_scopes(
        &self,
        plugin_id: &str,
        permission: &Permission,
    ) -> Option<Vec<String>> {
        self.inner
            .read()
            .unwrap()
            .get_approved_scopes(plugin_id, permission)
    }

    fn grant(
        &self,
        plugin_id: &str,
        permission: Permission,
        approved_scopes: Option<Vec<String>>,
    ) -> NexusResult<()> {
        self.inner
            .write()
            .unwrap()
            .grant(plugin_id, permission, approved_scopes)
    }

    fn revoke(&self, plugin_id: &str, permission: &Permission) -> NexusResult<()> {
        self.inner.write().unwrap().revoke(plugin_id, permission)
    }

    fn unrevoke(&self, plugin_id: &str, permission: &Permission) -> NexusResult<()> {
        self.inner.write().unwrap().unrevoke(plugin_id, permission)
    }

    fn revoke_all(&self, plugin_id: &str) -> NexusResult<()> {
        self.inner.write().unwrap().revoke_all(plugin_id)
    }

    fn defer(
        &self,
        plugin_id: &str,
        permission: Permission,
        approved_scopes: Option<Vec<String>>,
    ) -> NexusResult<()> {
        self.inner
            .write()
            .unwrap()
            .defer(plugin_id, permission, approved_scopes)
    }

    fn activate(&self, plugin_id: &str, permission: &Permission) -> NexusResult<()> {
        self.inner.write().unwrap().activate(plugin_id, permission)
    }

    fn add_approved_scope(
        &self,
        plugin_id: &str,
        permission: &Permission,
        scope: String,
    ) -> NexusResult<()> {
        self.inner
            .write()
            .unwrap()
            .add_approved_scope(plugin_id, permission, scope)
    }

    fn remove_approved_scope(
        &self,
        plugin_id: &str,
        permission: &Permission,
        scope: &str,
    ) -> NexusResult<()> {
        self.inner
            .write()
            .unwrap()
            .remove_approved_scope(plugin_id, permission, scope)
    }
}

// ---------------------------------------------------------------------------
// MockPermissionService — for testing
// ---------------------------------------------------------------------------

#[cfg(test)]
pub mod mock {
    use super::*;
    use std::sync::Mutex;

    #[derive(Debug, Clone, PartialEq)]
    pub enum PermissionCall {
        HasPermission(String, Permission),
        GetGrants(String),
        GetState(String, Permission),
        GetApprovedScopes(String, Permission),
        Grant(String, Permission),
        Revoke(String, Permission),
        Unrevoke(String, Permission),
        RevokeAll(String),
        Defer(String, Permission),
        Activate(String, Permission),
        AddApprovedScope(String, Permission, String),
        RemoveApprovedScope(String, Permission, String),
    }

    pub struct MockPermissionService {
        store: std::sync::RwLock<PermissionStore>,
        calls: Mutex<Vec<PermissionCall>>,
        _dir: tempfile::TempDir,
    }

    impl MockPermissionService {
        pub fn new() -> Self {
            let dir = tempfile::tempdir().unwrap();
            let store = PermissionStore::load(dir.path()).unwrap();
            Self {
                store: std::sync::RwLock::new(store),
                calls: Mutex::new(Vec::new()),
                _dir: dir,
            }
        }

        /// Pre-grant a permission for testing.
        pub fn with_grant(
            self,
            plugin_id: &str,
            permission: Permission,
            scopes: Option<Vec<String>>,
        ) -> Self {
            self.store
                .write()
                .unwrap()
                .grant(plugin_id, permission, scopes)
                .unwrap();
            self
        }

        /// Pre-defer a permission for testing.
        pub fn with_deferred(
            self,
            plugin_id: &str,
            permission: Permission,
            scopes: Option<Vec<String>>,
        ) -> Self {
            self.store
                .write()
                .unwrap()
                .defer(plugin_id, permission, scopes)
                .unwrap();
            self
        }

        /// Get recorded calls for assertions.
        #[allow(dead_code)]
        pub fn calls(&self) -> Vec<PermissionCall> {
            self.calls.lock().unwrap().clone()
        }

        /// Check if a specific call was made.
        pub fn was_called(&self, call: &PermissionCall) -> bool {
            self.calls.lock().unwrap().contains(call)
        }

        fn record(&self, call: PermissionCall) {
            self.calls.lock().unwrap().push(call);
        }
    }

    impl PermissionService for MockPermissionService {
        fn has_permission(&self, plugin_id: &str, permission: &Permission) -> bool {
            self.record(PermissionCall::HasPermission(
                plugin_id.to_string(),
                permission.clone(),
            ));
            self.store
                .read()
                .unwrap()
                .has_permission(plugin_id, permission)
        }

        fn get_grants(&self, plugin_id: &str) -> Vec<GrantedPermission> {
            self.record(PermissionCall::GetGrants(plugin_id.to_string()));
            self.store.read().unwrap().get_grants(plugin_id)
        }

        fn get_state(
            &self,
            plugin_id: &str,
            permission: &Permission,
        ) -> Option<PermissionState> {
            self.record(PermissionCall::GetState(
                plugin_id.to_string(),
                permission.clone(),
            ));
            self.store.read().unwrap().get_state(plugin_id, permission)
        }

        fn get_approved_scopes(
            &self,
            plugin_id: &str,
            permission: &Permission,
        ) -> Option<Vec<String>> {
            self.record(PermissionCall::GetApprovedScopes(
                plugin_id.to_string(),
                permission.clone(),
            ));
            self.store
                .read()
                .unwrap()
                .get_approved_scopes(plugin_id, permission)
        }

        fn grant(
            &self,
            plugin_id: &str,
            permission: Permission,
            approved_scopes: Option<Vec<String>>,
        ) -> NexusResult<()> {
            self.record(PermissionCall::Grant(
                plugin_id.to_string(),
                permission.clone(),
            ));
            self.store
                .write()
                .unwrap()
                .grant(plugin_id, permission, approved_scopes)
        }

        fn revoke(&self, plugin_id: &str, permission: &Permission) -> NexusResult<()> {
            self.record(PermissionCall::Revoke(
                plugin_id.to_string(),
                permission.clone(),
            ));
            self.store.write().unwrap().revoke(plugin_id, permission)
        }

        fn unrevoke(&self, plugin_id: &str, permission: &Permission) -> NexusResult<()> {
            self.record(PermissionCall::Unrevoke(
                plugin_id.to_string(),
                permission.clone(),
            ));
            self.store.write().unwrap().unrevoke(plugin_id, permission)
        }

        fn revoke_all(&self, plugin_id: &str) -> NexusResult<()> {
            self.record(PermissionCall::RevokeAll(plugin_id.to_string()));
            self.store.write().unwrap().revoke_all(plugin_id)
        }

        fn defer(
            &self,
            plugin_id: &str,
            permission: Permission,
            approved_scopes: Option<Vec<String>>,
        ) -> NexusResult<()> {
            self.record(PermissionCall::Defer(
                plugin_id.to_string(),
                permission.clone(),
            ));
            self.store
                .write()
                .unwrap()
                .defer(plugin_id, permission, approved_scopes)
        }

        fn activate(&self, plugin_id: &str, permission: &Permission) -> NexusResult<()> {
            self.record(PermissionCall::Activate(
                plugin_id.to_string(),
                permission.clone(),
            ));
            self.store.write().unwrap().activate(plugin_id, permission)
        }

        fn add_approved_scope(
            &self,
            plugin_id: &str,
            permission: &Permission,
            scope: String,
        ) -> NexusResult<()> {
            self.record(PermissionCall::AddApprovedScope(
                plugin_id.to_string(),
                permission.clone(),
                scope.clone(),
            ));
            self.store
                .write()
                .unwrap()
                .add_approved_scope(plugin_id, permission, scope)
        }

        fn remove_approved_scope(
            &self,
            plugin_id: &str,
            permission: &Permission,
            scope: &str,
        ) -> NexusResult<()> {
            self.record(PermissionCall::RemoveApprovedScope(
                plugin_id.to_string(),
                permission.clone(),
                scope.to_string(),
            ));
            self.store
                .write()
                .unwrap()
                .remove_approved_scope(plugin_id, permission, scope)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_service_delegates_to_store() {
        let dir = tempfile::tempdir().unwrap();
        let store = PermissionStore::load(dir.path()).unwrap();
        let svc = DefaultPermissionService::new(store);

        assert!(!svc.has_permission("plug-a", &Permission::SystemInfo));
        assert_eq!(svc.get_state("plug-a", &Permission::SystemInfo), None);

        svc.grant("plug-a", Permission::SystemInfo, None).unwrap();
        assert!(svc.has_permission("plug-a", &Permission::SystemInfo));
        assert_eq!(
            svc.get_state("plug-a", &Permission::SystemInfo),
            Some(PermissionState::Active)
        );
    }

    #[test]
    fn default_service_revoke_unrevoke_cycle() {
        let dir = tempfile::tempdir().unwrap();
        let store = PermissionStore::load(dir.path()).unwrap();
        let svc = DefaultPermissionService::new(store);

        svc.grant("plug-a", Permission::FilesystemRead, Some(vec!["/a".into()]))
            .unwrap();
        assert!(svc.has_permission("plug-a", &Permission::FilesystemRead));

        svc.revoke("plug-a", &Permission::FilesystemRead).unwrap();
        assert!(!svc.has_permission("plug-a", &Permission::FilesystemRead));
        assert_eq!(
            svc.get_approved_scopes("plug-a", &Permission::FilesystemRead),
            Some(vec!["/a".to_string()])
        );

        svc.unrevoke("plug-a", &Permission::FilesystemRead).unwrap();
        assert!(svc.has_permission("plug-a", &Permission::FilesystemRead));
    }

    #[test]
    fn default_service_defer_activate() {
        let dir = tempfile::tempdir().unwrap();
        let store = PermissionStore::load(dir.path()).unwrap();
        let svc = DefaultPermissionService::new(store);

        svc.defer("plug-a", Permission::SystemInfo, None).unwrap();
        assert!(!svc.has_permission("plug-a", &Permission::SystemInfo));
        assert_eq!(
            svc.get_state("plug-a", &Permission::SystemInfo),
            Some(PermissionState::Deferred)
        );

        svc.activate("plug-a", &Permission::SystemInfo).unwrap();
        assert!(svc.has_permission("plug-a", &Permission::SystemInfo));
    }

    #[test]
    fn default_service_scope_operations() {
        let dir = tempfile::tempdir().unwrap();
        let store = PermissionStore::load(dir.path()).unwrap();
        let svc = DefaultPermissionService::new(store);

        svc.grant("plug-a", Permission::FilesystemRead, Some(vec![]))
            .unwrap();
        svc.add_approved_scope("plug-a", &Permission::FilesystemRead, "/home".into())
            .unwrap();
        assert_eq!(
            svc.get_approved_scopes("plug-a", &Permission::FilesystemRead),
            Some(vec!["/home".to_string()])
        );

        svc.remove_approved_scope("plug-a", &Permission::FilesystemRead, "/home")
            .unwrap();
        assert_eq!(
            svc.get_approved_scopes("plug-a", &Permission::FilesystemRead),
            Some(vec![])
        );
    }

    #[test]
    fn default_service_revoke_all() {
        let dir = tempfile::tempdir().unwrap();
        let store = PermissionStore::load(dir.path()).unwrap();
        let svc = DefaultPermissionService::new(store);

        svc.grant("plug-a", Permission::SystemInfo, None).unwrap();
        svc.grant("plug-a", Permission::FilesystemRead, None)
            .unwrap();
        svc.revoke_all("plug-a").unwrap();
        assert!(svc.get_grants("plug-a").is_empty());
    }

    #[test]
    fn default_service_get_approved_paths_alias() {
        let dir = tempfile::tempdir().unwrap();
        let store = PermissionStore::load(dir.path()).unwrap();
        let svc = DefaultPermissionService::new(store);

        svc.grant(
            "plug-a",
            Permission::FilesystemRead,
            Some(vec!["/foo".into()]),
        )
        .unwrap();
        assert_eq!(
            svc.get_approved_paths("plug-a", &Permission::FilesystemRead),
            Some(vec!["/foo".to_string()])
        );
    }

    #[test]
    fn mock_records_calls() {
        use mock::{MockPermissionService, PermissionCall};

        let svc = MockPermissionService::new();
        svc.grant("plug-a", Permission::SystemInfo, None).unwrap();
        svc.has_permission("plug-a", &Permission::SystemInfo);

        assert!(svc.was_called(&PermissionCall::Grant(
            "plug-a".to_string(),
            Permission::SystemInfo
        )));
        assert!(svc.was_called(&PermissionCall::HasPermission(
            "plug-a".to_string(),
            Permission::SystemInfo
        )));
    }

    #[test]
    fn mock_with_preset_grants() {
        use mock::MockPermissionService;

        let svc = MockPermissionService::new()
            .with_grant("plug-a", Permission::SystemInfo, None)
            .with_deferred("plug-b", Permission::FilesystemRead, Some(vec![]));

        assert!(svc.has_permission("plug-a", &Permission::SystemInfo));
        assert!(!svc.has_permission("plug-b", &Permission::FilesystemRead));
        assert_eq!(
            svc.get_state("plug-b", &Permission::FilesystemRead),
            Some(PermissionState::Deferred)
        );
    }
}
