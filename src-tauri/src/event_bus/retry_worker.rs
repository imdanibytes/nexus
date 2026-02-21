use std::sync::Arc;
use std::time::Duration;

use tokio::time::interval;

use super::executor::RouteActionExecutor;
use super::store::EventStore;

/// Run the background retry worker that processes pending deliveries
/// and periodically cleans up expired events.
///
/// This is an async function — the caller is responsible for spawning it
/// (e.g. via `tauri::async_runtime::spawn`).
pub async fn run(store: Arc<EventStore>, executor: RouteActionExecutor) {
    let mut retry_tick = interval(Duration::from_secs(5));
    let mut cleanup_tick = interval(Duration::from_secs(3600));

    // Consume the first immediate tick
    retry_tick.tick().await;
    cleanup_tick.tick().await;

    loop {
        tokio::select! {
            _ = retry_tick.tick() => {
                process_pending(&store, &executor).await;
            }
            _ = cleanup_tick.tick() => {
                cleanup_expired(&store);
            }
        }
    }
}

/// Claim pending deliveries and execute them.
async fn process_pending(store: &Arc<EventStore>, executor: &RouteActionExecutor) {
    let batch = match store.claim_ready(50) {
        Ok(rows) => rows,
        Err(e) => {
            log::error!("Retry worker: failed to claim deliveries: {}", e);
            return;
        }
    };

    if batch.is_empty() {
        return;
    }

    log::debug!("Retry worker: processing {} deliveries", batch.len());

    for (delivery, event_json) in batch {
        let store = store.clone();
        let executor = executor.clone();

        tokio::spawn(async move {
            let event = match EventStore::parse_event_json(&event_json) {
                Ok(e) => e,
                Err(e) => {
                    log::error!(
                        "Retry worker: failed to parse event for delivery {}: {}",
                        delivery.id,
                        e
                    );
                    let _ = store.mark_failed(delivery.id, &e);
                    return;
                }
            };

            match executor.execute_single(delivery.action.clone(), event).await {
                Ok(()) => {
                    log::info!(
                        "Retry worker: delivery {} completed (attempt {})",
                        delivery.id,
                        delivery.attempts + 1
                    );
                    if let Err(e) = store.mark_completed(delivery.id) {
                        log::error!("Retry worker: failed to mark completed: {}", e);
                    }
                }
                Err(e) => {
                    log::warn!(
                        "Retry worker: delivery {} failed (attempt {}): {}",
                        delivery.id,
                        delivery.attempts + 1,
                        e
                    );
                    if let Err(me) = store.mark_failed(delivery.id, &e) {
                        log::error!("Retry worker: failed to mark failed: {}", me);
                    }
                }
            }
        });
    }
}

/// Clean up events and deliveries older than 7 days.
fn cleanup_expired(store: &EventStore) {
    let ttl = Duration::from_secs(7 * 86400);
    match store.cleanup_old(ttl) {
        Ok(0) => {}
        Ok(n) => log::info!("Retry worker: purged {} expired events", n),
        Err(e) => log::error!("Retry worker: TTL cleanup failed: {}", e),
    }

    // Log dead letter count for monitoring
    match store.dead_letter_count() {
        Ok(0) => {}
        Ok(n) => log::info!("Retry worker: {} deliveries in dead letter queue", n),
        Err(e) => log::error!("Retry worker: failed to count dead letters: {}", e),
    }
}
