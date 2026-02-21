use std::path::Path;
use std::sync::Mutex;
use std::time::Duration;

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use super::cloud_event::CloudEvent;
use super::routing::RouteAction;

/// A row from the `deliveries` table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryRow {
    pub id: i64,
    pub event_id: String,
    pub action: RouteAction,
    pub status: String,
    pub attempts: i32,
    pub last_error: Option<String>,
}

/// SQLite-backed durable event store with delivery tracking.
///
/// Events are persisted when published. Each matching route action becomes a
/// delivery row with status tracking and retry metadata. A background worker
/// claims pending deliveries, executes them, and marks them completed or
/// moves them to dead letter after max attempts.
pub struct EventStore {
    db: Mutex<Connection>,
}

impl EventStore {
    /// Open (or create) the event store database in the given data directory.
    pub fn new(data_dir: &Path) -> Result<Self, String> {
        let db_path = data_dir.join("event_store.db");
        let conn =
            Connection::open(&db_path).map_err(|e| format!("Failed to open event store: {}", e))?;

        // WAL mode for better concurrent read/write performance
        conn.execute_batch("PRAGMA journal_mode=WAL;")
            .map_err(|e| format!("Failed to set WAL mode: {}", e))?;

        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS events (
                id          TEXT PRIMARY KEY,
                source      TEXT NOT NULL,
                event_type  TEXT NOT NULL,
                subject     TEXT,
                data        TEXT NOT NULL,
                time        TEXT NOT NULL,
                created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
            );

            CREATE TABLE IF NOT EXISTS deliveries (
                id           INTEGER PRIMARY KEY AUTOINCREMENT,
                event_id     TEXT NOT NULL REFERENCES events(id),
                action       TEXT NOT NULL,
                status       TEXT NOT NULL DEFAULT 'pending',
                attempts     INTEGER NOT NULL DEFAULT 0,
                max_attempts INTEGER NOT NULL DEFAULT 5,
                next_retry   TEXT NOT NULL,
                last_error   TEXT,
                created_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
                updated_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
            );

            CREATE INDEX IF NOT EXISTS idx_deliveries_status_retry
                ON deliveries(status, next_retry);
            CREATE INDEX IF NOT EXISTS idx_deliveries_event_id
                ON deliveries(event_id);
            CREATE INDEX IF NOT EXISTS idx_events_created_at
                ON events(created_at);
            ",
        )
        .map_err(|e| format!("Failed to initialize event store schema: {}", e))?;

        Ok(Self {
            db: Mutex::new(conn),
        })
    }

    /// Persist a CloudEvent to the events table.
    pub fn insert_event(&self, event: &CloudEvent) -> Result<(), String> {
        let db = self.db.lock().map_err(|e| e.to_string())?;
        let data_json =
            serde_json::to_string(&event.data).map_err(|e| format!("Serialize data: {}", e))?;
        let time_str = event.time.to_rfc3339();

        db.execute(
            "INSERT OR IGNORE INTO events (id, source, event_type, subject, data, time)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                event.id,
                event.source,
                event.event_type,
                event.subject,
                data_json,
                time_str,
            ],
        )
        .map_err(|e| format!("Insert event: {}", e))?;

        Ok(())
    }

    /// Create delivery rows for each route action, all with status=pending and next_retry=now.
    pub fn insert_deliveries(
        &self,
        event_id: &str,
        actions: Vec<RouteAction>,
    ) -> Result<(), String> {
        let db = self.db.lock().map_err(|e| e.to_string())?;
        let now = Utc::now().to_rfc3339();

        for action in &actions {
            let action_json = serde_json::to_string(action)
                .map_err(|e| format!("Serialize action: {}", e))?;
            db.execute(
                "INSERT INTO deliveries (event_id, action, status, next_retry)
                 VALUES (?1, ?2, 'pending', ?3)",
                params![event_id, action_json, now],
            )
            .map_err(|e| format!("Insert delivery: {}", e))?;
        }

        Ok(())
    }

    /// Atomically claim a batch of pending deliveries that are ready for retry.
    /// Sets their status to `in_flight` and returns them.
    pub fn claim_ready(&self, batch_size: usize) -> Result<Vec<(DeliveryRow, String)>, String> {
        let db = self.db.lock().map_err(|e| e.to_string())?;
        let now = Utc::now().to_rfc3339();

        // Select ready deliveries
        let mut stmt = db
            .prepare(
                "SELECT d.id, d.event_id, d.action, d.status, d.attempts, d.last_error,
                        e.source, e.event_type, e.subject, e.data, e.time
                 FROM deliveries d
                 JOIN events e ON e.id = d.event_id
                 WHERE d.status = 'pending' AND d.next_retry <= ?1
                 ORDER BY d.next_retry ASC
                 LIMIT ?2",
            )
            .map_err(|e| format!("Prepare claim: {}", e))?;

        let rows: Vec<(DeliveryRow, String)> = stmt
            .query_map(params![now, batch_size as i64], |row| {
                let action_json: String = row.get(2)?;
                let data_json: String = row.get(9)?;
                let time_str: String = row.get(10)?;

                // Reconstruct a minimal CloudEvent JSON for the executor
                let event_json = serde_json::json!({
                    "specversion": "1.0",
                    "id": row.get::<_, String>(1)?,
                    "source": row.get::<_, String>(6)?,
                    "type": row.get::<_, String>(7)?,
                    "subject": row.get::<_, Option<String>>(8)?,
                    "time": time_str,
                    "data": serde_json::from_str::<serde_json::Value>(&data_json)
                        .unwrap_or(serde_json::Value::Null),
                    "datacontenttype": "application/json",
                })
                .to_string();

                Ok((
                    DeliveryRow {
                        id: row.get(0)?,
                        event_id: row.get(1)?,
                        action: serde_json::from_str(&action_json).unwrap_or_else(|_| {
                            RouteAction::EmitFrontend {
                                channel: "__invalid__".into(),
                            }
                        }),
                        status: "in_flight".to_string(),
                        attempts: row.get(4)?,
                        last_error: row.get(5)?,
                    },
                    event_json,
                ))
            })
            .map_err(|e| format!("Query claim: {}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Collect claim: {}", e))?;

        // Mark claimed rows as in_flight
        for (delivery, _) in &rows {
            db.execute(
                "UPDATE deliveries SET status = 'in_flight', updated_at = ?1 WHERE id = ?2",
                params![now, delivery.id],
            )
            .map_err(|e| format!("Mark in_flight: {}", e))?;
        }

        Ok(rows)
    }

    /// Mark a delivery as successfully completed.
    pub fn mark_completed(&self, delivery_id: i64) -> Result<(), String> {
        let db = self.db.lock().map_err(|e| e.to_string())?;
        let now = Utc::now().to_rfc3339();
        db.execute(
            "UPDATE deliveries SET status = 'completed', updated_at = ?1 WHERE id = ?2",
            params![now, delivery_id],
        )
        .map_err(|e| format!("Mark completed: {}", e))?;
        Ok(())
    }

    /// Mark a delivery as failed. Increments attempt count, computes next retry
    /// with exponential backoff + jitter, or moves to dead_letter if max attempts reached.
    pub fn mark_failed(&self, delivery_id: i64, error: &str) -> Result<(), String> {
        let db = self.db.lock().map_err(|e| e.to_string())?;
        let now = Utc::now();
        let now_str = now.to_rfc3339();

        // Get current attempts and max_attempts
        let (attempts, max_attempts): (i32, i32) = db
            .query_row(
                "SELECT attempts, max_attempts FROM deliveries WHERE id = ?1",
                params![delivery_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|e| format!("Query delivery: {}", e))?;

        let new_attempts = attempts + 1;

        if new_attempts >= max_attempts {
            // Move to dead letter
            db.execute(
                "UPDATE deliveries SET status = 'dead_letter', attempts = ?1, last_error = ?2, updated_at = ?3
                 WHERE id = ?4",
                params![new_attempts, error, now_str, delivery_id],
            )
            .map_err(|e| format!("Mark dead_letter: {}", e))?;
        } else {
            // Compute next retry with exponential backoff + jitter
            let backoff_secs = std::cmp::min(2_i64.pow(new_attempts as u32), 300);
            let jitter_max = std::cmp::min(2_i64.pow(new_attempts as u32), 30);
            let jitter = if jitter_max > 0 {
                rand::random_range(0..=jitter_max)
            } else {
                0
            };
            let delay = Duration::from_secs((backoff_secs + jitter) as u64);
            let next_retry: DateTime<Utc> = now + delay;
            let next_retry_str = next_retry.to_rfc3339();

            db.execute(
                "UPDATE deliveries SET status = 'pending', attempts = ?1, last_error = ?2,
                 next_retry = ?3, updated_at = ?4
                 WHERE id = ?5",
                params![new_attempts, error, next_retry_str, now_str, delivery_id],
            )
            .map_err(|e| format!("Mark failed: {}", e))?;
        }

        Ok(())
    }

    /// Delete events and deliveries older than the given TTL.
    /// Returns the number of events purged.
    pub fn cleanup_old(&self, ttl: Duration) -> Result<usize, String> {
        let db = self.db.lock().map_err(|e| e.to_string())?;
        let cutoff = (Utc::now() - ttl).to_rfc3339();

        // Delete deliveries first (foreign key)
        db.execute(
            "DELETE FROM deliveries WHERE event_id IN (SELECT id FROM events WHERE created_at < ?1)",
            params![cutoff],
        )
        .map_err(|e| format!("Cleanup deliveries: {}", e))?;

        let deleted = db
            .execute("DELETE FROM events WHERE created_at < ?1", params![cutoff])
            .map_err(|e| format!("Cleanup events: {}", e))?;

        Ok(deleted)
    }

    /// Count deliveries in the dead_letter state.
    pub fn dead_letter_count(&self) -> Result<usize, String> {
        let db = self.db.lock().map_err(|e| e.to_string())?;
        let count: i64 = db
            .query_row(
                "SELECT COUNT(*) FROM deliveries WHERE status = 'dead_letter'",
                [],
                |row| row.get(0),
            )
            .map_err(|e| format!("Count dead letters: {}", e))?;
        Ok(count as usize)
    }

    /// Query dead-lettered deliveries for inspection.
    pub fn query_dead_letters(&self, limit: usize) -> Result<Vec<DeliveryRow>, String> {
        let db = self.db.lock().map_err(|e| e.to_string())?;
        let mut stmt = db
            .prepare(
                "SELECT id, event_id, action, status, attempts, last_error
                 FROM deliveries WHERE status = 'dead_letter'
                 ORDER BY updated_at DESC LIMIT ?1",
            )
            .map_err(|e| format!("Prepare dead letters: {}", e))?;

        let rows = stmt
            .query_map(params![limit as i64], |row| {
                let action_json: String = row.get(2)?;
                Ok(DeliveryRow {
                    id: row.get(0)?,
                    event_id: row.get(1)?,
                    action: serde_json::from_str(&action_json).unwrap_or_else(|_| {
                        RouteAction::EmitFrontend {
                            channel: "__invalid__".into(),
                        }
                    }),
                    status: row.get(3)?,
                    attempts: row.get(4)?,
                    last_error: row.get(5)?,
                })
            })
            .map_err(|e| format!("Query dead letters: {}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Collect dead letters: {}", e))?;

        Ok(rows)
    }

    /// Reconstruct a CloudEvent from a JSON string (used by the retry worker).
    pub fn parse_event_json(json: &str) -> Result<CloudEvent, String> {
        serde_json::from_str(json).map_err(|e| format!("Parse event JSON: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_store() -> (TempDir, EventStore) {
        let tmp = TempDir::new().unwrap();
        let store = EventStore::new(tmp.path()).unwrap();
        (tmp, store)
    }

    fn make_event(id: &str, event_type: &str) -> CloudEvent {
        CloudEvent {
            specversion: "1.0".to_string(),
            id: id.to_string(),
            source: "nexus://test".to_string(),
            event_type: event_type.to_string(),
            time: Utc::now(),
            subject: None,
            datacontenttype: "application/json".to_string(),
            data: serde_json::json!({"key": "value"}),
            extensions: Default::default(),
        }
    }

    #[test]
    fn insert_and_claim() {
        let (_tmp, store) = make_store();
        let event = make_event("evt1", "test.event");

        store.insert_event(&event).unwrap();
        store
            .insert_deliveries(
                "evt1",
                vec![RouteAction::EmitFrontend {
                    channel: "ch".into(),
                }],
            )
            .unwrap();

        let claimed = store.claim_ready(10).unwrap();
        assert_eq!(claimed.len(), 1);
        assert_eq!(claimed[0].0.event_id, "evt1");
        assert_eq!(claimed[0].0.status, "in_flight");

        // Second claim should return nothing (already in_flight)
        let claimed2 = store.claim_ready(10).unwrap();
        assert!(claimed2.is_empty());
    }

    #[test]
    fn mark_completed_works() {
        let (_tmp, store) = make_store();
        let event = make_event("evt2", "test.event");

        store.insert_event(&event).unwrap();
        store
            .insert_deliveries(
                "evt2",
                vec![RouteAction::EmitFrontend {
                    channel: "ch".into(),
                }],
            )
            .unwrap();

        let claimed = store.claim_ready(10).unwrap();
        store.mark_completed(claimed[0].0.id).unwrap();

        // Should not be claimable again
        let claimed2 = store.claim_ready(10).unwrap();
        assert!(claimed2.is_empty());
    }

    #[test]
    fn mark_failed_retries_then_dead_letters() {
        let (_tmp, store) = make_store();
        let event = make_event("evt3", "test.event");

        store.insert_event(&event).unwrap();
        store
            .insert_deliveries(
                "evt3",
                vec![RouteAction::EmitFrontend {
                    channel: "ch".into(),
                }],
            )
            .unwrap();

        // Simulate 5 failures (max_attempts = 5)
        for i in 0..5 {
            // Manually set next_retry to past so claim_ready picks it up
            {
                let db = store.db.lock().unwrap();
                let past = (Utc::now() - Duration::from_secs(3600)).to_rfc3339();
                db.execute(
                    "UPDATE deliveries SET next_retry = ?1, status = 'pending' WHERE event_id = 'evt3'",
                    params![past],
                )
                .unwrap();
            }

            let claimed = store.claim_ready(10).unwrap();
            assert_eq!(claimed.len(), 1, "Attempt {} should claim", i);
            store
                .mark_failed(claimed[0].0.id, &format!("error {}", i))
                .unwrap();
        }

        // After 5 failures, should be dead-lettered
        assert_eq!(store.dead_letter_count().unwrap(), 1);

        let dead = store.query_dead_letters(10).unwrap();
        assert_eq!(dead.len(), 1);
        assert_eq!(dead[0].status, "dead_letter");
        assert_eq!(dead[0].attempts, 5);
    }

    #[test]
    fn cleanup_old_removes_expired() {
        let (_tmp, store) = make_store();
        let event = make_event("evt4", "test.event");

        store.insert_event(&event).unwrap();
        store
            .insert_deliveries(
                "evt4",
                vec![RouteAction::EmitFrontend {
                    channel: "ch".into(),
                }],
            )
            .unwrap();

        // Backdate the event
        {
            let db = store.db.lock().unwrap();
            let old = (Utc::now() - Duration::from_secs(8 * 86400)).to_rfc3339();
            db.execute(
                "UPDATE events SET created_at = ?1 WHERE id = 'evt4'",
                params![old],
            )
            .unwrap();
        }

        let purged = store.cleanup_old(Duration::from_secs(7 * 86400)).unwrap();
        assert_eq!(purged, 1);
    }

    #[test]
    fn duplicate_event_insert_ignored() {
        let (_tmp, store) = make_store();
        let event = make_event("evt5", "test.event");

        store.insert_event(&event).unwrap();
        // Second insert should not error (INSERT OR IGNORE)
        store.insert_event(&event).unwrap();
    }
}
