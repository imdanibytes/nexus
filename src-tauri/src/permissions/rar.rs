//! RFC 9396 Rich Authorization Requests — `authorization_details` on OAuth tokens.
//!
//! Encodes Nexus plugin permissions as structured authorization details carried
//! on access/refresh tokens. Middleware checks the token first (fast path),
//! falls back to PermissionStore for Deferred/stale cases.

use serde::{Deserialize, Serialize};

use super::types::{GrantedPermission, Permission, PermissionState};

/// A single RFC 9396 authorization detail entry.
///
/// Maps to the JSON structure:
/// ```json
/// {"type": "nexus:fs", "actions": ["read"], "locations": ["/Users/dani/projects"]}
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthorizationDetail {
    /// Permission category (required by RFC 9396). Uses `nexus:` prefix.
    #[serde(rename = "type")]
    pub detail_type: String,

    /// Specific operations within the category.
    pub actions: Vec<String>,

    /// Approved scopes/paths. Informational for now — fine-grained scope
    /// checking remains in individual handlers via `get_approved_scopes()`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locations: Option<Vec<String>>,

    /// Extension ID (only for `nexus:extension` type).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identifier: Option<String>,
}

/// Convert a Permission variant to its RAR type and action.
fn permission_to_type_action(perm: &Permission) -> (&'static str, &'static str) {
    match perm {
        Permission::SystemInfo => ("nexus:system", "info"),
        Permission::FilesystemRead => ("nexus:fs", "read"),
        Permission::FilesystemWrite => ("nexus:fs", "write"),
        Permission::ProcessList => ("nexus:process", "list"),
        Permission::ProcessExec => ("nexus:process", "exec"),
        Permission::DockerRead => ("nexus:docker", "read"),
        Permission::DockerManage => ("nexus:docker", "manage"),
        Permission::NetworkLocal => ("nexus:network", "local"),
        Permission::NetworkInternet => ("nexus:network", "internet"),
        Permission::McpCall => ("nexus:mcp", "call"),
        Permission::Extension(_) => ("nexus:extension", ""),
        Permission::McpAccess(_) => ("nexus:mcp", "access"),
    }
}

/// Build RFC 9396 `authorization_details` from a set of granted permissions.
///
/// Only Active grants are included — Deferred and Revoked are excluded.
/// Each permission becomes one AuthorizationDetail entry.
pub fn build_authorization_details(grants: &[GrantedPermission]) -> Vec<AuthorizationDetail> {
    grants
        .iter()
        .filter(|g| g.state == PermissionState::Active)
        .map(|g| permission_to_detail(&g.permission, &g.approved_scopes))
        .collect()
}

/// Convert a single Permission + scopes into an AuthorizationDetail.
fn permission_to_detail(
    perm: &Permission,
    approved_scopes: &Option<Vec<String>>,
) -> AuthorizationDetail {
    match perm {
        Permission::Extension(ext_str) => {
            // Format: "ext:{ext_id}:{operation}"
            let parts: Vec<&str> = ext_str.splitn(3, ':').collect();
            let (ext_id, operation) = if parts.len() >= 3 {
                (parts[1].to_string(), parts[2].to_string())
            } else {
                (ext_str.clone(), String::new())
            };

            let locations = approved_scopes
                .as_ref()
                .filter(|s| !s.is_empty())
                .cloned();

            let mut actions = Vec::new();
            if !operation.is_empty() {
                actions.push(operation);
            }

            AuthorizationDetail {
                detail_type: "nexus:extension".to_string(),
                actions,
                locations,
                identifier: Some(ext_id),
            }
        }
        Permission::McpAccess(mcp_str) => {
            // Format: "mcp:{target_plugin_id}"
            let target_plugin_id = mcp_str
                .strip_prefix("mcp:")
                .unwrap_or(mcp_str)
                .to_string();

            AuthorizationDetail {
                detail_type: "nexus:mcp".to_string(),
                actions: vec!["access".to_string()],
                locations: None,
                identifier: Some(target_plugin_id),
            }
        }
        _ => {
            let (detail_type, action) = permission_to_type_action(perm);
            let locations = approved_scopes
                .as_ref()
                .filter(|s| !s.is_empty())
                .cloned();

            AuthorizationDetail {
                detail_type: detail_type.to_string(),
                actions: vec![action.to_string()],
                locations,
                identifier: None,
            }
        }
    }
}

/// Check whether a set of authorization details satisfies a required Permission.
///
/// This is a **coarse** check: type + action match only. Fine-grained scope
/// checking (filesystem paths, extension scopes) remains in individual handlers.
pub fn details_satisfy(details: &[AuthorizationDetail], required: &Permission) -> bool {
    match required {
        Permission::Extension(ext_str) => {
            let parts: Vec<&str> = ext_str.splitn(3, ':').collect();
            let (ext_id, operation) = if parts.len() >= 3 {
                (parts[1], parts[2])
            } else {
                return false;
            };

            details.iter().any(|d| {
                d.detail_type == "nexus:extension"
                    && d.identifier.as_deref() == Some(ext_id)
                    && d.actions.iter().any(|a| a == operation)
            })
        }
        Permission::McpAccess(mcp_str) => {
            let target = mcp_str.strip_prefix("mcp:").unwrap_or(mcp_str);
            details.iter().any(|d| {
                d.detail_type == "nexus:mcp"
                    && d.actions.iter().any(|a| a == "access")
                    && d.identifier.as_deref() == Some(target)
            })
        }
        _ => {
            let (required_type, required_action) = permission_to_type_action(required);
            details.iter().any(|d| {
                d.detail_type == required_type
                    && d.actions.iter().any(|a| a == required_action)
            })
        }
    }
}

/// All supported authorization detail types for the server metadata endpoint.
pub const SUPPORTED_DETAIL_TYPES: &[&str] = &[
    "nexus:system",
    "nexus:fs",
    "nexus:process",
    "nexus:docker",
    "nexus:network",
    "nexus:mcp",
    "nexus:extension",
];

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn grant(perm: Permission, state: PermissionState, scopes: Option<Vec<String>>) -> GrantedPermission {
        GrantedPermission {
            plugin_id: "com.test.plugin".to_string(),
            permission: perm,
            granted_at: Utc::now(),
            approved_scopes: scopes,
            state,
            revoked_at: None,
        }
    }

    // ── Serialization ────────────────────────────────────────────

    #[test]
    fn serialization_roundtrip_basic() {
        let detail = AuthorizationDetail {
            detail_type: "nexus:fs".to_string(),
            actions: vec!["read".to_string()],
            locations: Some(vec!["/home/user".to_string()]),
            identifier: None,
        };

        let json = serde_json::to_string(&detail).unwrap();
        let parsed: AuthorizationDetail = serde_json::from_str(&json).unwrap();
        assert_eq!(detail, parsed);
    }

    #[test]
    fn serialization_roundtrip_extension() {
        let detail = AuthorizationDetail {
            detail_type: "nexus:extension".to_string(),
            actions: vec!["status".to_string(), "commit".to_string()],
            locations: Some(vec!["/repo/path".to_string()]),
            identifier: Some("git-ops".to_string()),
        };

        let json = serde_json::to_string(&detail).unwrap();
        let parsed: AuthorizationDetail = serde_json::from_str(&json).unwrap();
        assert_eq!(detail, parsed);
    }

    #[test]
    fn serialization_skips_none_fields() {
        let detail = AuthorizationDetail {
            detail_type: "nexus:system".to_string(),
            actions: vec!["info".to_string()],
            locations: None,
            identifier: None,
        };

        let json = serde_json::to_string(&detail).unwrap();
        assert!(!json.contains("locations"));
        assert!(!json.contains("identifier"));
    }

    #[test]
    fn type_field_uses_rfc_name() {
        let detail = AuthorizationDetail {
            detail_type: "nexus:system".to_string(),
            actions: vec!["info".to_string()],
            locations: None,
            identifier: None,
        };

        let value: serde_json::Value = serde_json::to_value(&detail).unwrap();
        assert!(value.get("type").is_some());
        assert!(value.get("detail_type").is_none());
    }

    // ── build_authorization_details ──────────────────────────────

    #[test]
    fn build_from_all_permission_types() {
        let grants = vec![
            grant(Permission::SystemInfo, PermissionState::Active, None),
            grant(Permission::FilesystemRead, PermissionState::Active, Some(vec!["/home".into()])),
            grant(Permission::FilesystemWrite, PermissionState::Active, Some(vec!["/tmp".into()])),
            grant(Permission::ProcessList, PermissionState::Active, None),
            grant(Permission::ProcessExec, PermissionState::Active, None),
            grant(Permission::DockerRead, PermissionState::Active, None),
            grant(Permission::DockerManage, PermissionState::Active, None),
            grant(Permission::NetworkLocal, PermissionState::Active, None),
            grant(Permission::NetworkInternet, PermissionState::Active, None),
            grant(Permission::McpCall, PermissionState::Active, None),
        ];

        let details = build_authorization_details(&grants);
        assert_eq!(details.len(), 10);

        assert_eq!(details[0].detail_type, "nexus:system");
        assert_eq!(details[0].actions, vec!["info"]);

        assert_eq!(details[1].detail_type, "nexus:fs");
        assert_eq!(details[1].actions, vec!["read"]);
        assert_eq!(details[1].locations, Some(vec!["/home".to_string()]));

        assert_eq!(details[2].detail_type, "nexus:fs");
        assert_eq!(details[2].actions, vec!["write"]);

        assert_eq!(details[3].detail_type, "nexus:process");
        assert_eq!(details[3].actions, vec!["list"]);

        assert_eq!(details[4].detail_type, "nexus:process");
        assert_eq!(details[4].actions, vec!["exec"]);

        assert_eq!(details[5].detail_type, "nexus:docker");
        assert_eq!(details[5].actions, vec!["read"]);

        assert_eq!(details[6].detail_type, "nexus:docker");
        assert_eq!(details[6].actions, vec!["manage"]);

        assert_eq!(details[7].detail_type, "nexus:network");
        assert_eq!(details[7].actions, vec!["local"]);

        assert_eq!(details[8].detail_type, "nexus:network");
        assert_eq!(details[8].actions, vec!["internet"]);

        assert_eq!(details[9].detail_type, "nexus:mcp");
        assert_eq!(details[9].actions, vec!["call"]);
    }

    #[test]
    fn build_extension_permission() {
        let grants = vec![grant(
            Permission::Extension("ext:git-ops:status".to_string()),
            PermissionState::Active,
            Some(vec!["/repo/path".to_string()]),
        )];

        let details = build_authorization_details(&grants);
        assert_eq!(details.len(), 1);
        assert_eq!(details[0].detail_type, "nexus:extension");
        assert_eq!(details[0].actions, vec!["status"]);
        assert_eq!(details[0].identifier, Some("git-ops".to_string()));
        assert_eq!(details[0].locations, Some(vec!["/repo/path".to_string()]));
    }

    #[test]
    fn build_excludes_non_active_grants() {
        let grants = vec![
            grant(Permission::SystemInfo, PermissionState::Active, None),
            grant(Permission::FilesystemRead, PermissionState::Deferred, None),
            grant(Permission::DockerRead, PermissionState::Revoked, None),
        ];

        let details = build_authorization_details(&grants);
        assert_eq!(details.len(), 1);
        assert_eq!(details[0].detail_type, "nexus:system");
    }

    #[test]
    fn build_empty_grants() {
        let details = build_authorization_details(&[]);
        assert!(details.is_empty());
    }

    #[test]
    fn build_empty_scopes_not_included_as_locations() {
        let grants = vec![grant(
            Permission::FilesystemRead,
            PermissionState::Active,
            Some(vec![]),
        )];

        let details = build_authorization_details(&grants);
        assert_eq!(details.len(), 1);
        assert!(details[0].locations.is_none(), "empty scopes should not produce locations");
    }

    // ── details_satisfy ──────────────────────────────────────────

    #[test]
    fn satisfy_basic_permission() {
        let details = vec![AuthorizationDetail {
            detail_type: "nexus:system".to_string(),
            actions: vec!["info".to_string()],
            locations: None,
            identifier: None,
        }];

        assert!(details_satisfy(&details, &Permission::SystemInfo));
        assert!(!details_satisfy(&details, &Permission::FilesystemRead));
    }

    #[test]
    fn satisfy_fs_read_vs_write() {
        let details = vec![AuthorizationDetail {
            detail_type: "nexus:fs".to_string(),
            actions: vec!["read".to_string()],
            locations: None,
            identifier: None,
        }];

        assert!(details_satisfy(&details, &Permission::FilesystemRead));
        assert!(!details_satisfy(&details, &Permission::FilesystemWrite));
    }

    #[test]
    fn satisfy_extension_permission() {
        let details = vec![AuthorizationDetail {
            detail_type: "nexus:extension".to_string(),
            actions: vec!["status".to_string(), "commit".to_string()],
            locations: None,
            identifier: Some("git-ops".to_string()),
        }];

        assert!(details_satisfy(
            &details,
            &Permission::Extension("ext:git-ops:status".into())
        ));
        assert!(details_satisfy(
            &details,
            &Permission::Extension("ext:git-ops:commit".into())
        ));
        assert!(!details_satisfy(
            &details,
            &Permission::Extension("ext:git-ops:push".into())
        ));
        assert!(!details_satisfy(
            &details,
            &Permission::Extension("ext:other:status".into())
        ));
    }

    #[test]
    fn satisfy_multiple_details() {
        let details = vec![
            AuthorizationDetail {
                detail_type: "nexus:system".to_string(),
                actions: vec!["info".to_string()],
                locations: None,
                identifier: None,
            },
            AuthorizationDetail {
                detail_type: "nexus:fs".to_string(),
                actions: vec!["read".to_string()],
                locations: None,
                identifier: None,
            },
        ];

        assert!(details_satisfy(&details, &Permission::SystemInfo));
        assert!(details_satisfy(&details, &Permission::FilesystemRead));
        assert!(!details_satisfy(&details, &Permission::DockerRead));
    }

    #[test]
    fn satisfy_empty_details_denies_all() {
        assert!(!details_satisfy(&[], &Permission::SystemInfo));
        assert!(!details_satisfy(&[], &Permission::FilesystemRead));
    }

    // ── McpAccess ─────────────────────────────────────────────

    #[test]
    fn build_mcp_access_permission() {
        let grants = vec![grant(
            Permission::McpAccess("mcp:com.nexus.agent".to_string()),
            PermissionState::Active,
            None,
        )];

        let details = build_authorization_details(&grants);
        assert_eq!(details.len(), 1);
        assert_eq!(details[0].detail_type, "nexus:mcp");
        assert_eq!(details[0].actions, vec!["access"]);
        assert_eq!(details[0].identifier, Some("com.nexus.agent".to_string()));
        assert!(details[0].locations.is_none());
    }

    #[test]
    fn satisfy_mcp_access_permission() {
        let details = vec![AuthorizationDetail {
            detail_type: "nexus:mcp".to_string(),
            actions: vec!["access".to_string()],
            locations: None,
            identifier: Some("com.nexus.agent".to_string()),
        }];

        assert!(details_satisfy(
            &details,
            &Permission::McpAccess("mcp:com.nexus.agent".into())
        ));
        // Different target plugin → denied
        assert!(!details_satisfy(
            &details,
            &Permission::McpAccess("mcp:com.nexus.cookie-jar".into())
        ));
        // Blanket mcp:call is NOT satisfied by an access detail
        assert!(!details_satisfy(&details, &Permission::McpCall));
    }

    #[test]
    fn mcp_call_does_not_satisfy_mcp_access() {
        let details = vec![AuthorizationDetail {
            detail_type: "nexus:mcp".to_string(),
            actions: vec!["call".to_string()],
            locations: None,
            identifier: None,
        }];

        // Blanket mcp:call detail satisfies McpCall
        assert!(details_satisfy(&details, &Permission::McpCall));
        // But it does NOT satisfy a specific McpAccess
        assert!(!details_satisfy(
            &details,
            &Permission::McpAccess("mcp:com.nexus.agent".into())
        ));
    }

    #[test]
    fn satisfy_all_permission_types() {
        let grants = vec![
            grant(Permission::SystemInfo, PermissionState::Active, None),
            grant(Permission::FilesystemRead, PermissionState::Active, None),
            grant(Permission::FilesystemWrite, PermissionState::Active, None),
            grant(Permission::ProcessList, PermissionState::Active, None),
            grant(Permission::ProcessExec, PermissionState::Active, None),
            grant(Permission::DockerRead, PermissionState::Active, None),
            grant(Permission::DockerManage, PermissionState::Active, None),
            grant(Permission::NetworkLocal, PermissionState::Active, None),
            grant(Permission::NetworkInternet, PermissionState::Active, None),
            grant(Permission::McpCall, PermissionState::Active, None),
        ];
        let details = build_authorization_details(&grants);

        assert!(details_satisfy(&details, &Permission::SystemInfo));
        assert!(details_satisfy(&details, &Permission::FilesystemRead));
        assert!(details_satisfy(&details, &Permission::FilesystemWrite));
        assert!(details_satisfy(&details, &Permission::ProcessList));
        assert!(details_satisfy(&details, &Permission::ProcessExec));
        assert!(details_satisfy(&details, &Permission::DockerRead));
        assert!(details_satisfy(&details, &Permission::DockerManage));
        assert!(details_satisfy(&details, &Permission::NetworkLocal));
        assert!(details_satisfy(&details, &Permission::NetworkInternet));
        assert!(details_satisfy(&details, &Permission::McpCall));
    }
}
