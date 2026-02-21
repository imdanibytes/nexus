use std::sync::Arc;

use crate::audit::writer::AuditWriter;
use crate::audit::{AuditActor, AuditEntry, AuditResult, AuditSeverity};
use crate::oauth::store::OAuthStore;
use crate::oauth::types::OAuthClientInfo;

#[tauri::command]
pub async fn oauth_list_clients(
    store: tauri::State<'_, Arc<OAuthStore>>,
) -> Result<Vec<OAuthClientInfo>, String> {
    Ok(store.list_clients().iter().map(OAuthClientInfo::from).collect())
}

#[tauri::command]
pub async fn oauth_revoke_client(
    store: tauri::State<'_, Arc<OAuthStore>>,
    audit: tauri::State<'_, AuditWriter>,
    client_id: String,
) -> Result<(), String> {
    store.revoke_client(&client_id);
    audit.record(AuditEntry {
        actor: AuditActor::User, source_id: None, severity: AuditSeverity::Critical, action: "security.oauth.revoke_client".into(),
        subject: Some(client_id), result: AuditResult::Success,
        details: None,
    });
    Ok(())
}
