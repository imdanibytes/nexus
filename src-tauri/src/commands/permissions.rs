use std::sync::Arc;

use crate::host_api::approval::{ApprovalBridge, ApprovalDecision};
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

/// Called by the frontend approval dialog when the user makes a decision.
///
/// For `Approve` (persist): writes the approved path to `PermissionStore`
/// BEFORE sending the decision on the channel, guaranteeing the path is
/// persisted by the time the HTTP handler resumes.
#[tauri::command]
pub async fn runtime_approval_respond(
    state: tauri::State<'_, AppState>,
    bridge: tauri::State<'_, Arc<ApprovalBridge>>,
    request_id: String,
    decision: ApprovalDecision,
    plugin_id: String,
    category: String,
    context: std::collections::HashMap<String, String>,
) -> Result<(), String> {
    // Persist the approved path before unblocking the handler
    if decision == ApprovalDecision::Approve {
        if category == "filesystem" {
            if let Some(parent_dir) = context.get("parent_dir") {
                let permission_str = context.get("permission").cloned().unwrap_or_default();
                let permission: Permission =
                    serde_json::from_value(serde_json::Value::String(permission_str))
                        .map_err(|e| format!("invalid permission: {}", e))?;

                let mut mgr = state.write().await;
                mgr.permissions
                    .add_approved_path(&plugin_id, &permission, parent_dir.clone())
                    .map_err(|e| e.to_string())?;
            }
        }
        // Future categories (e.g. "network") persist their own data here
    }

    bridge.respond(&request_id, decision);
    Ok(())
}
