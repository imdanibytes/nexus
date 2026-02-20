use crate::plugin_manager::registry::RegistryEntry;
use crate::AppState;

#[tauri::command]
pub async fn marketplace_search(
    state: tauri::State<'_, AppState>,
    query: String,
) -> Result<Vec<RegistryEntry>, String> {
    let mgr = state.read().await;
    Ok(mgr.search_marketplace(&query))
}

#[tauri::command]
pub async fn marketplace_refresh(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let mut mgr = state.write().await;
    mgr.refresh_registry().await.map_err(|e| e.to_string())
}

/// Load registry data from the local disk cache (no network).
/// Returns instantly — used for marketplace page open and startup.
#[tauri::command]
pub async fn marketplace_load(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let mut mgr = state.write().await;
    mgr.load_registry_cache().map_err(|e| e.to_string())
}
