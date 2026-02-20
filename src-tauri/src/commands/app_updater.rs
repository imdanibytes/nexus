use crate::AppState;
use serde::Serialize;
use tauri::Emitter;
use tauri_plugin_updater::UpdaterExt;
use tokio::sync::Mutex;

/// Cached pending update so `download_app_update` can consume it after `check_app_update`.
pub struct PendingAppUpdate(pub Mutex<Option<tauri_plugin_updater::Update>>);

#[derive(Serialize, Clone)]
pub struct AppUpdateInfo {
    pub version: String,
    pub body: Option<String>,
    pub date: Option<String>,
}

#[tauri::command]
pub async fn check_app_update(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    pending: tauri::State<'_, PendingAppUpdate>,
) -> Result<Option<AppUpdateInfo>, String> {
    let channel = {
        let mgr = state.read().await;
        mgr.settings.update_channel.clone()
    };

    let endpoint: url::Url = match channel.as_str() {
        "nightly" => "https://github.com/imdanibytes/nexus/releases/download/nightly/latest.json",
        _ => "https://github.com/imdanibytes/nexus/releases/latest/download/latest.json",
    }
    .parse()
    .map_err(|e: url::ParseError| e.to_string())?;

    let updater = app
        .updater_builder()
        .endpoints(vec![endpoint])
        .map_err(|e: tauri_plugin_updater::Error| e.to_string())?
        .build()
        .map_err(|e: tauri_plugin_updater::Error| e.to_string())?;

    let update = updater.check().await.map_err(|e| e.to_string())?;

    match update {
        Some(u) => {
            let info = AppUpdateInfo {
                version: u.version.clone(),
                body: u.body.clone(),
                date: u.date.as_ref().map(|d| d.to_string()),
            };
            *pending.0.lock().await = Some(u);
            Ok(Some(info))
        }
        None => {
            *pending.0.lock().await = None;
            Ok(None)
        }
    }
}

#[tauri::command]
pub async fn download_app_update(
    app: tauri::AppHandle,
    pending: tauri::State<'_, PendingAppUpdate>,
) -> Result<(), String> {
    let update = pending
        .0
        .lock()
        .await
        .take()
        .ok_or("No pending update — call check_app_update first")?;

    let progress_handle = app.clone();
    let finish_handle = app.clone();

    update
        .download_and_install(
            move |chunk_length, content_length| {
                let _ = progress_handle.emit(
                    "nexus://app-update-progress",
                    serde_json::json!({
                        "event": "progress",
                        "chunkLength": chunk_length,
                        "contentLength": content_length,
                    }),
                );
            },
            move || {
                let _ = finish_handle.emit(
                    "nexus://app-update-progress",
                    serde_json::json!({ "event": "finished" }),
                );
            },
        )
        .await
        .map_err(|e: tauri_plugin_updater::Error| e.to_string())
}

#[tauri::command]
pub async fn get_update_channel(state: tauri::State<'_, AppState>) -> Result<String, String> {
    let mgr = state.read().await;
    Ok(mgr.settings.update_channel.clone())
}

#[tauri::command]
pub async fn set_update_channel(
    state: tauri::State<'_, AppState>,
    channel: String,
) -> Result<(), String> {
    let mut mgr = state.write().await;
    mgr.settings.update_channel = channel;
    mgr.settings.save().map_err(|e| e.to_string())
}
