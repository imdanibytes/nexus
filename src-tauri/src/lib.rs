mod commands;
mod error;
pub mod host_api;
mod permissions;
mod plugin_manager;

use host_api::approval::ApprovalBridge;
use plugin_manager::PluginManager;
use std::sync::Arc;
use tauri::Manager;
use tokio::sync::RwLock;

pub type AppState = Arc<RwLock<PluginManager>>;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            let app_handle = app.handle().clone();
            let data_dir = app_handle
                .path()
                .app_data_dir()
                .expect("failed to get app data dir");
            std::fs::create_dir_all(&data_dir).ok();

            let state = Arc::new(RwLock::new(PluginManager::new(data_dir.clone())));
            app.manage(state.clone());

            let approval_bridge = Arc::new(ApprovalBridge::new(app_handle.clone()));
            app.manage(approval_bridge.clone());

            // Spawn Host API server and Docker network setup
            let state_clone = state.clone();
            tauri::async_runtime::spawn(async move {
                // Ensure nexus-bridge Docker network exists
                if let Err(e) = plugin_manager::docker::ensure_network().await {
                    log::error!("Failed to create Docker network: {}", e);
                }

                // Start the Host API server
                if let Err(e) = host_api::start_server(state_clone, approval_bridge).await {
                    log::error!("Host API server failed: {}", e);
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::plugins::plugin_list,
            commands::plugins::plugin_preview_remote,
            commands::plugins::plugin_preview_local,
            commands::plugins::plugin_install,
            commands::plugins::plugin_install_local,
            commands::plugins::plugin_start,
            commands::plugins::plugin_stop,
            commands::plugins::plugin_remove,
            commands::plugins::plugin_sync_status,
            commands::plugins::plugin_logs,
            commands::plugins::plugin_get_settings,
            commands::plugins::plugin_save_settings,
            commands::marketplace::marketplace_search,
            commands::marketplace::marketplace_refresh,
            commands::permissions::permission_grant,
            commands::permissions::permission_revoke,
            commands::permissions::permission_list,
            commands::permissions::permission_remove_path,
            commands::system::app_version,
            commands::system::check_docker,
            commands::system::open_docker_desktop,
            commands::system::container_resource_usage,
            commands::system::get_resource_quotas,
            commands::system::save_resource_quotas,
            commands::permissions::runtime_approval_respond,
            commands::registries::registry_list,
            commands::registries::registry_add,
            commands::registries::registry_remove,
            commands::registries::registry_toggle,
            commands::mcp::mcp_get_settings,
            commands::mcp::mcp_set_enabled,
            commands::mcp::mcp_list_tools,
            commands::mcp::mcp_config_snippet,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
