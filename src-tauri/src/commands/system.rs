use crate::runtime::ContainerFilters;
use crate::ActiveTheme;
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
pub struct EngineStatus {
    pub engine_id: String,
    pub installed: bool,
    pub running: bool,
    pub version: Option<String>,
    pub socket: String,
    pub message: String,
}

/// Check whether a socket/pipe path exists on disk.
fn socket_exists(socket: &str) -> bool {
    // Strip common URI prefixes
    let path = socket
        .strip_prefix("unix://")
        .or_else(|| socket.strip_prefix("npipe://"))
        .unwrap_or(socket);
    std::path::Path::new(path).exists()
}

#[tauri::command]
pub async fn check_docker(state: tauri::State<'_, AppState>) -> Result<EngineStatus, String> {
    let runtime = { state.read().await.runtime.clone() };
    let engine_id = runtime.engine_id().to_string();
    let socket = runtime.socket_path();

    if !socket_exists(&socket) {
        return Ok(EngineStatus {
            engine_id,
            installed: false,
            running: false,
            version: None,
            socket,
            message: "Container engine not found — no socket detected".to_string(),
        });
    }

    match tokio::time::timeout(std::time::Duration::from_secs(3), runtime.ping()).await {
        Ok(Ok(_)) => {
            let version = runtime.version().await.unwrap_or(None);
            Ok(EngineStatus {
                engine_id,
                installed: true,
                running: true,
                version,
                socket,
                message: "Container engine is running".to_string(),
            })
        }
        Ok(Err(e)) => Ok(EngineStatus {
            engine_id,
            installed: true,
            running: false,
            version: None,
            socket,
            message: format!("Container engine not responding: {}", e),
        }),
        Err(_) => Ok(EngineStatus {
            engine_id,
            installed: true,
            running: false,
            version: None,
            socket,
            message: "Container engine connection timed out".to_string(),
        }),
    }
}

#[tauri::command]
pub async fn container_resource_usage(
    state: tauri::State<'_, AppState>,
) -> Result<crate::runtime::ResourceUsage, String> {
    let runtime = { state.read().await.runtime.clone() };
    let mut filters = ContainerFilters::default();
    filters
        .labels
        .insert("nexus.plugin.id".to_string(), String::new());
    runtime
        .aggregate_stats(filters)
        .await
        .map_err(|e| e.to_string())
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

#[tauri::command]
pub async fn get_update_check_interval(
    state: tauri::State<'_, AppState>,
) -> Result<u32, String> {
    let mgr = state.read().await;
    Ok(mgr.settings.update_check_interval_minutes)
}

#[tauri::command]
pub async fn set_update_check_interval(
    state: tauri::State<'_, AppState>,
    minutes: u32,
) -> Result<(), String> {
    let mut mgr = state.write().await;
    mgr.settings.update_check_interval_minutes = minutes;
    mgr.settings.save().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_language(
    state: tauri::State<'_, AppState>,
    language: String,
) -> Result<(), String> {
    let mut mgr = state.write().await;
    mgr.settings.language = language;
    mgr.settings.save().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_theme(
    state: tauri::State<'_, AppState>,
    active: tauri::State<'_, ActiveTheme>,
    theme: String,
) -> Result<(), String> {
    active.set(theme.clone());
    let mut mgr = state.write().await;
    mgr.settings.theme = theme;
    mgr.settings.save().map_err(|e| e.to_string())
}

/// HEAD a URL to check if it's reachable (2xx/3xx = true).
/// Used by the extension marketplace to verify manifest URLs exist before enabling install.
#[tauri::command]
pub async fn check_url_reachable(url: String) -> Result<bool, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| e.to_string())?;
    match client.head(&url).send().await {
        Ok(resp) => Ok(resp.status().is_success() || resp.status().is_redirection()),
        Err(e) => {
            log::warn!("URL reachability check failed for {}: {}", url, e);
            Ok(true)
        }
    }
}
