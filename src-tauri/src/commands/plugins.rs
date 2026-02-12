use crate::plugin_manager::{docker, health};
use crate::plugin_manager::manifest::PluginManifest;
use crate::plugin_manager::registry;
use crate::plugin_manager::storage::InstalledPlugin;
use crate::AppState;
use std::path::Path;

#[tauri::command]
pub async fn plugin_list(state: tauri::State<'_, AppState>) -> Result<Vec<InstalledPlugin>, String> {
    let mgr = state.read().await;
    Ok(mgr.list().into_iter().cloned().collect())
}

#[tauri::command]
pub async fn plugin_install(
    state: tauri::State<'_, AppState>,
    manifest_url: String,
) -> Result<InstalledPlugin, String> {
    let manifest = registry::fetch_manifest(&manifest_url)
        .await
        .map_err(|e| e.to_string())?;

    let mut mgr = state.write().await;
    mgr.install(manifest).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn plugin_install_local(
    state: tauri::State<'_, AppState>,
    manifest_path: String,
) -> Result<InstalledPlugin, String> {
    let data = std::fs::read_to_string(&manifest_path)
        .map_err(|e| format!("Failed to read manifest: {}", e))?;
    let manifest: PluginManifest =
        serde_json::from_str(&data).map_err(|e| format!("Invalid manifest: {}", e))?;
    manifest
        .validate()
        .map_err(|e| format!("Invalid manifest: {}", e))?;

    // Auto-build: if the image doesn't exist locally and a Dockerfile sits
    // next to the manifest, build it before installing.
    let image_exists = docker::image_exists(&manifest.image)
        .await
        .unwrap_or(false);
    if !image_exists {
        let manifest_dir = Path::new(&manifest_path)
            .parent()
            .ok_or("Invalid manifest path")?;
        let dockerfile = manifest_dir.join("Dockerfile");
        if dockerfile.exists() {
            log::info!(
                "Image {} not found, building from {}",
                manifest.image,
                manifest_dir.display()
            );
            docker::build_image(manifest_dir, &manifest.image)
                .await
                .map_err(|e| format!("Docker build failed: {}", e))?;
        }
    }

    let mut mgr = state.write().await;
    mgr.install(manifest).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn plugin_start(
    state: tauri::State<'_, AppState>,
    plugin_id: String,
) -> Result<(), String> {
    let mut mgr = state.write().await;
    mgr.start(&plugin_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn plugin_stop(
    state: tauri::State<'_, AppState>,
    plugin_id: String,
) -> Result<(), String> {
    let mut mgr = state.write().await;
    mgr.stop(&plugin_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn plugin_remove(
    state: tauri::State<'_, AppState>,
    plugin_id: String,
) -> Result<(), String> {
    let mut mgr = state.write().await;
    mgr.remove(&plugin_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn plugin_sync_status(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<InstalledPlugin>, String> {
    health::sync_plugin_states(&state).await;
    let mgr = state.read().await;
    Ok(mgr.list().into_iter().cloned().collect())
}

#[tauri::command]
pub async fn plugin_logs(
    state: tauri::State<'_, AppState>,
    plugin_id: String,
    tail: Option<u32>,
) -> Result<Vec<String>, String> {
    let mgr = state.read().await;
    mgr.logs(&plugin_id, tail.unwrap_or(100))
        .await
        .map_err(|e| e.to_string())
}
