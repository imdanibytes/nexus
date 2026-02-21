use crate::audit::writer::AuditWriter;
use crate::audit::{AuditActor, AuditEntry, AuditResult, AuditSeverity};
use crate::plugin_manager::registry::{RegistryKind, RegistrySource, RegistryTrust};
use crate::AppState;

#[tauri::command]
pub async fn registry_list(state: tauri::State<'_, AppState>) -> Result<Vec<RegistrySource>, String> {
    let mgr = state.read().await;
    Ok(mgr.registry_store.list().to_vec())
}

#[tauri::command]
pub async fn registry_add(
    state: tauri::State<'_, AppState>,
    audit: tauri::State<'_, AuditWriter>,
    name: String,
    kind: String,
    url: String,
) -> Result<RegistrySource, String> {
    let mut mgr = state.write().await;

    let registry_kind = match kind.as_str() {
        "remote" => RegistryKind::Remote,
        "local" => RegistryKind::Local,
        _ => return Err(format!("Invalid registry kind: {}. Use 'remote' or 'local'", kind)),
    };

    // Generate a slug-style ID from the name
    let id = name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string();

    let source = RegistrySource {
        id,
        name,
        kind: registry_kind,
        url,
        enabled: true,
        trust: RegistryTrust::Community,
    };

    mgr.registry_store.add(source.clone()).map_err(|e| e.to_string())?;
    audit.record(AuditEntry {
        actor: AuditActor::User, source_id: None, severity: AuditSeverity::Info, action: "settings.registry.add".into(),
        subject: Some(source.id.clone()), result: AuditResult::Success,
        details: Some(serde_json::json!({"name": source.name, "url": source.url})),
    });
    Ok(source)
}

/// Built-in registries that cannot be removed by the user.
const PROTECTED_REGISTRIES: &[&str] = &["nexus-community", "nexus-mcp-local"];

#[tauri::command]
pub async fn registry_remove(
    state: tauri::State<'_, AppState>,
    audit: tauri::State<'_, AuditWriter>,
    id: String,
) -> Result<(), String> {
    if PROTECTED_REGISTRIES.contains(&id.as_str()) {
        return Err(format!("Cannot remove built-in registry '{}'", id));
    }
    let mut mgr = state.write().await;
    mgr.registry_store.remove(&id).map_err(|e| e.to_string())?;
    audit.record(AuditEntry {
        actor: AuditActor::User, source_id: None, severity: AuditSeverity::Info, action: "settings.registry.remove".into(),
        subject: Some(id), result: AuditResult::Success,
        details: None,
    });
    Ok(())
}

#[tauri::command]
pub async fn registry_toggle(
    state: tauri::State<'_, AppState>,
    audit: tauri::State<'_, AuditWriter>,
    id: String,
    enabled: bool,
) -> Result<(), String> {
    let mut mgr = state.write().await;
    mgr.registry_store.toggle(&id, enabled).map_err(|e| e.to_string())?;
    audit.record(AuditEntry {
        actor: AuditActor::User, source_id: None, severity: AuditSeverity::Info, action: "settings.registry.toggle".into(),
        subject: Some(id), result: AuditResult::Success,
        details: Some(serde_json::json!({"enabled": enabled})),
    });
    Ok(())
}
