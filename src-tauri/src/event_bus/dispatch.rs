use std::path::Path;
use std::sync::Arc;

use super::executor::RouteActionExecutor;
use super::store::EventStore;
use super::{create_event_bus, retry_worker, SharedEventBus, SharedEventStore};
use crate::AppState;

/// Single entry point for the entire event bus system.
/// Bundles the pub/sub bus, durable store, and route action executor.
#[derive(Clone)]
pub struct Dispatch {
    pub bus: SharedEventBus,
    pub store: SharedEventStore,
    pub executor: RouteActionExecutor,
}

impl Dispatch {
    pub fn new(
        data_dir: &Path,
        state: AppState,
        app_handle: tauri::AppHandle,
    ) -> Result<Self, String> {
        let bus = create_event_bus(data_dir);
        let store = Arc::new(EventStore::new(data_dir)?);
        let executor = RouteActionExecutor::new(state, app_handle);
        Ok(Self {
            bus,
            store,
            executor,
        })
    }

    /// Spawn the background retry worker.
    ///
    /// Uses `tauri::async_runtime::spawn` so this is safe to call from the
    /// Tauri `setup` closure (which runs on the main thread before a bare
    /// `tokio::spawn` would have runtime context).
    pub fn spawn_retry_worker(&self) {
        let store = self.store.clone();
        let executor = self.executor.clone();
        tauri::async_runtime::spawn(async move {
            retry_worker::run(store, executor).await;
        });
    }
}
