use std::path::Path;
use std::sync::Mutex;
use std::time::Duration;

use rusqlite::{params, Connection};

use super::{AuditEntry, AuditLogRow, AuditQuery};

/// SQLite-backed durable audit store.
///
/// All administrative actions (plugin lifecycle, permission changes, security
/// events, settings changes) are persisted here for observability and compliance.
/// The background writer inserts entries in batches; the frontend queries via
/// Tauri commands.
pub struct AuditStore {
    db: Mutex<Connection>,
}

impl AuditStore {
    /// Open (or create) the audit database in the given data directory.
    pub fn new(data_dir: &Path) -> Result<Self, String> {
        let db_path = data_dir.join("audit.db");
        let conn =
            Connection::open(&db_path).map_err(|e| format!("Failed to open audit store: {}", e))?;

        conn.execute_batch("PRAGMA journal_mode=WAL;")
            .map_err(|e| format!("Failed to set WAL mode: {}", e))?;

        // Create table (includes all columns for fresh databases).
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS audit_log (
                id         INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp  TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
                actor      TEXT    NOT NULL,
                source_id  TEXT,
                severity   TEXT    NOT NULL DEFAULT 'info',
                action     TEXT    NOT NULL,
                subject    TEXT,
                result     TEXT    NOT NULL DEFAULT 'success',
                details    TEXT
            );

            CREATE INDEX IF NOT EXISTS idx_audit_timestamp ON audit_log(timestamp);
            CREATE INDEX IF NOT EXISTS idx_audit_action    ON audit_log(action);
            CREATE INDEX IF NOT EXISTS idx_audit_actor     ON audit_log(actor);
            CREATE INDEX IF NOT EXISTS idx_audit_subject   ON audit_log(subject);
            ",
        )
        .map_err(|e| format!("Failed to initialize audit schema: {}", e))?;

        // Schema migration: add source_id + severity to databases created before
        // these columns existed. CREATE TABLE IF NOT EXISTS is a no-op on existing
        // tables, so we need ALTER TABLE to add missing columns.
        {
            let has_severity: bool = conn
                .prepare("SELECT severity FROM audit_log LIMIT 0")
                .is_ok();
            if !has_severity {
                conn.execute_batch(
                    "ALTER TABLE audit_log ADD COLUMN source_id TEXT;
                     ALTER TABLE audit_log ADD COLUMN severity TEXT NOT NULL DEFAULT 'info';",
                )
                .map_err(|e| format!("Migration error: {}", e))?;
                log::info!("Audit store: migrated schema (added source_id, severity)");
            }
        }

        // Create severity index after migration ensures the column exists.
        conn.execute_batch(
            "CREATE INDEX IF NOT EXISTS idx_audit_severity ON audit_log(severity);",
        )
        .map_err(|e| format!("Failed to create severity index: {}", e))?;

        Ok(Self {
            db: Mutex::new(conn),
        })
    }

    /// Insert a batch of audit entries in a single transaction.
    pub fn insert_batch(&self, entries: &[AuditEntry]) -> Result<(), String> {
        if entries.is_empty() {
            return Ok(());
        }

        let conn = self.db.lock().map_err(|e| format!("Lock error: {}", e))?;
        let tx = conn
            .unchecked_transaction()
            .map_err(|e| format!("Transaction error: {}", e))?;

        {
            let mut stmt = tx
                .prepare_cached(
                    "INSERT INTO audit_log (actor, source_id, severity, action, subject, result, details)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                )
                .map_err(|e| format!("Prepare error: {}", e))?;

            for entry in entries {
                let details_json = entry
                    .details
                    .as_ref()
                    .map(|v| serde_json::to_string(v).unwrap_or_default());

                stmt.execute(params![
                    entry.actor.as_str(),
                    entry.source_id,
                    entry.severity.as_str(),
                    entry.action,
                    entry.subject,
                    entry.result.as_str(),
                    details_json,
                ])
                .map_err(|e| format!("Insert error: {}", e))?;
            }
        }

        tx.commit()
            .map_err(|e| format!("Commit error: {}", e))?;
        Ok(())
    }

    /// Query the audit log with optional filters. Results ordered by timestamp DESC.
    pub fn query(&self, q: &AuditQuery) -> Result<Vec<AuditLogRow>, String> {
        let conn = self.db.lock().map_err(|e| format!("Lock error: {}", e))?;

        let mut sql = String::from("SELECT id, timestamp, actor, source_id, severity, action, subject, result, details FROM audit_log");
        let mut conditions: Vec<String> = Vec::new();
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(ref action) = q.action {
            if action.contains('*') {
                conditions.push(format!("action GLOB ?{}", param_values.len() + 1));
                param_values.push(Box::new(action.clone()));
            } else {
                conditions.push(format!("action = ?{}", param_values.len() + 1));
                param_values.push(Box::new(action.clone()));
            }
        }

        if let Some(ref actor) = q.actor {
            if actor.contains('*') {
                conditions.push(format!("actor GLOB ?{}", param_values.len() + 1));
                param_values.push(Box::new(actor.clone()));
            } else {
                conditions.push(format!("actor = ?{}", param_values.len() + 1));
                param_values.push(Box::new(actor.clone()));
            }
        }

        if let Some(ref source_id) = q.source_id {
            if source_id.contains('*') {
                conditions.push(format!("source_id GLOB ?{}", param_values.len() + 1));
                param_values.push(Box::new(source_id.clone()));
            } else {
                conditions.push(format!("source_id = ?{}", param_values.len() + 1));
                param_values.push(Box::new(source_id.clone()));
            }
        }

        if let Some(ref severity) = q.severity {
            conditions.push(format!("severity = ?{}", param_values.len() + 1));
            param_values.push(Box::new(severity.clone()));
        }

        if let Some(ref subject) = q.subject {
            if subject.contains('*') {
                conditions.push(format!("subject GLOB ?{}", param_values.len() + 1));
                param_values.push(Box::new(subject.clone()));
            } else {
                conditions.push(format!("subject = ?{}", param_values.len() + 1));
                param_values.push(Box::new(subject.clone()));
            }
        }

        if let Some(ref result) = q.result {
            conditions.push(format!("result = ?{}", param_values.len() + 1));
            param_values.push(Box::new(result.clone()));
        }

        if let Some(ref since) = q.since {
            conditions.push(format!("timestamp >= ?{}", param_values.len() + 1));
            param_values.push(Box::new(since.clone()));
        }

        if let Some(ref until) = q.until {
            conditions.push(format!("timestamp <= ?{}", param_values.len() + 1));
            param_values.push(Box::new(until.clone()));
        }

        if !conditions.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&conditions.join(" AND "));
        }

        sql.push_str(" ORDER BY timestamp DESC");

        let limit = q.limit.unwrap_or(100).min(10_000);
        let offset = q.offset.unwrap_or(0);
        sql.push_str(&format!(" LIMIT {} OFFSET {}", limit, offset));

        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| format!("Query prepare error: {}", e))?;

        let rows = stmt
            .query_map(params_refs.as_slice(), |row| {
                let details_str: Option<String> = row.get(8)?;
                let details = details_str
                    .and_then(|s| serde_json::from_str(&s).ok());

                Ok(AuditLogRow {
                    id: row.get(0)?,
                    timestamp: row.get(1)?,
                    actor: row.get(2)?,
                    source_id: row.get(3)?,
                    severity: row.get::<_, Option<String>>(4)?.unwrap_or_else(|| "info".into()),
                    action: row.get(5)?,
                    subject: row.get(6)?,
                    result: row.get(7)?,
                    details,
                })
            })
            .map_err(|e| format!("Query error: {}", e))?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(|e| format!("Row error: {}", e))?);
        }

        Ok(results)
    }

    /// Count audit entries matching the given filters.
    pub fn count(&self, q: &AuditQuery) -> Result<usize, String> {
        let conn = self.db.lock().map_err(|e| format!("Lock error: {}", e))?;

        let mut sql = String::from("SELECT COUNT(*) FROM audit_log");
        let mut conditions: Vec<String> = Vec::new();
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(ref action) = q.action {
            if action.contains('*') {
                conditions.push(format!("action GLOB ?{}", param_values.len() + 1));
                param_values.push(Box::new(action.clone()));
            } else {
                conditions.push(format!("action = ?{}", param_values.len() + 1));
                param_values.push(Box::new(action.clone()));
            }
        }

        if let Some(ref actor) = q.actor {
            if actor.contains('*') {
                conditions.push(format!("actor GLOB ?{}", param_values.len() + 1));
                param_values.push(Box::new(actor.clone()));
            } else {
                conditions.push(format!("actor = ?{}", param_values.len() + 1));
                param_values.push(Box::new(actor.clone()));
            }
        }

        if let Some(ref source_id) = q.source_id {
            if source_id.contains('*') {
                conditions.push(format!("source_id GLOB ?{}", param_values.len() + 1));
                param_values.push(Box::new(source_id.clone()));
            } else {
                conditions.push(format!("source_id = ?{}", param_values.len() + 1));
                param_values.push(Box::new(source_id.clone()));
            }
        }

        if let Some(ref severity) = q.severity {
            conditions.push(format!("severity = ?{}", param_values.len() + 1));
            param_values.push(Box::new(severity.clone()));
        }

        if let Some(ref subject) = q.subject {
            if subject.contains('*') {
                conditions.push(format!("subject GLOB ?{}", param_values.len() + 1));
                param_values.push(Box::new(subject.clone()));
            } else {
                conditions.push(format!("subject = ?{}", param_values.len() + 1));
                param_values.push(Box::new(subject.clone()));
            }
        }

        if let Some(ref result) = q.result {
            conditions.push(format!("result = ?{}", param_values.len() + 1));
            param_values.push(Box::new(result.clone()));
        }

        if let Some(ref since) = q.since {
            conditions.push(format!("timestamp >= ?{}", param_values.len() + 1));
            param_values.push(Box::new(since.clone()));
        }

        if let Some(ref until) = q.until {
            conditions.push(format!("timestamp <= ?{}", param_values.len() + 1));
            param_values.push(Box::new(until.clone()));
        }

        if !conditions.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&conditions.join(" AND "));
        }

        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();

        let count: usize = conn
            .query_row(&sql, params_refs.as_slice(), |row| row.get(0))
            .map_err(|e| format!("Count error: {}", e))?;

        Ok(count)
    }

    /// Delete audit entries older than the given TTL. Returns the number of deleted rows.
    pub fn cleanup_old(&self, ttl: Duration) -> Result<usize, String> {
        let conn = self.db.lock().map_err(|e| format!("Lock error: {}", e))?;
        let cutoff = chrono::Utc::now() - chrono::Duration::from_std(ttl).unwrap_or_default();
        let cutoff_str = cutoff.to_rfc3339();

        let deleted = conn
            .execute(
                "DELETE FROM audit_log WHERE timestamp < ?1",
                params![cutoff_str],
            )
            .map_err(|e| format!("Cleanup error: {}", e))?;

        Ok(deleted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::{AuditActor, AuditResult, AuditSeverity};

    fn temp_store() -> AuditStore {
        let dir = tempfile::tempdir().unwrap();
        AuditStore::new(dir.path()).unwrap()
    }

    #[test]
    fn insert_and_query() {
        let store = temp_store();

        let entries = vec![
            AuditEntry {
                actor: AuditActor::User,
                source_id: None,
                severity: AuditSeverity::Warn,
                action: "plugin.install".into(),
                subject: Some("com.test.plugin".into()),
                result: AuditResult::Success,
                details: Some(serde_json::json!({"version": "1.0.0"})),
            },
            AuditEntry {
                actor: AuditActor::System,
                source_id: None,
                severity: AuditSeverity::Info,
                action: "extension.enable".into(),
                subject: Some("codebase-indexer".into()),
                result: AuditResult::Success,
                details: None,
            },
        ];

        store.insert_batch(&entries).unwrap();

        let all = store.query(&AuditQuery::default()).unwrap();
        assert_eq!(all.len(), 2);
        // Most recent first
        assert_eq!(all[0].action, "extension.enable");
        assert_eq!(all[0].severity, "info");
        assert_eq!(all[1].action, "plugin.install");
        assert_eq!(all[1].severity, "warn");
    }

    #[test]
    fn query_filters() {
        let store = temp_store();

        let entries = vec![
            AuditEntry {
                actor: AuditActor::User,
                source_id: None,
                severity: AuditSeverity::Warn,
                action: "plugin.install".into(),
                subject: Some("com.test.a".into()),
                result: AuditResult::Success,
                details: None,
            },
            AuditEntry {
                actor: AuditActor::System,
                source_id: None,
                severity: AuditSeverity::Info,
                action: "plugin.start".into(),
                subject: Some("com.test.a".into()),
                result: AuditResult::Failure,
                details: Some(serde_json::json!({"error": "timeout"})),
            },
            AuditEntry {
                actor: AuditActor::User,
                source_id: None,
                severity: AuditSeverity::Critical,
                action: "permission.grant".into(),
                subject: Some("com.test.b".into()),
                result: AuditResult::Success,
                details: None,
            },
        ];

        store.insert_batch(&entries).unwrap();

        // Filter by action glob
        let q = AuditQuery {
            action: Some("plugin.*".into()),
            ..Default::default()
        };
        assert_eq!(store.query(&q).unwrap().len(), 2);

        // Filter by result
        let q = AuditQuery {
            result: Some("failure".into()),
            ..Default::default()
        };
        let rows = store.query(&q).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].action, "plugin.start");

        // Filter by actor
        let q = AuditQuery {
            actor: Some("system".into()),
            ..Default::default()
        };
        assert_eq!(store.query(&q).unwrap().len(), 1);

        // Filter by severity
        let q = AuditQuery {
            severity: Some("critical".into()),
            ..Default::default()
        };
        let rows = store.query(&q).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].action, "permission.grant");

        // Count
        let q = AuditQuery {
            action: Some("plugin.*".into()),
            ..Default::default()
        };
        assert_eq!(store.count(&q).unwrap(), 2);
    }

    #[test]
    fn pagination() {
        let store = temp_store();

        let entries: Vec<AuditEntry> = (0..10)
            .map(|i| AuditEntry {
                actor: AuditActor::User,
                source_id: None,
                severity: AuditSeverity::Info,
                action: format!("test.action.{}", i),
                subject: None,
                result: AuditResult::Success,
                details: None,
            })
            .collect();

        store.insert_batch(&entries).unwrap();

        let q = AuditQuery {
            limit: Some(3),
            offset: Some(0),
            ..Default::default()
        };
        assert_eq!(store.query(&q).unwrap().len(), 3);

        let q = AuditQuery {
            limit: Some(3),
            offset: Some(8),
            ..Default::default()
        };
        assert_eq!(store.query(&q).unwrap().len(), 2);

        assert_eq!(store.count(&AuditQuery::default()).unwrap(), 10);
    }

    #[test]
    fn cleanup_old() {
        let store = temp_store();

        // Insert an entry with a backdated timestamp so TTL cleanup can find it
        {
            let conn = store.db.lock().unwrap();
            conn.execute(
                "INSERT INTO audit_log (timestamp, actor, action, result) VALUES (?1, 'user', 'test.action', 'success')",
                params![
                    (chrono::Utc::now() - chrono::Duration::hours(1)).to_rfc3339()
                ],
            ).unwrap();
        }
        assert_eq!(store.count(&AuditQuery::default()).unwrap(), 1);

        // Cleanup with 30-minute TTL should remove the 1-hour-old entry
        let deleted = store.cleanup_old(Duration::from_secs(1800)).unwrap();
        assert_eq!(deleted, 1);
        assert_eq!(store.count(&AuditQuery::default()).unwrap(), 0);
    }

    #[test]
    fn empty_batch() {
        let store = temp_store();
        store.insert_batch(&[]).unwrap();
        assert_eq!(store.count(&AuditQuery::default()).unwrap(), 0);
    }
}
