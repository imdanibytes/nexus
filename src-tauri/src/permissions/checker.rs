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

/// Map a request path + HTTP method to its required permission.
///
/// Paths here are as seen inside the nested router (after Axum strips the
/// `/api` prefix from `.nest("/api", ...)`). Do NOT use `/api/v1/...` — the
/// middleware never sees that prefix.
///
/// For container endpoints, GET requests require `ContainerRead` while
/// POST/PUT/DELETE require `ContainerManage`.
pub fn required_permission_for_endpoint(
    path: &str,
    method: &axum::http::Method,
) -> Option<Permission> {
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
        p if p.starts_with("/v1/containers/") || p == "/v1/containers" => {
            if method == axum::http::Method::GET {
                Some(Permission::ContainerRead)
            } else {
                Some(Permission::ContainerManage)
            }
        }
        // Network permissions are enforced in the handler itself (local vs internet classification)
        p if p.starts_with("/v1/network/") => None,
        // Settings and storage require auth but no specific permission — it's the plugin's own data
        p if p.starts_with("/v1/settings") => None,
        p if p.starts_with("/v1/storage") => None,
        // MCP tool access for plugins (gateway auth checks mcp:call directly)
        p if p.starts_with("/v1/mcp/") => Some(Permission::McpCall),
        // Extension permissions are checked in the handler (dynamic based on path params)
        p if p.starts_with("/v1/extensions") => None,
        // Meta endpoints: self-introspection requires only auth, credentials checked in handler
        p if p.starts_with("/v1/meta/") => None,
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Method;

    #[test]
    fn system_endpoints_require_system_info() {
        assert_eq!(
            required_permission_for_endpoint("/v1/system/info", &Method::GET),
            Some(Permission::SystemInfo)
        );
        assert_eq!(
            required_permission_for_endpoint("/v1/system/anything", &Method::GET),
            Some(Permission::SystemInfo)
        );
    }

    #[test]
    fn fs_read_endpoints() {
        assert_eq!(
            required_permission_for_endpoint("/v1/fs/read", &Method::GET),
            Some(Permission::FilesystemRead)
        );
        assert_eq!(
            required_permission_for_endpoint("/v1/fs/list", &Method::GET),
            Some(Permission::FilesystemRead)
        );
        assert_eq!(
            required_permission_for_endpoint("/v1/fs/glob", &Method::GET),
            Some(Permission::FilesystemRead)
        );
        assert_eq!(
            required_permission_for_endpoint("/v1/fs/grep", &Method::GET),
            Some(Permission::FilesystemRead)
        );
    }

    #[test]
    fn fs_write_endpoint() {
        assert_eq!(
            required_permission_for_endpoint("/v1/fs/write", &Method::POST),
            Some(Permission::FilesystemWrite)
        );
        assert_eq!(
            required_permission_for_endpoint("/v1/fs/edit", &Method::POST),
            Some(Permission::FilesystemWrite)
        );
    }

    #[test]
    fn process_endpoint() {
        assert_eq!(
            required_permission_for_endpoint("/v1/process/list", &Method::GET),
            Some(Permission::ProcessList)
        );
    }

    #[test]
    fn process_exec_endpoint() {
        assert_eq!(
            required_permission_for_endpoint("/v1/process/exec", &Method::POST),
            Some(Permission::ProcessExec)
        );
    }

    #[test]
    fn container_read_endpoints() {
        assert_eq!(
            required_permission_for_endpoint("/v1/containers", &Method::GET),
            Some(Permission::ContainerRead)
        );
        assert_eq!(
            required_permission_for_endpoint("/v1/containers/abc123", &Method::GET),
            Some(Permission::ContainerRead)
        );
        assert_eq!(
            required_permission_for_endpoint("/v1/containers/images", &Method::GET),
            Some(Permission::ContainerRead)
        );
        assert_eq!(
            required_permission_for_endpoint("/v1/containers/engine", &Method::GET),
            Some(Permission::ContainerRead)
        );
    }

    #[test]
    fn container_manage_endpoints() {
        assert_eq!(
            required_permission_for_endpoint("/v1/containers/abc123/start", &Method::POST),
            Some(Permission::ContainerManage)
        );
        assert_eq!(
            required_permission_for_endpoint("/v1/containers/abc123/stop", &Method::POST),
            Some(Permission::ContainerManage)
        );
        assert_eq!(
            required_permission_for_endpoint("/v1/containers/abc123", &Method::DELETE),
            Some(Permission::ContainerManage)
        );
        assert_eq!(
            required_permission_for_endpoint("/v1/containers/images/abc123", &Method::DELETE),
            Some(Permission::ContainerManage)
        );
    }

    #[test]
    fn network_defers_to_handler() {
        assert_eq!(
            required_permission_for_endpoint("/v1/network/proxy", &Method::POST),
            None
        );
    }

    #[test]
    fn settings_requires_only_auth() {
        assert_eq!(
            required_permission_for_endpoint("/v1/settings", &Method::GET),
            None
        );
    }

    #[test]
    fn mcp_endpoints_require_mcp_call() {
        assert_eq!(
            required_permission_for_endpoint("/v1/mcp/tools", &Method::GET),
            Some(Permission::McpCall)
        );
        assert_eq!(
            required_permission_for_endpoint("/v1/mcp/call", &Method::POST),
            Some(Permission::McpCall)
        );
        assert_eq!(
            required_permission_for_endpoint("/v1/mcp/events", &Method::GET),
            Some(Permission::McpCall)
        );
    }

    #[test]
    fn extensions_defers_to_handler() {
        assert_eq!(
            required_permission_for_endpoint("/v1/extensions", &Method::GET),
            None
        );
        assert_eq!(
            required_permission_for_endpoint("/v1/extensions/git-ops/status", &Method::POST),
            None
        );
    }

    #[test]
    fn meta_defers_to_handler() {
        assert_eq!(
            required_permission_for_endpoint("/v1/meta/self", &Method::GET),
            None
        );
        assert_eq!(
            required_permission_for_endpoint("/v1/meta/stats", &Method::GET),
            None
        );
        assert_eq!(
            required_permission_for_endpoint("/v1/meta/credentials", &Method::GET),
            None
        );
        assert_eq!(
            required_permission_for_endpoint("/v1/meta/credentials/aws-credentials", &Method::POST),
            None
        );
    }

    #[test]
    fn unknown_path_returns_none() {
        assert_eq!(
            required_permission_for_endpoint("/v1/bogus", &Method::GET),
            None
        );
        assert_eq!(
            required_permission_for_endpoint("/something/else", &Method::GET),
            None
        );
    }
}
