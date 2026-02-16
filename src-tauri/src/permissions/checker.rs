use super::service::PermissionService;
use super::types::Permission;

#[allow(dead_code)]
pub fn check_permission(
    store: &dyn PermissionService,
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
        p if p.starts_with("/v1/fs/read")
            || p.starts_with("/v1/fs/list")
            || p.starts_with("/v1/fs/glob")
            || p.starts_with("/v1/fs/grep") =>
        {
            Some(Permission::FilesystemRead)
        }
        p if p.starts_with("/v1/fs/write") || p.starts_with("/v1/fs/edit") => {
            Some(Permission::FilesystemWrite)
        }
        p if p.starts_with("/v1/process/exec") => Some(Permission::ProcessExec),
        p if p.starts_with("/v1/process/") => Some(Permission::ProcessList),
        p if p.starts_with("/v1/docker/") => Some(Permission::DockerRead),
        // Network permissions are enforced in the handler itself (local vs internet classification)
        p if p.starts_with("/v1/network/") => None,
        // Settings and storage require auth but no specific permission — it's the plugin's own data
        p if p.starts_with("/v1/settings") => None,
        p if p.starts_with("/v1/storage") => None,
        // MCP tool access for plugins (gateway auth checks mcp:call directly)
        p if p.starts_with("/v1/mcp/") => Some(Permission::McpCall),
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
        assert_eq!(
            required_permission_for_endpoint("/v1/fs/glob"),
            Some(Permission::FilesystemRead)
        );
        assert_eq!(
            required_permission_for_endpoint("/v1/fs/grep"),
            Some(Permission::FilesystemRead)
        );
    }

    #[test]
    fn fs_write_endpoint() {
        assert_eq!(
            required_permission_for_endpoint("/v1/fs/write"),
            Some(Permission::FilesystemWrite)
        );
        assert_eq!(
            required_permission_for_endpoint("/v1/fs/edit"),
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
    fn process_exec_endpoint() {
        assert_eq!(
            required_permission_for_endpoint("/v1/process/exec"),
            Some(Permission::ProcessExec)
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
    fn mcp_endpoints_require_mcp_call() {
        assert_eq!(
            required_permission_for_endpoint("/v1/mcp/tools"),
            Some(Permission::McpCall)
        );
        assert_eq!(
            required_permission_for_endpoint("/v1/mcp/call"),
            Some(Permission::McpCall)
        );
        assert_eq!(
            required_permission_for_endpoint("/v1/mcp/events"),
            Some(Permission::McpCall)
        );
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
