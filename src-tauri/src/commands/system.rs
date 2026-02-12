use serde::Serialize;

#[derive(Serialize)]
pub struct AppVersionInfo {
    pub version: String,
    pub name: String,
}

#[tauri::command]
pub async fn app_version() -> AppVersionInfo {
    AppVersionInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        name: "Nexus".to_string(),
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
    match crate::plugin_manager::docker::connect() {
        Ok(docker) => {
            match tokio::time::timeout(std::time::Duration::from_secs(3), docker.ping()).await {
                Ok(Ok(_)) => {
                    // Docker is running — try to get version info
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
            // Check if the Docker CLI binary exists even if daemon isn't reachable
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
