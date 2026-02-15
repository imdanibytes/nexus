use crate::plugin_manager::docker;
use crate::plugin_manager::manifest::PluginManifest;
use crate::plugin_manager::storage::PluginStatus;
use crate::AppState;

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tauri::Emitter;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

/// Emitted on the `nexus://dev-rebuild` Tauri event channel.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DevRebuildEvent {
    pub plugin_id: String,
    pub status: &'static str, // started | building | restarting | complete | error
    pub message: String,
}

struct WatcherHandle {
    _watcher: RecommendedWatcher,
    cancel: tokio::sync::watch::Sender<bool>,
}

pub struct DevWatcher {
    watchers: Mutex<HashMap<String, WatcherHandle>>,
}

impl DevWatcher {
    pub fn new() -> Self {
        DevWatcher {
            watchers: Mutex::new(HashMap::new()),
        }
    }

    /// Start watching a directory for changes. On change, triggers a debounced rebuild.
    pub async fn start_watching(
        self: &Arc<Self>,
        plugin_id: String,
        watch_dir: PathBuf,
        state: AppState,
        app_handle: tauri::AppHandle,
    ) -> Result<(), String> {
        let mut watchers = self.watchers.lock().await;

        // Already watching? Stop the old one first.
        if let Some(old) = watchers.remove(&plugin_id) {
            let _ = old.cancel.send(true);
        }

        let (cancel_tx, cancel_rx) = tokio::sync::watch::channel(false);
        let (fs_tx, mut fs_rx) = mpsc::channel::<()>(16);

        // Create the FS watcher
        let tx = fs_tx.clone();
        let watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            if let Ok(event) = res {
                // Only trigger on content changes, not metadata
                match event.kind {
                    EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {
                        let _ = tx.try_send(());
                    }
                    _ => {}
                }
            }
        })
        .map_err(|e| format!("Failed to create file watcher: {}", e))?;

        let mut watcher = watcher;
        watcher
            .watch(watch_dir.as_ref(), RecursiveMode::Recursive)
            .map_err(|e| format!("Failed to watch directory: {}", e))?;

        log::info!(
            "Dev watcher started for plugin '{}' on {}",
            plugin_id,
            watch_dir.display()
        );

        // Spawn the debounce + rebuild loop
        let pid = plugin_id.clone();
        let dir = watch_dir.clone();
        tokio::spawn(async move {
            let mut cancel_rx = cancel_rx;
            loop {
                tokio::select! {
                    _ = cancel_rx.changed() => {
                        log::info!("Dev watcher cancelled for '{}'", pid);
                        break;
                    }
                    recv = fs_rx.recv() => {
                        if recv.is_none() {
                            break;
                        }
                        // Debounce: drain any additional events within 2 seconds
                        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                        while fs_rx.try_recv().is_ok() {}

                        // Trigger rebuild
                        rebuild_plugin(&state, &app_handle, &pid, &dir).await;
                    }
                }
            }
        });

        watchers.insert(
            plugin_id,
            WatcherHandle {
                _watcher: watcher,
                cancel: cancel_tx,
            },
        );
        Ok(())
    }

    /// Stop watching a plugin's source directory.
    pub async fn stop_watching(&self, plugin_id: &str) {
        let mut watchers = self.watchers.lock().await;
        if let Some(handle) = watchers.remove(plugin_id) {
            let _ = handle.cancel.send(true);
            log::info!("Dev watcher stopped for '{}'", plugin_id);
        }
    }

    /// Check if a plugin is currently being watched.
    #[allow(dead_code)]
    pub async fn is_watching(&self, plugin_id: &str) -> bool {
        self.watchers.lock().await.contains_key(plugin_id)
    }
}

fn emit_rebuild(app_handle: &tauri::AppHandle, plugin_id: &str, status: &'static str, message: String) {
    let event = DevRebuildEvent {
        plugin_id: plugin_id.to_string(),
        status,
        message,
    };
    let _ = app_handle.emit("nexus://dev-rebuild", &event);
}

/// Read manifest, build image, reinstall plugin, restart if it was running.
pub async fn rebuild_plugin(
    state: &AppState,
    app_handle: &tauri::AppHandle,
    plugin_id: &str,
    source_dir: &Path,
) {
    emit_rebuild(app_handle, plugin_id, "started", "Rebuild triggered by file change".into());

    // Read manifest
    let manifest_path = source_dir.join("plugin.json");
    let manifest_data = match std::fs::read_to_string(&manifest_path) {
        Ok(d) => d,
        Err(e) => {
            emit_rebuild(app_handle, plugin_id, "error", format!("Failed to read manifest: {}", e));
            return;
        }
    };
    let manifest: PluginManifest = match serde_json::from_str(&manifest_data) {
        Ok(m) => m,
        Err(e) => {
            emit_rebuild(app_handle, plugin_id, "error", format!("Invalid manifest: {}", e));
            return;
        }
    };

    // Build Docker image
    emit_rebuild(app_handle, plugin_id, "building", format!("Building image {}", manifest.image));
    if let Err(e) = docker::build_image(source_dir, &manifest.image).await {
        emit_rebuild(app_handle, plugin_id, "error", format!("Docker build failed: {}", e));
        return;
    }

    // Check current state
    let was_running = {
        let mgr = state.read().await;
        mgr.storage
            .get(plugin_id)
            .map(|p| p.status == PluginStatus::Running)
            .unwrap_or(false)
    };

    // Reinstall (preserves permissions, dev_mode, volume)
    emit_rebuild(app_handle, plugin_id, "restarting", "Reinstalling plugin...".into());
    {
        let mut mgr = state.write().await;

        // Collect existing permissions to re-grant
        let existing_perms: Vec<crate::permissions::Permission> = mgr
            .permissions
            .get_grants(plugin_id)
            .into_iter()
            .map(|g| g.permission)
            .collect();

        let local_path = Some(manifest_path.display().to_string());

        if let Err(e) = mgr
            .install(manifest, existing_perms, vec![], None, local_path)
            .await
        {
            emit_rebuild(app_handle, plugin_id, "error", format!("Reinstall failed: {}", e));
            return;
        }

        // Restart if it was running
        if was_running {
            if let Err(e) = mgr.start(plugin_id).await {
                emit_rebuild(app_handle, plugin_id, "error", format!("Restart failed: {}", e));
                return;
            }
        }

        mgr.notify_tools_changed();
    }

    emit_rebuild(app_handle, plugin_id, "complete", "Rebuild complete".into());
    log::info!("Dev rebuild complete for '{}'", plugin_id);
}
