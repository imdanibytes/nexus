use crate::api_keys::{ApiKey, ApiKeyStore};
use crate::audit::writer::AuditWriter;
use crate::audit::{AuditActor, AuditEntry, AuditResult, AuditSeverity};
use serde::Serialize;

#[derive(Serialize)]
pub struct GeneratedApiKey {
    pub key: ApiKey,
    pub raw: String,
}

#[tauri::command]
pub async fn api_key_list(
    store: tauri::State<'_, ApiKeyStore>,
) -> Result<Vec<ApiKey>, String> {
    Ok(store.list())
}

#[tauri::command]
pub async fn api_key_generate(
    store: tauri::State<'_, ApiKeyStore>,
    audit: tauri::State<'_, AuditWriter>,
    name: String,
) -> Result<GeneratedApiKey, String> {
    let (key, raw) = store.generate(&name);
    audit.record(AuditEntry {
        actor: AuditActor::User, source_id: None, severity: AuditSeverity::Critical, action: "security.api_key.generate".into(),
        subject: Some(key.id.clone()), result: AuditResult::Success,
        details: Some(serde_json::json!({"name": name})),
    });
    Ok(GeneratedApiKey { key, raw })
}

#[tauri::command]
pub async fn api_key_revoke(
    store: tauri::State<'_, ApiKeyStore>,
    audit: tauri::State<'_, AuditWriter>,
    id: String,
) -> Result<(), String> {
    if store.revoke(&id) {
        audit.record(AuditEntry {
            actor: AuditActor::User, source_id: None, severity: AuditSeverity::Critical, action: "security.api_key.revoke".into(),
            subject: Some(id), result: AuditResult::Success,
            details: None,
        });
        Ok(())
    } else {
        Err("API key not found".into())
    }
}

#[tauri::command]
pub async fn api_key_get_default(
    store: tauri::State<'_, ApiKeyStore>,
) -> Result<Option<String>, String> {
    Ok(store.get_default_raw())
}

#[tauri::command]
pub async fn api_key_regenerate_default(
    store: tauri::State<'_, ApiKeyStore>,
    audit: tauri::State<'_, AuditWriter>,
) -> Result<GeneratedApiKey, String> {
    let (key, raw) = store.regenerate_default();
    audit.record(AuditEntry {
        actor: AuditActor::User, source_id: None, severity: AuditSeverity::Critical, action: "security.api_key.regenerate_default".into(),
        subject: Some(key.id.clone()), result: AuditResult::Success,
        details: None,
    });
    Ok(GeneratedApiKey { key, raw })
}
