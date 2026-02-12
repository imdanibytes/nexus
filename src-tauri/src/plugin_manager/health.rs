use super::storage::PluginStatus;
use crate::AppState;
use std::time::Duration;

pub fn spawn_health_checker(state: AppState) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(15));

        loop {
            interval.tick().await;

            let plugins: Vec<(String, Option<String>, u16, Option<String>, u64)> = {
                let mgr = state.read().await;
                mgr.storage
                    .list()
                    .iter()
                    .filter(|p| p.status == PluginStatus::Running)
                    .map(|p| {
                        (
                            p.manifest.id.clone(),
                            p.container_id.clone(),
                            p.assigned_port,
                            p.manifest.health.as_ref().map(|h| h.endpoint.clone()),
                            p.manifest
                                .health
                                .as_ref()
                                .map(|h| h.interval_secs)
                                .unwrap_or(30),
                        )
                    })
                    .collect()
            };

            for (plugin_id, container_id, port, health_endpoint, _interval_secs) in &plugins {
                let healthy = if let Some(endpoint) = health_endpoint {
                    check_health(*port, endpoint).await
                } else if let Some(cid) = container_id {
                    super::docker::container_running(cid).await.unwrap_or(false)
                } else {
                    false
                };

                if !healthy {
                    let mut mgr = state.write().await;
                    if let Some(plugin) = mgr.storage.get_mut(plugin_id) {
                        if plugin.status == PluginStatus::Running {
                            log::warn!("Plugin {} failed health check", plugin_id);
                            plugin.status = PluginStatus::Error;
                            let _ = mgr.storage.save();
                        }
                    }
                }
            }
        }
    });
}

async fn check_health(port: u16, endpoint: &str) -> bool {
    let url = format!("http://127.0.0.1:{}{}", port, endpoint);
    match reqwest::get(&url).await {
        Ok(resp) => resp.status().is_success(),
        Err(_) => false,
    }
}
