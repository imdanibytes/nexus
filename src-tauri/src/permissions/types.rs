use serde::{Deserialize, Serialize};

/// Three-state permission lifecycle: Active → Revoked, Deferred → Active, Deferred → Revoked.
///
/// - **Active**: Permission is granted and enforced. API calls proceed normally.
/// - **Revoked**: Permission was revoked by the user. API calls are denied (403).
/// - **Deferred**: User skipped this permission at install time. First use triggers
///   a JIT approval dialog; approving transitions to Active, denying to Revoked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PermissionState {
    #[default]
    Active,
    Revoked,
    Deferred,
}

/// Known permission variants plus dynamic extension permissions.
///
/// Extension permissions use the format "ext:{extension_id}:{operation_name}",
/// e.g. "ext:weather:get_forecast". They are stored as `Extension(String)`
/// and serialized/deserialized with a custom impl to avoid ambiguity with the
/// known string variants.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Permission {
    SystemInfo,
    FilesystemRead,
    FilesystemWrite,
    ProcessList,
    DockerRead,
    DockerManage,
    NetworkLocal,
    NetworkInternet,
    McpCall,
    ProcessExec,
    /// Dynamic extension permission: "ext:{ext_id}:{operation}"
    Extension(String),
}

/// All known permission string values (excluding dynamic Extension variants).
const KNOWN_PERMISSIONS: &[&str] = &[
    "system:info",
    "filesystem:read",
    "filesystem:write",
    "process:list",
    "process:exec",
    "docker:read",
    "docker:manage",
    "network:local",
    "network:internet",
    "mcp:call",
];

impl Permission {
    /// The serialized string form of this permission.
    pub fn as_str(&self) -> &str {
        match self {
            Permission::SystemInfo => "system:info",
            Permission::FilesystemRead => "filesystem:read",
            Permission::FilesystemWrite => "filesystem:write",
            Permission::ProcessList => "process:list",
            Permission::DockerRead => "docker:read",
            Permission::DockerManage => "docker:manage",
            Permission::NetworkLocal => "network:local",
            Permission::NetworkInternet => "network:internet",
            Permission::McpCall => "mcp:call",
            Permission::ProcessExec => "process:exec",
            Permission::Extension(s) => s.as_str(),
        }
    }

    pub fn risk_level(&self) -> &'static str {
        match self {
            Permission::SystemInfo => "low",
            Permission::FilesystemRead => "medium",
            Permission::FilesystemWrite => "high",
            Permission::ProcessList => "medium",
            Permission::DockerRead => "medium",
            Permission::DockerManage => "high",
            Permission::NetworkLocal => "medium",
            Permission::NetworkInternet => "medium",
            Permission::McpCall => "medium",
            Permission::ProcessExec => "critical",
            // Extension permissions derive risk from the operation; default to medium
            Permission::Extension(_) => "medium",
        }
    }

    pub fn description(&self) -> &str {
        match self {
            Permission::SystemInfo => "Read OS info, hostname, uptime",
            Permission::FilesystemRead => "Read files on approved paths",
            Permission::FilesystemWrite => "Write files to approved paths",
            Permission::ProcessList => "List running processes",
            Permission::DockerRead => "List containers, read stats",
            Permission::DockerManage => "Start/stop/create containers",
            Permission::NetworkLocal => "HTTP requests to LAN",
            Permission::NetworkInternet => "HTTP requests to internet",
            Permission::McpCall => "Call MCP tools from other plugins",
            Permission::ProcessExec => "Execute commands on the host system",
            Permission::Extension(s) => s.as_str(),
        }
    }
}

impl Serialize for Permission {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for Permission {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "system:info" => Ok(Permission::SystemInfo),
            "filesystem:read" => Ok(Permission::FilesystemRead),
            "filesystem:write" => Ok(Permission::FilesystemWrite),
            "process:list" => Ok(Permission::ProcessList),
            "docker:read" => Ok(Permission::DockerRead),
            "docker:manage" => Ok(Permission::DockerManage),
            "network:local" => Ok(Permission::NetworkLocal),
            "network:internet" => Ok(Permission::NetworkInternet),
            "mcp:call" => Ok(Permission::McpCall),
            "process:exec" => Ok(Permission::ProcessExec),
            _ if s.starts_with("ext:") => Ok(Permission::Extension(s)),
            _ => Err(serde::de::Error::unknown_variant(&s, KNOWN_PERMISSIONS)),
        }
    }
}

impl std::fmt::Display for Permission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_permissions_roundtrip() {
        let perms = vec![
            Permission::SystemInfo,
            Permission::FilesystemRead,
            Permission::FilesystemWrite,
            Permission::ProcessList,
            Permission::DockerRead,
            Permission::DockerManage,
            Permission::NetworkLocal,
            Permission::NetworkInternet,
            Permission::McpCall,
            Permission::ProcessExec,
        ];

        for perm in perms {
            let json = serde_json::to_value(&perm).unwrap();
            let deserialized: Permission = serde_json::from_value(json.clone()).unwrap();
            assert_eq!(perm, deserialized, "roundtrip failed for {:?}", json);
        }
    }

    #[test]
    fn extension_permission_roundtrip() {
        let perm = Permission::Extension("ext:git-ops:status".to_string());
        let json = serde_json::to_value(&perm).unwrap();
        assert_eq!(json, serde_json::json!("ext:git-ops:status"));

        let deserialized: Permission = serde_json::from_value(json).unwrap();
        assert_eq!(perm, deserialized);
    }

    #[test]
    fn unknown_permission_fails_deserialization() {
        let result = serde_json::from_value::<Permission>(serde_json::json!("bogus:perm"));
        assert!(result.is_err());
    }

    #[test]
    fn extension_prefix_required() {
        // "ext:" prefix → Extension variant
        let ok = serde_json::from_value::<Permission>(serde_json::json!("ext:foo:bar"));
        assert!(ok.is_ok());

        // No "ext:" prefix and not a known permission → error
        let err = serde_json::from_value::<Permission>(serde_json::json!("foo:bar"));
        assert!(err.is_err());
    }

    #[test]
    fn as_str_matches_serialization() {
        let perms = vec![
            (Permission::SystemInfo, "system:info"),
            (Permission::FilesystemRead, "filesystem:read"),
            (Permission::FilesystemWrite, "filesystem:write"),
            (Permission::NetworkLocal, "network:local"),
            (Permission::Extension("ext:x:y".into()), "ext:x:y"),
        ];

        for (perm, expected) in perms {
            assert_eq!(perm.as_str(), expected);
            let json = serde_json::to_value(&perm).unwrap();
            assert_eq!(json.as_str().unwrap(), expected);
        }
    }

    #[test]
    fn risk_levels_are_valid() {
        let all_perms = vec![
            Permission::SystemInfo,
            Permission::FilesystemRead,
            Permission::FilesystemWrite,
            Permission::ProcessList,
            Permission::ProcessExec,
            Permission::DockerRead,
            Permission::DockerManage,
            Permission::NetworkLocal,
            Permission::NetworkInternet,
            Permission::Extension("ext:test:op".into()),
        ];

        for perm in all_perms {
            let risk = perm.risk_level();
            assert!(
                ["low", "medium", "high", "critical"].contains(&risk),
                "invalid risk level '{}' for {:?}",
                risk,
                perm
            );
        }
    }

    #[test]
    fn descriptions_are_nonempty() {
        let perms = vec![
            Permission::SystemInfo,
            Permission::FilesystemRead,
            Permission::FilesystemWrite,
            Permission::ProcessList,
            Permission::DockerRead,
            Permission::DockerManage,
            Permission::NetworkLocal,
            Permission::NetworkInternet,
            Permission::ProcessExec,
        ];

        for perm in perms {
            assert!(!perm.description().is_empty(), "empty description for {:?}", perm);
        }
    }

    #[test]
    fn display_matches_as_str() {
        let perm = Permission::FilesystemRead;
        assert_eq!(format!("{}", perm), perm.as_str());
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrantedPermission {
    pub plugin_id: String,
    pub permission: Permission,
    pub granted_at: chrono::DateTime<chrono::Utc>,
    /// Generalized scope whitelist. Replaces the former `approved_paths`.
    ///
    /// - `None` = unrestricted (no scope checking)
    /// - `Some([])` = restricted, nothing approved yet (runtime approval on first use)
    /// - `Some(["value1", "value2"])` = these specific scope values are approved
    ///
    /// For filesystem permissions, scope values are directory paths.
    /// For extension permissions with `scope_key`, scope values are the
    /// operation-specific resource identifiers (e.g. repo paths, domains).
    #[serde(alias = "approved_paths")]
    pub approved_scopes: Option<Vec<String>>,
    /// Source of truth for the permission lifecycle. Backward-compatible:
    /// old JSON without this field deserializes as `Active` (the default).
    /// Migration in `PermissionStore::load()` reconciles with legacy `revoked_at`.
    #[serde(default)]
    pub state: PermissionState,
    /// Legacy timestamp preserved for Revoked state. Written when `state` transitions
    /// to Revoked, cleared when transitioning to Active. `state` is the source of truth.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revoked_at: Option<chrono::DateTime<chrono::Utc>>,
}
