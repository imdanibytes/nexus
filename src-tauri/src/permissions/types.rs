use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    #[serde(rename = "system:info")]
    SystemInfo,
    #[serde(rename = "filesystem:read")]
    FilesystemRead,
    #[serde(rename = "filesystem:write")]
    FilesystemWrite,
    #[serde(rename = "process:list")]
    ProcessList,
    #[serde(rename = "docker:read")]
    DockerRead,
    #[serde(rename = "docker:manage")]
    DockerManage,
    #[serde(rename = "network:local")]
    NetworkLocal,
    #[serde(rename = "network:internet")]
    NetworkInternet,
}

impl Permission {
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
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Permission::SystemInfo => "Read OS info, hostname, uptime",
            Permission::FilesystemRead => "Read files on approved paths",
            Permission::FilesystemWrite => "Write files to approved paths",
            Permission::ProcessList => "List running processes",
            Permission::DockerRead => "List containers, read stats",
            Permission::DockerManage => "Start/stop/create containers",
            Permission::NetworkLocal => "HTTP requests to LAN",
            Permission::NetworkInternet => "HTTP requests to internet",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrantedPermission {
    pub plugin_id: String,
    pub permission: Permission,
    pub granted_at: chrono::DateTime<chrono::Utc>,
    pub approved_paths: Option<Vec<String>>,
}
