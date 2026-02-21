use std::sync::Arc;

use crate::audit::store::AuditStore;
use crate::audit::{AuditLogRow, AuditQuery};

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn audit_query(
    store: tauri::State<'_, Arc<AuditStore>>,
    action: Option<String>,
    actor: Option<String>,
    source_id: Option<String>,
    severity: Option<String>,
    subject: Option<String>,
    result: Option<String>,
    since: Option<String>,
    until: Option<String>,
    limit: Option<usize>,
    offset: Option<usize>,
) -> Result<Vec<AuditLogRow>, String> {
    let q = AuditQuery {
        action,
        actor,
        source_id,
        severity,
        subject,
        result,
        since,
        until,
        limit,
        offset,
    };
    store.query(&q)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn audit_count(
    store: tauri::State<'_, Arc<AuditStore>>,
    action: Option<String>,
    actor: Option<String>,
    source_id: Option<String>,
    severity: Option<String>,
    subject: Option<String>,
    result: Option<String>,
    since: Option<String>,
    until: Option<String>,
) -> Result<usize, String> {
    let q = AuditQuery {
        action,
        actor,
        source_id,
        severity,
        subject,
        result,
        since,
        until,
        limit: None,
        offset: None,
    };
    store.count(&q)
}

/// Export audit entries as a JSON string (for download).
#[tauri::command]
pub async fn audit_export(
    store: tauri::State<'_, Arc<AuditStore>>,
    since: Option<String>,
    until: Option<String>,
) -> Result<String, String> {
    let q = AuditQuery {
        since,
        until,
        limit: Some(10_000),
        ..Default::default()
    };
    let rows = store.query(&q)?;
    serde_json::to_string_pretty(&rows).map_err(|e| e.to_string())
}
