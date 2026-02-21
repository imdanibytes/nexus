use std::sync::Arc;

use crate::audit::writer::AuditWriter;
use crate::audit::{AuditActor, AuditEntry, AuditResult, AuditSeverity};
use crate::host_api::approval::{ApprovalBridge, ApprovalDecision};
use crate::permissions::{GrantedPermission, Permission};
use crate::AppState;

#[tauri::command]
pub async fn permission_grant(
    state: tauri::State<'_, AppState>,
    audit: tauri::State<'_, AuditWriter>,
    plugin_id: String,
    permissions: Vec<Permission>,
) -> Result<(), String> {
    let mgr = state.read().await;
    let perm_strs: Vec<String> = permissions.iter().map(|p| p.as_str().to_string()).collect();
    for perm in permissions {
        mgr.permissions
            .grant(&plugin_id, perm, None)
            .map_err(|e| e.to_string())?;
    }

    // Recompute authorization_details so the next token issuance picks up new permissions
    if let Some(client) = mgr.oauth_store.get_client_by_plugin_id(&plugin_id) {
        let grants = mgr.permissions.get_grants(&plugin_id);
        let details = crate::permissions::rar::build_authorization_details(&grants);
        mgr.oauth_store.set_plugin_auth_details(&client.client_id, details);
    }

    mgr.notify_tools_changed();
    audit.record(AuditEntry {
        actor: AuditActor::User, source_id: None, severity: AuditSeverity::Critical, action: "permission.grant".into(),
        subject: Some(plugin_id), result: AuditResult::Success,
        details: Some(serde_json::json!({"permissions": perm_strs})),
    });
    Ok(())
}

#[tauri::command]
pub async fn permission_revoke(
    state: tauri::State<'_, AppState>,
    audit: tauri::State<'_, AuditWriter>,
    plugin_id: String,
    permissions: Vec<Permission>,
) -> Result<(), String> {
    let perm_strs: Vec<String> = permissions.iter().map(|p| p.as_str().to_string()).collect();
    let mgr = state.read().await;
    for perm in &permissions {
        mgr.permissions
            .revoke(&plugin_id, perm)
            .map_err(|e| e.to_string())?;
    }

    // Revoke the plugin's OAuth tokens so it re-authenticates with a fresh
    // token that no longer carries the revoked permission.
    if let Some(client) = mgr.oauth_store.get_client_by_plugin_id(&plugin_id) {
        mgr.oauth_store.revoke_plugin_tokens(&client.client_id);
        // Recompute authorization_details without the revoked permission
        let grants = mgr.permissions.get_grants(&plugin_id);
        let details = crate::permissions::rar::build_authorization_details(&grants);
        mgr.oauth_store.set_plugin_auth_details(&client.client_id, details);
    }

    mgr.notify_tools_changed();
    audit.record(AuditEntry {
        actor: AuditActor::User, source_id: None, severity: AuditSeverity::Critical, action: "permission.revoke".into(),
        subject: Some(plugin_id), result: AuditResult::Success,
        details: Some(serde_json::json!({"permissions": perm_strs})),
    });
    Ok(())
}

#[tauri::command]
pub async fn permission_unrevoke(
    state: tauri::State<'_, AppState>,
    audit: tauri::State<'_, AuditWriter>,
    plugin_id: String,
    permissions: Vec<Permission>,
) -> Result<(), String> {
    let perm_strs: Vec<String> = permissions.iter().map(|p| p.as_str().to_string()).collect();
    let mgr = state.read().await;
    for perm in &permissions {
        mgr.permissions
            .unrevoke(&plugin_id, perm)
            .map_err(|e| e.to_string())?;
    }

    // Recompute authorization_details since we re-activated permissions
    if let Some(client) = mgr.oauth_store.get_client_by_plugin_id(&plugin_id) {
        let grants = mgr.permissions.get_grants(&plugin_id);
        let details = crate::permissions::rar::build_authorization_details(&grants);
        mgr.oauth_store.set_plugin_auth_details(&client.client_id, details);
    }

    mgr.notify_tools_changed();
    audit.record(AuditEntry {
        actor: AuditActor::User, source_id: None, severity: AuditSeverity::Critical, action: "permission.unrevoke".into(),
        subject: Some(plugin_id), result: AuditResult::Success,
        details: Some(serde_json::json!({"permissions": perm_strs})),
    });
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

#[tauri::command]
pub async fn permission_remove_path(
    state: tauri::State<'_, AppState>,
    plugin_id: String,
    permission: Permission,
    path: String,
) -> Result<(), String> {
    let mgr = state.read().await;
    mgr.permissions
        .remove_approved_scope(&plugin_id, &permission, &path)
        .map_err(|e| e.to_string())
}

/// Remove a scope value from an extension permission's approved_scopes.
#[tauri::command]
pub async fn permission_remove_scope(
    state: tauri::State<'_, AppState>,
    plugin_id: String,
    permission: Permission,
    scope: String,
) -> Result<(), String> {
    let mgr = state.read().await;
    mgr.permissions
        .remove_approved_scope(&plugin_id, &permission, &scope)
        .map_err(|e| e.to_string())
}

/// Called by the frontend approval dialog when the user makes a decision.
///
/// For `Approve` (persist): writes the approved scope to `PermissionService`
/// BEFORE sending the decision on the channel, guaranteeing the scope is
/// persisted by the time the HTTP handler resumes.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn runtime_approval_respond(
    state: tauri::State<'_, AppState>,
    bridge: tauri::State<'_, Arc<ApprovalBridge>>,
    audit: tauri::State<'_, AuditWriter>,
    request_id: String,
    decision: ApprovalDecision,
    plugin_id: String,
    category: String,
    context: std::collections::HashMap<String, String>,
) -> Result<(), String> {
    if decision == ApprovalDecision::Approve {
        if category == "deferred_permission" {
            // Deferred → Active: persist the state transition before signaling the channel
            let permission_str = context.get("permission").cloned().unwrap_or_default();
            let permission: Permission =
                serde_json::from_value(serde_json::Value::String(permission_str))
                    .map_err(|e| format!("invalid permission: {}", e))?;

            let mgr = state.read().await;
            mgr.permissions
                .activate(&plugin_id, &permission)
                .map_err(|e| e.to_string())?;
        } else if category == "filesystem" {
            // Filesystem scope: persist the parent directory
            if let Some(parent_dir) = context.get("parent_dir") {
                let permission_str = context.get("permission").cloned().unwrap_or_default();
                let permission: Permission =
                    serde_json::from_value(serde_json::Value::String(permission_str))
                        .map_err(|e| format!("invalid permission: {}", e))?;

                let mgr = state.read().await;
                mgr.permissions
                    .add_approved_scope(&plugin_id, &permission, parent_dir.clone())
                    .map_err(|e| e.to_string())?;
            }
        } else if category.starts_with("extension_scope:") {
            // Extension scope: persist the scope value
            if let Some(scope_value) = context.get("scope_value") {
                let permission_str = context.get("permission").cloned().unwrap_or_default();
                let permission: Permission =
                    serde_json::from_value(serde_json::Value::String(permission_str))
                        .map_err(|e| format!("invalid permission: {}", e))?;

                let mgr = state.read().await;
                mgr.permissions
                    .add_approved_scope(&plugin_id, &permission, scope_value.clone())
                    .map_err(|e| e.to_string())?;
            }
        }
    }

    let decision_str = format!("{:?}", decision);
    audit.record(AuditEntry {
        actor: AuditActor::User, source_id: None, severity: AuditSeverity::Critical, action: "permission.runtime_approval".into(),
        subject: Some(plugin_id), result: AuditResult::Success,
        details: Some(serde_json::json!({"decision": decision_str, "category": category})),
    });

    bridge.respond(&request_id, decision);
    Ok(())
}
