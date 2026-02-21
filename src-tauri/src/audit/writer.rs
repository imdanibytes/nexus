use std::sync::Arc;
use std::time::Duration;

use tokio::sync::mpsc;
use tokio::time::interval;

use super::store::AuditStore;
use super::AuditEntry;

const CHANNEL_CAPACITY: usize = 1024;
const BATCH_SIZE: usize = 50;
const FLUSH_INTERVAL: Duration = Duration::from_millis(500);
const CLEANUP_INTERVAL: Duration = Duration::from_secs(3600);
const DEFAULT_TTL: Duration = Duration::from_secs(30 * 86400); // 30 days

/// Cheaply cloneable handle for recording audit entries.
///
/// Callers use `record()` to push entries into a bounded channel.
/// A background task drains the channel and batch-inserts into SQLite.
#[derive(Clone)]
pub struct AuditWriter {
    tx: mpsc::Sender<AuditEntry>,
}

impl AuditWriter {
    /// Record an audit entry. Non-blocking — drops the entry if the channel is full.
    pub fn record(&self, entry: AuditEntry) {
        if self.tx.try_send(entry).is_err() {
            log::warn!("Audit channel full, entry dropped");
        }
    }
}

/// Create the background writer and return the AuditWriter handle.
///
/// The caller is responsible for spawning `run()` with `tauri::async_runtime::spawn`.
/// This returns both the writer handle and the future to spawn.
pub fn create(store: Arc<AuditStore>) -> (AuditWriter, impl std::future::Future<Output = ()>) {
    let (tx, rx) = mpsc::channel(CHANNEL_CAPACITY);
    let writer = AuditWriter { tx };
    let future = run(rx, store);
    (writer, future)
}

async fn run(mut rx: mpsc::Receiver<AuditEntry>, store: Arc<AuditStore>) {
    let mut buffer: Vec<AuditEntry> = Vec::with_capacity(BATCH_SIZE);
    let mut flush_tick = interval(FLUSH_INTERVAL);
    let mut cleanup_tick = interval(CLEANUP_INTERVAL);

    // Consume the first immediate ticks
    flush_tick.tick().await;
    cleanup_tick.tick().await;

    loop {
        tokio::select! {
            entry = rx.recv() => {
                match entry {
                    Some(e) => {
                        buffer.push(e);
                        if buffer.len() >= BATCH_SIZE {
                            flush(&store, &mut buffer);
                        }
                    }
                    None => {
                        // Channel closed — flush remaining and exit
                        if !buffer.is_empty() {
                            flush(&store, &mut buffer);
                        }
                        break;
                    }
                }
            }
            _ = flush_tick.tick() => {
                if !buffer.is_empty() {
                    flush(&store, &mut buffer);
                }
            }
            _ = cleanup_tick.tick() => {
                match store.cleanup_old(DEFAULT_TTL) {
                    Ok(0) => {}
                    Ok(n) => log::info!("Audit cleanup: purged {} old entries", n),
                    Err(e) => log::error!("Audit cleanup failed: {}", e),
                }
            }
        }
    }
}

fn flush(store: &AuditStore, buffer: &mut Vec<AuditEntry>) {
    if let Err(e) = store.insert_batch(buffer) {
        log::error!("Audit batch insert failed: {}", e);
    }
    buffer.clear();
}
