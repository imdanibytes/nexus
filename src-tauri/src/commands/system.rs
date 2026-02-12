use crate::plugin_manager::docker;
use crate::AppState;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct AppVersionInfo {
    pub version: String,
    pub name: String,
    pub commit: Option<String>,
}

#[tauri::command]
pub async fn app_version() -> AppVersionInfo {
    AppVersionInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        name: "Nexus".to_string(),
        commit: option_env!("NEXUS_COMMIT").map(|s| s.to_string()),
    }
}

#[derive(Serialize)]
pub struct DockerStatus {
    pub installed: bool,
    pub running: bool,
    pub version: Option<String>,
    pub message: String,
}

#[tauri::command]
pub async fn check_docker() -> DockerStatus {
    match docker::connect() {
        Ok(docker) => {
            match tokio::time::timeout(std::time::Duration::from_secs(3), docker.ping()).await {
                Ok(Ok(_)) => {
                    let version = match docker.version().await {
                        Ok(v) => v.version,
                        Err(_) => None,
                    };
                    DockerStatus {
                        installed: true,
                        running: true,
                        version,
                        message: "Docker is running".to_string(),
                    }
                }
                Ok(Err(e)) => DockerStatus {
                    installed: true,
                    running: false,
                    version: None,
                    message: format!("Docker is installed but not responding: {}", e),
                },
                Err(_) => DockerStatus {
                    installed: true,
                    running: false,
                    version: None,
                    message: "Docker connection timed out — engine may not be running".to_string(),
                },
            }
        }
        Err(_) => {
            let cli_exists = std::process::Command::new("docker")
                .arg("--version")
                .output()
                .is_ok();

            DockerStatus {
                installed: cli_exists,
                running: false,
                version: None,
                message: if cli_exists {
                    "Docker is installed but the engine is not running".to_string()
                } else {
                    "Docker is not installed".to_string()
                },
            }
        }
    }
}

#[tauri::command]
pub async fn open_docker_desktop() -> Result<(), String> {
    std::process::Command::new("/usr/bin/open")
        .arg("/Applications/Docker.app")
        .spawn()
        .map_err(|e| format!("Failed to open Docker Desktop: {}", e))?;
    Ok(())
}

// Resource usage / quotas

#[derive(Serialize)]
pub struct ResourceUsage {
    pub cpu_percent: f64,
    pub memory_mb: f64,
}

#[tauri::command]
pub async fn container_resource_usage() -> Result<ResourceUsage, String> {
    docker::aggregate_stats().await.map_err(|e| e.to_string())
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ResourceQuotas {
    pub cpu_percent: Option<f64>,
    pub memory_mb: Option<u64>,
}

#[tauri::command]
pub async fn get_resource_quotas(
    state: tauri::State<'_, AppState>,
) -> Result<ResourceQuotas, String> {
    let mgr = state.read().await;
    let settings = &mgr.settings;
    Ok(ResourceQuotas {
        cpu_percent: settings.cpu_quota_percent,
        memory_mb: settings.memory_limit_mb,
    })
}

#[tauri::command]
pub async fn save_resource_quotas(
    state: tauri::State<'_, AppState>,
    cpu_percent: Option<f64>,
    memory_mb: Option<u64>,
) -> Result<(), String> {
    let mut mgr = state.write().await;
    mgr.settings.cpu_quota_percent = cpu_percent;
    mgr.settings.memory_limit_mb = memory_mb;
    mgr.settings.save().map_err(|e| e.to_string())
}
