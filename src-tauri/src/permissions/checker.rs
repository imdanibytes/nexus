use super::types::Permission;
use super::store::PermissionStore;

pub fn check_permission(
    store: &PermissionStore,
    plugin_id: &str,
    required: &Permission,
) -> bool {
    store.has_permission(plugin_id, required)
}

pub fn required_permission_for_endpoint(path: &str) -> Option<Permission> {
    match path {
        p if p.starts_with("/api/v1/system/") => Some(Permission::SystemInfo),
        p if p.starts_with("/api/v1/fs/read") || p.starts_with("/api/v1/fs/list") => {
            Some(Permission::FilesystemRead)
        }
        p if p.starts_with("/api/v1/fs/write") => Some(Permission::FilesystemWrite),
        p if p.starts_with("/api/v1/process/") => Some(Permission::ProcessList),
        p if p.starts_with("/api/v1/docker/") => Some(Permission::DockerRead),
        p if p.starts_with("/api/v1/network/") => None, // Checked per-request based on URL
        _ => None,
    }
}
