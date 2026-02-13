use super::types::Permission;
use super::store::PermissionStore;

#[allow(dead_code)]
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
        // Extension permissions are checked in the handler (dynamic based on path params)
        p if p.starts_with("/v1/extensions") => None,
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_endpoints_require_system_info() {
        assert_eq!(
            required_permission_for_endpoint("/v1/system/info"),
            Some(Permission::SystemInfo)
        );
        assert_eq!(
            required_permission_for_endpoint("/v1/system/anything"),
            Some(Permission::SystemInfo)
        );
    }

    #[test]
    fn fs_read_endpoints() {
        assert_eq!(
            required_permission_for_endpoint("/v1/fs/read"),
            Some(Permission::FilesystemRead)
        );
        assert_eq!(
            required_permission_for_endpoint("/v1/fs/list"),
            Some(Permission::FilesystemRead)
        );
    }

    #[test]
    fn fs_write_endpoint() {
        assert_eq!(
            required_permission_for_endpoint("/v1/fs/write"),
            Some(Permission::FilesystemWrite)
        );
    }

    #[test]
    fn process_endpoint() {
        assert_eq!(
            required_permission_for_endpoint("/v1/process/list"),
            Some(Permission::ProcessList)
        );
    }

    #[test]
    fn docker_endpoints() {
        assert_eq!(
            required_permission_for_endpoint("/v1/docker/containers"),
            Some(Permission::DockerRead)
        );
        assert_eq!(
            required_permission_for_endpoint("/v1/docker/stats/abc123"),
            Some(Permission::DockerRead)
        );
    }

    #[test]
    fn network_defers_to_handler() {
        assert_eq!(required_permission_for_endpoint("/v1/network/proxy"), None);
    }

    #[test]
    fn settings_requires_only_auth() {
        assert_eq!(required_permission_for_endpoint("/v1/settings"), None);
    }

    #[test]
    fn extensions_defers_to_handler() {
        assert_eq!(required_permission_for_endpoint("/v1/extensions"), None);
        assert_eq!(
            required_permission_for_endpoint("/v1/extensions/git-ops/status"),
            None
        );
    }

    #[test]
    fn unknown_path_returns_none() {
        assert_eq!(required_permission_for_endpoint("/v1/bogus"), None);
        assert_eq!(required_permission_for_endpoint("/something/else"), None);
    }
}
