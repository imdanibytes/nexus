use super::types::Permission;
use super::store::PermissionStore;

pub fn check_permission(
    store: &PermissionStore,
    plugin_id: &str,
    required: &Permission,
) -> bool {
    store.has_permission(plugin_id, required)
}

/// Map a request path to its required permission.
///
/// Paths here are as seen inside the nested router (after Axum strips the
/// `/api` prefix from `.nest("/api", ...)`). Do NOT use `/api/v1/...` — the
/// middleware never sees that prefix.
pub fn required_permission_for_endpoint(path: &str) -> Option<Permission> {
    match path {
        p if p.starts_with("/v1/system/") => Some(Permission::SystemInfo),
        p if p.starts_with("/v1/fs/read") || p.starts_with("/v1/fs/list") => {
            Some(Permission::FilesystemRead)
        }
        p if p.starts_with("/v1/fs/write") => Some(Permission::FilesystemWrite),
        p if p.starts_with("/v1/process/") => Some(Permission::ProcessList),
        p if p.starts_with("/v1/docker/") => Some(Permission::DockerRead),
        // Network permissions are enforced in the handler itself (local vs internet classification)
        p if p.starts_with("/v1/network/") => None,
        // Settings require auth (via middleware token check) but no specific permission
        p if p.starts_with("/v1/settings") => None,
        _ => None,
    }
}
