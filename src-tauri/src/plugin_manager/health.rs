use super::storage::PluginStatus;
use crate::lifecycle_events::{self, LifecycleEvent};
use crate::runtime::ContainerState;
use crate::AppState;

/// Reconcile stored plugin states against actual Docker container states.
/// Returns true if any state was updated.
pub async fn sync_plugin_states(state: &AppState, app: Option<&tauri::AppHandle>) -> bool {
    let (container_ids, runtime) = {
        let mgr = state.read().await;
        let ids: Vec<(String, Option<String>, PluginStatus)> = mgr
            .storage
            .list()
            .iter()
            .map(|p| {
                (
                    p.manifest.id.clone(),
                    p.container_id.clone(),
                    p.status.clone(),
                )
            })
            .collect();
        (ids, mgr.runtime.clone())
    };

    if container_ids.is_empty() {
        return false;
    }

    let mut changed = false;

    for (plugin_id, container_id, stored_status) in container_ids {
        let actual_status = match &container_id {
            Some(cid) => match runtime.container_state(cid).await {
                Ok(state) => state,
                Err(_) => ContainerState::Gone,
            },
            None => ContainerState::Gone,
        };

        let new_status = match actual_status {
            ContainerState::Running => PluginStatus::Running,
            ContainerState::Stopped => PluginStatus::Stopped,
            ContainerState::Gone => {
                // Container was removed externally
                if stored_status == PluginStatus::Running {
                    PluginStatus::Error
                } else {
                    stored_status.clone()
                }
            }
        };

        if new_status != stored_status {
            let mut mgr = state.write().await;
            if let Some(plugin) = mgr.storage.get_mut(&plugin_id) {
                log::info!(
                    "Plugin {} state sync: {:?} → {:?}",
                    plugin_id,
                    stored_status,
                    new_status
                );
                plugin.status = new_status.clone();
                if actual_status == ContainerState::Gone {
                    plugin.container_id = None;
                }
                // Clone before save to satisfy borrow checker
                let plugin_snapshot = plugin.clone();
                let _ = mgr.storage.save();

                // Emit lifecycle event for the state transition
                match new_status {
                    PluginStatus::Error => {
                        lifecycle_events::emit(app, LifecycleEvent::PluginError {
                            plugin_id: plugin_id.clone(),
                            action: "sync".into(),
                            message: "Container stopped or disappeared externally".into(),
                        });
                    }
                    _ => {
                        lifecycle_events::emit(app, LifecycleEvent::PluginStopped {
                            plugin: plugin_snapshot,
                        });
                    }
                }
            }
            changed = true;
        }
    }

    changed
}
