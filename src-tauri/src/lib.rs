mod commands;
mod error;
pub mod extensions;
pub mod host_api;
pub mod lifecycle_events;
pub mod mcp_wrap;
mod notification;
pub mod oauth;
mod permissions;
mod plugin_manager;
pub mod runtime;
mod update_checker;
pub(crate) mod util;
mod version;

use host_api::approval::ApprovalBridge;
use plugin_manager::dev_watcher::DevWatcher;
use plugin_manager::PluginManager;
use runtime::docker::DockerRuntime;
use std::sync::Arc;
use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::Manager;
use tokio::sync::RwLock;

pub type AppState = Arc<RwLock<PluginManager>>;

/// Shared active theme identifier — readable by the Axum server (e.g. OAuth consent page)
/// and writable by the Tauri `set_theme` command.
#[derive(Clone)]
pub struct ActiveTheme(Arc<std::sync::RwLock<String>>);

impl ActiveTheme {
    pub fn new(theme: String) -> Self {
        Self(Arc::new(std::sync::RwLock::new(theme)))
    }

    pub fn get(&self) -> String {
        self.0.read().unwrap_or_else(|e| e.into_inner()).clone()
    }

    pub fn set(&self, theme: String) {
        let mut t = self.0.write().unwrap_or_else(|e| e.into_inner());
        *t = theme;
    }
}

/// Show the main window and switch to Regular activation policy (dock icon visible).
#[cfg(target_os = "macos")]
fn show_window(app: &tauri::AppHandle) {
    let _ = app.set_activation_policy(tauri::ActivationPolicy::Regular);
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

/// Hide the main window and switch to Accessory activation policy (no dock icon).
#[cfg(target_os = "macos")]
fn hide_window(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.hide();
    }
    let _ = app.set_activation_policy(tauri::ActivationPolicy::Accessory);
}

#[cfg(not(target_os = "macos"))]
fn show_window(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

#[cfg(not(target_os = "macos"))]
fn hide_window(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.hide();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            // Another instance tried to launch — bring the existing window to front
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
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

                // Tag window title so dev builds are visually distinct
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.set_title("Nexus (dev)");
                }
            }

            // Request OS notification permission (macOS shows a system dialog on first launch)
            notification::init();

            let app_handle = app.handle().clone();
            let data_dir = app_handle
                .path()
                .app_data_dir()
                .expect("failed to get app data dir");
            std::fs::create_dir_all(&data_dir).ok();

            let docker_runtime = DockerRuntime::new()
                .expect("failed to connect to Docker daemon");
            let runtime: Arc<dyn runtime::ContainerRuntime> = Arc::new(docker_runtime);

            let perm_store = permissions::PermissionStore::load(&data_dir).unwrap_or_default();
            let perm_service: Arc<dyn permissions::PermissionService> =
                Arc::new(permissions::DefaultPermissionService::new(perm_store));

            // OAuth 2.1 store — shared between Host API, Tauri commands, and PluginManager
            let oauth_store = Arc::new(oauth::OAuthStore::load(&data_dir));
            app.manage(oauth_store.clone());

            let mgr = PluginManager::new(data_dir.clone(), runtime.clone(), perm_service, oauth_store.clone());

            let state = Arc::new(RwLock::new(mgr));
            PluginManager::wire_extension_ipc(&state);
            app.manage(state.clone());

            // Active theme — shared between Tauri UI and Axum (OAuth consent page)
            let theme = {
                let mgr = state.blocking_read();
                ActiveTheme::new(mgr.settings.theme.clone())
            };
            app.manage(theme.clone());

            let approval_bridge = Arc::new(ApprovalBridge::new(app_handle.clone()));
            app.manage(approval_bridge.clone());

            let dev_watcher = Arc::new(DevWatcher::new());
            app.manage(dev_watcher.clone());

            // Restore dev watchers for plugins with dev_mode enabled
            {
                let mgr = state.blocking_read();
                let dev_plugins: Vec<(String, std::path::PathBuf)> = mgr
                    .storage
                    .list()
                    .iter()
                    .filter(|p| p.dev_mode)
                    .filter_map(|p| {
                        p.local_manifest_path.as_ref().and_then(|mp| {
                            std::path::Path::new(mp)
                                .parent()
                                .map(|dir| (p.manifest.id.clone(), dir.to_path_buf()))
                        })
                    })
                    .collect();
                drop(mgr);

                if !dev_plugins.is_empty() {
                    let dw = dev_watcher.clone();
                    let s = state.clone();
                    let ah = app_handle.clone();
                    tauri::async_runtime::spawn(async move {
                        for (id, dir) in dev_plugins {
                            if let Err(e) = dw.start_watching(id.clone(), dir, s.clone(), ah.clone()).await {
                                log::warn!("Failed to restore dev watcher for '{}': {}", id, e);
                            }
                        }
                    });
                }
            }

            // Spawn Host API server and Docker network setup
            let state_clone = state.clone();
            let runtime_clone = runtime.clone();
            let oauth_clone = oauth_store.clone();
            let theme_clone = theme.clone();
            tauri::async_runtime::spawn(async move {
                // Ensure nexus-bridge Docker network exists
                if let Err(e) = runtime_clone.ensure_network("nexus-bridge").await {
                    log::error!("Failed to create Docker network: {}", e);
                }

                // Start the Host API server
                if let Err(e) = host_api::start_server(state_clone, approval_bridge, oauth_clone, theme_clone).await {
                    log::error!("Host API server failed: {}", e);
                }
            });

            // Build system tray with menu (keeps app running when window is closed)
            let show = MenuItemBuilder::with_id("show", "Show Nexus").build(app)?;
            let quit = MenuItemBuilder::with_id("quit", "Quit Nexus").build(app)?;
            let tray_menu = MenuBuilder::new(app).items(&[&show, &quit]).build()?;

            TrayIconBuilder::new()
                .icon(app.default_window_icon().cloned().unwrap())
                .icon_as_template(true)
                .tooltip("Nexus")
                .menu(&tray_menu)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "show" => show_window(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        show_window(tray.app_handle());
                    }
                })
                .build(app)?;

            Ok(())
        })
        .on_window_event(|window, event| {
            // Hide window instead of quitting — MCP gateway stays alive
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                hide_window(window.app_handle());
            }
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
            commands::plugins::check_image_available,
            commands::plugins::plugin_logs,
            commands::plugins::plugin_get_settings,
            commands::plugins::plugin_save_settings,
            commands::plugins::plugin_storage_info,
            commands::plugins::plugin_clear_storage,
            commands::plugins::plugin_dev_mode_toggle,
            commands::plugins::plugin_rebuild,
            commands::marketplace::marketplace_search,
            commands::marketplace::marketplace_refresh,
            commands::permissions::permission_grant,
            commands::permissions::permission_revoke,
            commands::permissions::permission_unrevoke,
            commands::permissions::permission_list,
            commands::permissions::permission_remove_path,
            commands::system::app_version,
            commands::system::check_engine,
            commands::system::container_resource_usage,
            commands::system::get_resource_quotas,
            commands::system::save_resource_quotas,
            commands::system::get_update_check_interval,
            commands::system::set_update_check_interval,
            commands::system::check_url_reachable,
            commands::system::set_language,
            commands::system::set_theme,
            commands::permissions::runtime_approval_respond,
            commands::registries::registry_list,
            commands::registries::registry_add,
            commands::registries::registry_remove,
            commands::registries::registry_toggle,
            commands::mcp::mcp_get_settings,
            commands::mcp::mcp_set_enabled,
            commands::mcp::mcp_list_tools,
            commands::mcp::mcp_config_snippet,
            commands::extensions::extension_list,
            commands::extensions::extension_install,
            commands::extensions::extension_install_local,
            commands::extensions::extension_enable,
            commands::extensions::extension_disable,
            commands::extensions::extension_remove,
            commands::extensions::extension_preview,
            commands::extensions::extension_marketplace_search,
            commands::permissions::permission_remove_scope,
            commands::updates::check_updates,
            commands::updates::get_cached_updates,
            commands::updates::dismiss_update,
            commands::updates::update_plugin,
            commands::updates::update_extension,
            commands::updates::update_extension_force_key,
            commands::updates::last_update_check,
            commands::mcp_wrap::mcp_discover_tools,
            commands::mcp_wrap::mcp_suggest_metadata,
            commands::mcp_wrap::mcp_generate_and_install,
            commands::oauth::oauth_list_clients,
            commands::oauth::oauth_revoke_client,
            notification::send_notification,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|_app_handle: &tauri::AppHandle, _event: tauri::RunEvent| {
        // macOS dock icon click — re-show the hidden window
        #[cfg(target_os = "macos")]
        if let tauri::RunEvent::Reopen {
            has_visible_windows, ..
        } = _event
        {
            if !has_visible_windows {
                show_window(_app_handle);
            }
        }
    });
}
