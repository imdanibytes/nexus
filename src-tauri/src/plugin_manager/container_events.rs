//! Real-time container event watcher.
//!
//! Subscribes to the container runtime's event stream and emits lifecycle
//! events when external state changes are detected (e.g. `docker stop`,
//! OOM kill, crash). Reconnects with exponential backoff on stream errors.

use crate::lifecycle_events::{self, LifecycleEvent};
use crate::plugin_manager::storage::PluginStatus;
use crate::runtime::{ContainerEvent, ContainerEventAction, ContainerRuntime, ContainerState};
use crate::AppState;
use futures_util::StreamExt;
use std::sync::Arc;
use std::time::Duration;

/// Spawn a background task that subscribes to container events and emits
/// lifecycle events when external state changes are detected.
pub fn spawn(app: tauri::AppHandle, state: AppState, runtime: Arc<dyn ContainerRuntime>) {
    tauri::async_runtime::spawn(async move {
        let mut backoff = Duration::from_secs(1);

        loop {
            match run_event_loop(&app, &state, &runtime).await {
                Ok(()) => {
                    log::warn!("Container event stream ended, reconnecting...");
                    backoff = Duration::from_secs(1);
                }
                Err(e) => {
                    log::warn!(
                        "Container event stream error: {e}, retrying in {}s",
                        backoff.as_secs()
                    );
                }
            }
            tokio::time::sleep(backoff).await;
            backoff = (backoff * 2).min(Duration::from_secs(30));
        }
    });
}

async fn run_event_loop(
    app: &tauri::AppHandle,
    state: &AppState,
    runtime: &Arc<dyn ContainerRuntime>,
) -> Result<(), String> {
    let stream = runtime
        .subscribe_events("nexus.plugin.id")
        .ok_or("Container runtime does not support event streaming")?;

    futures_util::pin_mut!(stream);

    while let Some(result) = stream.next().await {
        let event = result.map_err(|e| e.to_string())?;
        handle_event(app, state, runtime, event).await;
    }

    Ok(())
}

async fn handle_event(
    app: &tauri::AppHandle,
    state: &AppState,
    runtime: &Arc<dyn ContainerRuntime>,
    event: ContainerEvent,
) {
    let plugin_id = match event.labels.get("nexus.plugin.id") {
        Some(id) => id.clone(),
        None => return,
    };

    // Read stored state for this plugin
    let (container_id, stored_status) = {
        let mgr = state.read().await;
        match mgr.storage.get(&plugin_id) {
            Some(p) => (p.container_id.clone(), p.status.clone()),
            None => return, // Unknown plugin — ignore
        }
    };

    // Check actual container state to confirm the change
    let actual_state = match &container_id {
        Some(cid) => match runtime.container_state(cid).await {
            Ok(s) => s,
            Err(_) => ContainerState::Gone,
        },
        None => ContainerState::Gone,
    };

    let new_status = match actual_state {
        ContainerState::Running => PluginStatus::Running,
        ContainerState::Stopped => PluginStatus::Stopped,
        ContainerState::Gone => {
            if stored_status == PluginStatus::Running {
                PluginStatus::Error
            } else {
                stored_status.clone()
            }
        }
    };

    if new_status == stored_status {
        return; // No divergence — likely an event from a Nexus-initiated action
    }

    log::info!(
        "Container event {:?} for plugin {}: {:?} → {:?}",
        event.action,
        plugin_id,
        stored_status,
        new_status
    );

    let mut mgr = state.write().await;
    if let Some(plugin) = mgr.storage.get_mut(&plugin_id) {
        plugin.status = new_status.clone();
        if actual_state == ContainerState::Gone {
            plugin.container_id = None;
        }

        let plugin_snapshot = plugin.clone();
        let _ = mgr.storage.save();

        match new_status {
            PluginStatus::Error => {
                let message = match event.action {
                    ContainerEventAction::Oom => "Container killed by OOM".into(),
                    ContainerEventAction::Kill => "Container killed externally".into(),
                    ContainerEventAction::Destroy => "Container destroyed externally".into(),
                    _ => "Container stopped or disappeared externally".into(),
                };
                lifecycle_events::emit(
                    Some(app),
                    LifecycleEvent::PluginError {
                        plugin_id,
                        action: "container_event".into(),
                        message,
                    },
                );
            }
            _ => {
                lifecycle_events::emit(
                    Some(app),
                    LifecycleEvent::PluginStopped {
                        plugin: plugin_snapshot,
                    },
                );
            }
        }
    }
}
