use crate::permissions::{GrantedPermission, Permission};
use crate::AppState;

#[tauri::command]
pub async fn permission_grant(
    state: tauri::State<'_, AppState>,
    plugin_id: String,
    permissions: Vec<Permission>,
) -> Result<(), String> {
    let mut mgr = state.write().await;
    for perm in permissions {
        mgr.permissions
            .grant(&plugin_id, perm, None)
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn permission_revoke(
    state: tauri::State<'_, AppState>,
    plugin_id: String,
    permissions: Vec<Permission>,
) -> Result<(), String> {
    let mut mgr = state.write().await;
    for perm in &permissions {
        mgr.permissions
            .revoke(&plugin_id, perm)
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn permission_list(
    state: tauri::State<'_, AppState>,
    plugin_id: String,
) -> Result<Vec<GrantedPermission>, String> {
    let mgr = state.read().await;
    Ok(mgr.permissions.get_grants(&plugin_id))
}
