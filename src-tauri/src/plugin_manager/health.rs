use super::docker;
use super::storage::PluginStatus;
use crate::AppState;

/// Reconcile stored plugin states against actual Docker container states.
/// Returns true if any state was updated.
pub async fn sync_plugin_states(state: &AppState) -> bool {
    let container_ids: Vec<(String, Option<String>, PluginStatus)> = {
        let mgr = state.read().await;
        mgr.storage
            .list()
            .iter()
            .map(|p| {
                (
                    p.manifest.id.clone(),
                    p.container_id.clone(),
                    p.status.clone(),
                )
            })
            .collect()
    };

    if container_ids.is_empty() {
        return false;
    }

    let mut changed = false;

    for (plugin_id, container_id, stored_status) in container_ids {
        let actual_status = match &container_id {
            Some(cid) => match docker::container_state(cid).await {
                Ok("running") => ContainerState::Running,
                Ok(_) => ContainerState::Stopped,
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
                plugin.status = new_status;
                if actual_status == ContainerState::Gone {
                    plugin.container_id = None;
                }
                let _ = mgr.storage.save();
            }
            changed = true;
        }
    }

    changed
}

#[derive(Debug, PartialEq)]
enum ContainerState {
    Running,
    Stopped,
    Gone,
}
