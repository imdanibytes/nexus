use std::path::Path;

use crate::extensions::storage::InstalledExtension;
use crate::plugin_manager::{docker, registry};
use crate::plugin_manager::storage::InstalledPlugin;
use crate::update_checker::{self, AvailableUpdate};
use crate::AppState;

/// Check all installed plugins and extensions for available updates.
#[tauri::command]
pub async fn check_updates(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<AvailableUpdate>, String> {
    let mut mgr = state.write().await;

    let updates = update_checker::check_for_updates(
        &mgr.storage,
        &mgr.extension_loader.storage,
        &mgr.registry_cache,
        &mgr.extension_registry_cache,
        &mgr.extension_loader.trusted_keys,
        &mgr.registry_store,
        &mgr.update_state.dismissed,
    );

    mgr.update_state.last_checked = Some(chrono::Utc::now());
    mgr.update_state.available_updates = updates.clone();

    update_checker::save_update_state(&mgr.data_dir, &mgr.update_state)
        .map_err(|e| e.to_string())?;

    Ok(updates)
}

/// Return cached updates without re-checking.
#[tauri::command]
pub async fn get_cached_updates(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<AvailableUpdate>, String> {
    let mgr = state.read().await;
    Ok(mgr.update_state.available_updates.clone())
}

/// Dismiss an update so it no longer appears.
#[tauri::command]
pub async fn dismiss_update(
    state: tauri::State<'_, AppState>,
    item_id: String,
    version: String,
) -> Result<(), String> {
    let mut mgr = state.write().await;
    mgr.update_state
        .dismissed
        .insert(item_id.clone(), version);
    mgr.update_state
        .available_updates
        .retain(|u| u.item_id != item_id);

    update_checker::save_update_state(&mgr.data_dir, &mgr.update_state)
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Update a plugin to a new version from a manifest URL.
#[tauri::command]
pub async fn update_plugin(
    state: tauri::State<'_, AppState>,
    manifest_url: String,
    expected_digest: Option<String>,
    build_context: Option<String>,
) -> Result<InstalledPlugin, String> {
    let manifest = registry::fetch_manifest(&manifest_url)
        .await
        .map_err(|e| e.to_string())?;

    if let Some(ref ctx) = build_context {
        let ctx_path = Path::new(ctx);
        if ctx_path.join("Dockerfile").exists() {
            log::info!("Rebuilding image {} from {}", manifest.image, ctx_path.display());
            docker::build_image(ctx_path, &manifest.image)
                .await
                .map_err(|e| format!("Docker build failed: {}", e))?;
        }
    }

    let mut mgr = state.write().await;
    let result = mgr
        .update_plugin(manifest, expected_digest)
        .await
        .map_err(|e| e.to_string())?;

    mgr.notify_tools_changed();
    Ok(result)
}

/// Update an extension to a new version. Rejects key changes.
#[tauri::command]
pub async fn update_extension(
    state: tauri::State<'_, AppState>,
    manifest_url: String,
) -> Result<InstalledExtension, String> {
    let manifest = registry::fetch_extension_manifest(&manifest_url)
        .await
        .map_err(|e| e.to_string())?;

    let mut mgr = state.write().await;
    mgr.update_extension(manifest, false, Some(&manifest_url))
        .await
        .map_err(|e| e.to_string())
}

/// Update an extension, accepting author key changes (force rotate).
#[tauri::command]
pub async fn update_extension_force_key(
    state: tauri::State<'_, AppState>,
    manifest_url: String,
) -> Result<InstalledExtension, String> {
    let manifest = registry::fetch_extension_manifest(&manifest_url)
        .await
        .map_err(|e| e.to_string())?;

    let mut mgr = state.write().await;
    mgr.update_extension(manifest, true, Some(&manifest_url))
        .await
        .map_err(|e| e.to_string())
}

/// Return the last time updates were checked (ISO 8601).
#[tauri::command]
pub async fn last_update_check(
    state: tauri::State<'_, AppState>,
) -> Result<Option<String>, String> {
    let mgr = state.read().await;
    Ok(mgr.update_state.last_checked.map(|dt| dt.to_rfc3339()))
}
