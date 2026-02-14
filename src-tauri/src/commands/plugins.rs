use crate::permissions::Permission;
use crate::plugin_manager::{docker, health};
use crate::plugin_manager::manifest::PluginManifest;
use crate::plugin_manager::registry;
use crate::plugin_manager::storage::InstalledPlugin;
use crate::AppState;
use std::collections::HashMap;
use std::path::Path;

#[tauri::command]
pub async fn plugin_list(state: tauri::State<'_, AppState>) -> Result<Vec<InstalledPlugin>, String> {
    let mgr = state.read().await;
    Ok(mgr.list().into_iter().cloned().collect())
}

/// Preview a manifest from a remote URL without installing.
/// Returns the manifest so the frontend can show the permission dialog.
#[tauri::command]
pub async fn plugin_preview_remote(
    manifest_url: String,
) -> Result<PluginManifest, String> {
    let manifest = registry::fetch_manifest(&manifest_url)
        .await
        .map_err(|e| e.to_string())?;
    manifest
        .validate()
        .map_err(|e| format!("Invalid manifest: {}", e))?;
    Ok(manifest)
}

/// Preview a manifest from a local path without installing.
/// Returns the manifest so the frontend can show the permission dialog.
#[tauri::command]
pub async fn plugin_preview_local(
    manifest_path: String,
) -> Result<PluginManifest, String> {
    let data = std::fs::read_to_string(&manifest_path)
        .map_err(|e| format!("Failed to read manifest: {}", e))?;
    let manifest: PluginManifest =
        serde_json::from_str(&data).map_err(|e| format!("Invalid manifest: {}", e))?;
    manifest
        .validate()
        .map_err(|e| format!("Invalid manifest: {}", e))?;
    Ok(manifest)
}

#[tauri::command]
pub async fn plugin_install(
    state: tauri::State<'_, AppState>,
    manifest_url: String,
    approved_permissions: Vec<Permission>,
    deferred_permissions: Option<Vec<Permission>>,
    build_context: Option<String>,
) -> Result<InstalledPlugin, String> {
    let manifest = registry::fetch_manifest(&manifest_url)
        .await
        .map_err(|e| e.to_string())?;

    // If a build_context is provided (local registry), build the image from source.
    // Always rebuild so local dev changes are picked up on every install/update.
    if let Some(ref ctx) = build_context {
        let ctx_path = Path::new(ctx);
        if ctx_path.join("Dockerfile").exists() {
            log::info!(
                "Building image {} from {}",
                manifest.image,
                ctx_path.display()
            );
            docker::build_image(ctx_path, &manifest.image)
                .await
                .map_err(|e| format!("Docker build failed: {}", e))?;
        }
    }

    let mut mgr = state.write().await;
    mgr.install(manifest, approved_permissions, deferred_permissions.unwrap_or_default(), Some(&manifest_url))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn plugin_install_local(
    state: tauri::State<'_, AppState>,
    manifest_path: String,
    approved_permissions: Vec<Permission>,
    deferred_permissions: Option<Vec<Permission>>,
) -> Result<InstalledPlugin, String> {
    let data = std::fs::read_to_string(&manifest_path)
        .map_err(|e| format!("Failed to read manifest: {}", e))?;
    let manifest: PluginManifest =
        serde_json::from_str(&data).map_err(|e| format!("Invalid manifest: {}", e))?;
    manifest
        .validate()
        .map_err(|e| format!("Invalid manifest: {}", e))?;

    // Auto-build: if a Dockerfile sits next to the manifest, always rebuild.
    // Local installs are a dev workflow — always pick up the latest code.
    let manifest_dir = Path::new(&manifest_path)
        .parent()
        .ok_or("Invalid manifest path")?;
    let dockerfile = manifest_dir.join("Dockerfile");
    if dockerfile.exists() {
        log::info!(
            "Building image {} from {}",
            manifest.image,
            manifest_dir.display()
        );
        docker::build_image(manifest_dir, &manifest.image)
            .await
            .map_err(|e| format!("Docker build failed: {}", e))?;
    }

    let mut mgr = state.write().await;
    mgr.install(manifest, approved_permissions, deferred_permissions.unwrap_or_default(), None)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn plugin_start(
    state: tauri::State<'_, AppState>,
    plugin_id: String,
) -> Result<(), String> {
    let mut mgr = state.write().await;
    mgr.start(&plugin_id).await.map_err(|e| e.to_string())?;
    mgr.notify_tools_changed();
    Ok(())
}

#[tauri::command]
pub async fn plugin_stop(
    state: tauri::State<'_, AppState>,
    plugin_id: String,
) -> Result<(), String> {
    let mut mgr = state.write().await;
    mgr.stop(&plugin_id).await.map_err(|e| e.to_string())?;
    mgr.notify_tools_changed();
    Ok(())
}

#[tauri::command]
pub async fn plugin_remove(
    state: tauri::State<'_, AppState>,
    plugin_id: String,
) -> Result<(), String> {
    let mut mgr = state.write().await;
    mgr.remove(&plugin_id).await.map_err(|e| e.to_string())?;
    mgr.notify_tools_changed();
    Ok(())
}


/// Check if a Docker image is available in its remote registry.
/// Used by the marketplace to disable the install button when images don't exist.
#[tauri::command]
pub async fn check_image_available(image: String) -> Result<bool, String> {
    Ok(docker::check_image_available(&image).await)
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

#[tauri::command]
pub async fn plugin_get_settings(
    state: tauri::State<'_, AppState>,
    plugin_id: String,
) -> Result<HashMap<String, serde_json::Value>, String> {
    let mgr = state.read().await;

    // Start with defaults from the manifest
    let mut values = HashMap::new();
    if let Some(plugin) = mgr.storage.get(&plugin_id) {
        for def in &plugin.manifest.settings {
            if let Some(default) = &def.default {
                values.insert(def.key.clone(), default.clone());
            }
        }
    }

    // Overlay stored values
    let stored = mgr.plugin_settings.get(&plugin_id);
    for (k, v) in stored {
        values.insert(k, v);
    }

    Ok(values)
}

#[tauri::command]
pub async fn plugin_save_settings(
    state: tauri::State<'_, AppState>,
    plugin_id: String,
    values: HashMap<String, serde_json::Value>,
) -> Result<(), String> {
    let mut mgr = state.write().await;
    mgr.plugin_settings
        .set(&plugin_id, values)
        .map_err(|e| e.to_string())
}

/// Get storage usage info for a plugin (KV bytes).
#[tauri::command]
pub async fn plugin_storage_info(
    state: tauri::State<'_, AppState>,
    plugin_id: String,
) -> Result<u64, String> {
    let mgr = state.read().await;
    Ok(crate::host_api::storage::plugin_storage_bytes(
        &mgr.data_dir,
        &plugin_id,
    ))
}

/// Clear all KV storage for a plugin.
#[tauri::command]
pub async fn plugin_clear_storage(
    state: tauri::State<'_, AppState>,
    plugin_id: String,
) -> Result<(), String> {
    let mgr = state.read().await;
    crate::host_api::storage::remove_plugin_storage(&mgr.data_dir, &plugin_id);
    Ok(())
}
