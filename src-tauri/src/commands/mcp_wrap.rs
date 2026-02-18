use crate::lifecycle_events::{self, LifecycleEvent};
use crate::mcp_wrap::{classify, discovery, generate, PluginMetadata};
use crate::mcp_wrap::classify::ClassifiedTool;
use crate::plugin_manager::{manifest::PluginManifest, storage::InstalledPlugin};
use crate::permissions::Permission;
use crate::AppState;
use tauri::Manager;

/// Discover and classify tools from an MCP server command.
#[tauri::command]
pub async fn mcp_discover_tools(command: String) -> Result<Vec<ClassifiedTool>, String> {
    let raw_tools = discovery::discover_tools(&command)
        .await
        .map_err(|e| e.to_string())?;

    Ok(classify::classify_tools(&raw_tools))
}

/// Suggest plugin metadata from an MCP server command.
#[tauri::command]
pub async fn mcp_suggest_metadata(command: String) -> Result<PluginMetadata, String> {
    Ok(crate::mcp_wrap::suggest_metadata(&command))
}

/// Generate plugin artifacts, build Docker image, and install the plugin.
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn mcp_generate_and_install(
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
    command: String,
    tools: Vec<ClassifiedTool>,
    metadata: PluginMetadata,
    approved_permissions: Vec<Permission>,
    deferred_permissions: Vec<Permission>,
) -> Result<InstalledPlugin, String> {
    let data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    let mcp_plugins_dir = data_dir.join("mcp-plugins");
    std::fs::create_dir_all(&mcp_plugins_dir)
        .map_err(|e| format!("Failed to create mcp-plugins dir: {}", e))?;

    // 1. Generate plugin artifacts
    let plugin_dir = generate::generate_plugin(&tools, &metadata, &command, &mcp_plugins_dir)
        .map_err(|e| format!("Failed to generate plugin: {}", e))?;

    // 2. Build Docker image from generated Dockerfile
    let manifest_path = plugin_dir.join("plugin.json");
    let manifest_data = std::fs::read_to_string(&manifest_path)
        .map_err(|e| format!("Failed to read generated manifest: {}", e))?;
    let manifest: PluginManifest = serde_json::from_str(&manifest_data)
        .map_err(|e| format!("Invalid generated manifest: {}", e))?;

    let runtime = { state.read().await.runtime.clone() };
    runtime
        .build_image(&plugin_dir, &manifest.image)
        .await
        .map_err(|e| format!("Docker build failed: {}", e))?;

    // 3. Install via PluginManager
    let plugin_id = manifest.id.clone();

    lifecycle_events::emit(Some(&app_handle), LifecycleEvent::PluginInstalling {
        message: "Installing MCP plugin...".into(),
    });

    let mut mgr = state.write().await;
    match mgr.install(manifest, approved_permissions, deferred_permissions, None, None).await {
        Ok(plugin) => {
            lifecycle_events::emit(Some(&app_handle), LifecycleEvent::PluginInstalled {
                plugin: plugin.clone(),
            });
            Ok(plugin)
        }
        Err(e) => {
            lifecycle_events::emit(Some(&app_handle), LifecycleEvent::PluginError {
                plugin_id,
                action: "installing".into(),
                message: e.to_string(),
            });
            Err(e.to_string())
        }
    }
}
