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
        approved_paths: Option<Vec<String>>,
    ) -> NexusResult<()> {
        let entry = self.grants.entry(plugin_id.to_string()).or_default();

        if !entry.iter().any(|g| g.permission == permission) {
            entry.push(GrantedPermission {
                plugin_id: plugin_id.to_string(),
                permission,
                granted_at: chrono::Utc::now(),
                approved_paths,
            });
            self.save()?;
        }
        Ok(())
    }

    pub fn revoke(&mut self, plugin_id: &str, permission: &Permission) -> NexusResult<()> {
        if let Some(entry) = self.grants.get_mut(plugin_id) {
            entry.retain(|g| &g.permission != permission);
            self.save()?;
        }
        Ok(())
    }

    pub fn revoke_all(&mut self, plugin_id: &str) -> NexusResult<()> {
        self.grants.remove(plugin_id);
        self.save()?;
        Ok(())
    }

    pub fn has_permission(&self, plugin_id: &str, permission: &Permission) -> bool {
        self.grants
            .get(plugin_id)
            .is_some_and(|grants| grants.iter().any(|g| &g.permission == permission))
    }

    pub fn get_grants(&self, plugin_id: &str) -> Vec<GrantedPermission> {
        self.grants.get(plugin_id).cloned().unwrap_or_default()
    }

    pub fn get_approved_paths(&self, plugin_id: &str, permission: &Permission) -> Option<Vec<String>> {
        self.grants.get(plugin_id).and_then(|grants| {
            grants
                .iter()
                .find(|g| &g.permission == permission)
                .and_then(|g| g.approved_paths.clone())
        })
    }

    /// Add a path to the approved_paths list for a specific permission grant.
    ///
    /// No-op when the grant has `approved_paths: None` (unrestricted) — adding
    /// a path to an unrestricted grant would accidentally restrict it.
    pub fn add_approved_path(
        &mut self,
        plugin_id: &str,
        permission: &Permission,
        path: String,
    ) -> NexusResult<()> {
        if let Some(grants) = self.grants.get_mut(plugin_id) {
            if let Some(grant) = grants.iter_mut().find(|g| &g.permission == permission) {
                if let Some(ref mut paths) = grant.approved_paths {
                    if !paths.contains(&path) {
                        paths.push(path);
                        self.save()?;
                    }
                }
                // None = unrestricted — don't touch it
            }
        }
        Ok(())
    }
}
