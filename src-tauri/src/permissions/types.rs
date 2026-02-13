use serde::{Deserialize, Serialize};

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
    /// Dynamic extension permission: "ext:{ext_id}:{operation}"
    Extension(String),
}

/// All known permission string values (excluding dynamic Extension variants).
const KNOWN_PERMISSIONS: &[&str] = &[
    "system:info",
    "filesystem:read",
    "filesystem:write",
    "process:list",
    "docker:read",
    "docker:manage",
    "network:local",
    "network:internet",
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrantedPermission {
    pub plugin_id: String,
    pub permission: Permission,
    pub granted_at: chrono::DateTime<chrono::Utc>,
    pub approved_paths: Option<Vec<String>>,
}
