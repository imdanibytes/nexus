use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::extensions::capability::Capability;
use crate::extensions::manifest::ResourceTypeDef;
use crate::extensions::registry::ExtensionRegistry;
use crate::extensions::storage::InstalledExtension;
use crate::extensions::RiskLevel;
use crate::lifecycle_events::{self, LifecycleEvent};
use crate::permissions::Permission;
use crate::plugin_manager::registry::ExtensionRegistryEntry;
use crate::plugin_manager::PluginManager;
use crate::AppState;

#[derive(Debug, Clone, Serialize)]
pub struct ExtensionOperationStatus {
    pub name: String,
    pub description: String,
    pub risk_level: RiskLevel,
    pub scope_key: Option<String>,
    pub scope_description: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExtensionConsumer {
    pub plugin_id: String,
    pub plugin_name: String,
    pub granted: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExtensionStatus {
    pub id: String,
    pub display_name: String,
    pub description: String,
    pub operations: Vec<ExtensionOperationStatus>,
    pub capabilities: Vec<Capability>,
    pub consumers: Vec<ExtensionConsumer>,
    pub installed: bool,
    pub enabled: bool,
    #[serde(default)]
    pub resources: HashMap<String, ResourceTypeDef>,
}

/// Build an `ExtensionStatus` for a single extension by ID.
/// Checks the running registry first, then falls back to installed-but-disabled storage.
pub(crate) fn build_extension_status(mgr: &PluginManager, ext_id: &str) -> Option<ExtensionStatus> {
    // Check running registry first
    if let Some(ext_info) = mgr.extensions.list().into_iter().find(|e| e.id == ext_id) {
        let operations: Vec<ExtensionOperationStatus> = ext_info
            .operations
            .iter()
            .map(|op| ExtensionOperationStatus {
                name: op.name.clone(),
                description: op.description.clone(),
                risk_level: op.risk_level,
                scope_key: op.scope_key.clone(),
                scope_description: op.scope_description.clone(),
            })
            .collect();

        let mut consumers = Vec::new();
        for plugin in mgr.storage.list() {
            if let Some(deps) = plugin.manifest.extensions.get(&ext_info.id) {
                let all_granted = deps.operation_names().iter().all(|op_name| {
                    let perm_str = ExtensionRegistry::permission_string(&ext_info.id, op_name);
                    let perm = Permission::Extension(perm_str);
                    mgr.permissions.has_permission(&plugin.manifest.id, &perm)
                });

                consumers.push(ExtensionConsumer {
                    plugin_id: plugin.manifest.id.clone(),
                    plugin_name: plugin.manifest.name.clone(),
                    granted: all_granted,
                });
            }
        }

        let installed_ext = mgr.extension_loader.storage.get(&ext_info.id);
        let resources = installed_ext
            .map(|ie| ie.manifest.resources.clone())
            .unwrap_or_default();
        return Some(ExtensionStatus {
            id: ext_info.id,
            display_name: ext_info.display_name,
            description: ext_info.description,
            operations,
            capabilities: ext_info.capabilities,
            consumers,
            installed: installed_ext.is_some(),
            enabled: installed_ext.is_some_and(|e| e.enabled),
            resources,
        });
    }

    // Fall back to installed-but-disabled storage
    if let Some(installed) = mgr.extension_loader.storage.get(ext_id) {
        if !installed.enabled {
            let operations: Vec<ExtensionOperationStatus> = installed
                .manifest
                .operations
                .iter()
                .map(|op| ExtensionOperationStatus {
                    name: op.name.clone(),
                    description: op.description.clone(),
                    risk_level: op.risk_level,
                    scope_key: op.scope_key.clone(),
                    scope_description: op.scope_description.clone(),
                })
                .collect();

            return Some(ExtensionStatus {
                id: installed.manifest.id.clone(),
                display_name: installed.manifest.display_name.clone(),
                description: installed.manifest.description.clone(),
                operations,
                capabilities: installed.manifest.capabilities.clone(),
                consumers: Vec::new(),
                installed: true,
                enabled: false,
                resources: installed.manifest.resources.clone(),
            });
        }
    }

    None
}

/// List all registered (running) extensions with their status.
#[tauri::command]
pub async fn extension_list(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<ExtensionStatus>, String> {
    let mgr = state.read().await;

    let mut result = Vec::new();

    for ext_info in mgr.extensions.list() {
        let operations: Vec<ExtensionOperationStatus> = ext_info
            .operations
            .iter()
            .map(|op| ExtensionOperationStatus {
                name: op.name.clone(),
                description: op.description.clone(),
                risk_level: op.risk_level,
                scope_key: op.scope_key.clone(),
                scope_description: op.scope_description.clone(),
            })
            .collect();

        // Find which installed plugins consume this extension
        let mut consumers = Vec::new();
        for plugin in mgr.storage.list() {
            if let Some(deps) = plugin.manifest.extensions.get(&ext_info.id) {
                let all_granted = deps.operation_names().iter().all(|op_name| {
                    let perm_str =
                        ExtensionRegistry::permission_string(&ext_info.id, op_name);
                    let perm = Permission::Extension(perm_str);
                    mgr.permissions.has_permission(&plugin.manifest.id, &perm)
                });

                consumers.push(ExtensionConsumer {
                    plugin_id: plugin.manifest.id.clone(),
                    plugin_name: plugin.manifest.name.clone(),
                    granted: all_granted,
                });
            }
        }

        // Check if this extension is in the installed storage
        let installed_ext = mgr.extension_loader.storage.get(&ext_info.id);
        let resources = installed_ext
            .map(|ie| ie.manifest.resources.clone())
            .unwrap_or_default();

        result.push(ExtensionStatus {
            id: ext_info.id,
            display_name: ext_info.display_name,
            description: ext_info.description,
            operations,
            capabilities: ext_info.capabilities,
            consumers,
            installed: installed_ext.is_some(),
            enabled: installed_ext.is_some_and(|e| e.enabled),
            resources,
        });
    }

    // Also include installed-but-disabled extensions (not in the registry)
    for installed in mgr.extension_loader.storage.list() {
        if !installed.enabled {
            // Not in the running registry, show it as disabled
            if !result.iter().any(|r| r.id == installed.manifest.id) {
                let operations: Vec<ExtensionOperationStatus> = installed
                    .manifest
                    .operations
                    .iter()
                    .map(|op| ExtensionOperationStatus {
                        name: op.name.clone(),
                        description: op.description.clone(),
                        risk_level: op.risk_level,
                        scope_key: op.scope_key.clone(),
                        scope_description: op.scope_description.clone(),
                    })
                    .collect();

                result.push(ExtensionStatus {
                    id: installed.manifest.id.clone(),
                    display_name: installed.manifest.display_name.clone(),
                    description: installed.manifest.description.clone(),
                    operations,
                    capabilities: installed.manifest.capabilities.clone(),
                    consumers: Vec::new(),
                    installed: true,
                    enabled: false,
                    resources: installed.manifest.resources.clone(),
                });
            }
        }
    }

    Ok(result)
}

/// Install an extension from a manifest URL.
#[tauri::command]
pub async fn extension_install(
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
    manifest_url: String,
) -> Result<InstalledExtension, String> {
    // Fetch manifest
    let manifest = crate::plugin_manager::registry::fetch_extension_manifest(&manifest_url)
        .await
        .map_err(|e| e.to_string())?;

    let ext_id = manifest.id.clone();
    lifecycle_events::emit(Some(&app), LifecycleEvent::ExtensionInstalling {
        ext_id: ext_id.clone(),
    });

    let mut mgr = state.write().await;
    match mgr.extension_loader.install(manifest, Some(&manifest_url)).await {
        Ok(installed) => {
            if let Some(status) = build_extension_status(&mgr, &ext_id) {
                lifecycle_events::emit(Some(&app), LifecycleEvent::ExtensionInstalled {
                    extension: status,
                });
            }
            Ok(installed)
        }
        Err(e) => {
            lifecycle_events::emit(Some(&app), LifecycleEvent::ExtensionError {
                ext_id,
                action: "installing".into(),
                message: e.to_string(),
            });
            Err(e.to_string())
        }
    }
}

/// Enable an installed extension (spawns process, registers in runtime).
#[tauri::command]
pub async fn extension_enable(
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
    ext_id: String,
) -> Result<(), String> {
    lifecycle_events::emit(Some(&app), LifecycleEvent::ExtensionEnabling {
        ext_id: ext_id.clone(),
    });

    let mut mgr = state.write().await;
    match mgr.enable_extension(&ext_id) {
        Ok(()) => {
            if let Some(status) = build_extension_status(&mgr, &ext_id) {
                lifecycle_events::emit(Some(&app), LifecycleEvent::ExtensionEnabled {
                    extension: status,
                });
            }
            Ok(())
        }
        Err(e) => {
            lifecycle_events::emit(Some(&app), LifecycleEvent::ExtensionError {
                ext_id,
                action: "enabling".into(),
                message: e.to_string(),
            });
            Err(e.to_string())
        }
    }
}

/// Disable an extension (stops process, unregisters from runtime).
#[tauri::command]
pub async fn extension_disable(
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
    ext_id: String,
) -> Result<(), String> {
    lifecycle_events::emit(Some(&app), LifecycleEvent::ExtensionDisabling {
        ext_id: ext_id.clone(),
    });

    let mut mgr = state.write().await;
    match mgr.disable_extension(&ext_id) {
        Ok(()) => {
            if let Some(status) = build_extension_status(&mgr, &ext_id) {
                lifecycle_events::emit(Some(&app), LifecycleEvent::ExtensionDisabled {
                    extension: status,
                });
            }
            Ok(())
        }
        Err(e) => {
            lifecycle_events::emit(Some(&app), LifecycleEvent::ExtensionError {
                ext_id,
                action: "disabling".into(),
                message: e.to_string(),
            });
            Err(e.to_string())
        }
    }
}

/// Remove an extension entirely (stop, delete files, unregister).
#[tauri::command]
pub async fn extension_remove(
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
    ext_id: String,
) -> Result<(), String> {
    lifecycle_events::emit(Some(&app), LifecycleEvent::ExtensionRemoving {
        ext_id: ext_id.clone(),
    });

    let mut mgr = state.write().await;

    // Collect plugin IDs and their extension permissions to revoke
    let perm_prefix = format!("ext:{}:", ext_id);
    let revocations: Vec<(String, Vec<Permission>)> = mgr
        .storage
        .list()
        .iter()
        .map(|plugin| {
            let plugin_id = plugin.manifest.id.clone();
            let perms: Vec<Permission> = mgr
                .permissions
                .get_grants(&plugin_id)
                .into_iter()
                .filter(|g| g.permission.as_str().starts_with(&perm_prefix))
                .map(|g| g.permission)
                .collect();
            (plugin_id, perms)
        })
        .collect();

    for (plugin_id, perms) in revocations {
        for perm in &perms {
            let _ = mgr.permissions.revoke(&plugin_id, perm);
        }
    }

    match mgr.remove_extension(&ext_id) {
        Ok(()) => {
            lifecycle_events::emit(Some(&app), LifecycleEvent::ExtensionRemoved {
                ext_id,
            });
            Ok(())
        }
        Err(e) => {
            lifecycle_events::emit(Some(&app), LifecycleEvent::ExtensionError {
                ext_id,
                action: "removing".into(),
                message: e.to_string(),
            });
            Err(e.to_string())
        }
    }
}

/// Preview an extension from the marketplace (fetch manifest without installing).
#[tauri::command]
pub async fn extension_preview(
    manifest_url: String,
) -> Result<crate::extensions::manifest::ExtensionManifest, String> {
    crate::plugin_manager::registry::fetch_extension_manifest(&manifest_url)
        .await
        .map_err(|e| e.to_string())
}

/// Read the `[package] name` from a Cargo.toml file.
fn read_cargo_package_name(cargo_toml: &std::path::Path) -> Result<String, String> {
    let contents = std::fs::read_to_string(cargo_toml)
        .map_err(|e| format!("Failed to read Cargo.toml: {}", e))?;

    let mut in_package = false;
    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_package = trimmed == "[package]";
            continue;
        }
        if in_package {
            if let Some(rest) = trimmed.strip_prefix("name") {
                let rest = rest.trim_start();
                if let Some(rest) = rest.strip_prefix('=') {
                    let name = rest.trim().trim_matches('"').trim_matches('\'');
                    if !name.is_empty() {
                        return Ok(name.to_string());
                    }
                }
            }
        }
    }

    Err("Could not find [package] name in Cargo.toml".into())
}

/// Build a Cargo extension project in release mode.
/// Returns the path to the built binary on success.
async fn cargo_build_extension(
    manifest_dir: &std::path::Path,
) -> Result<std::path::PathBuf, String> {
    let cargo_toml = manifest_dir.join("Cargo.toml");
    if !cargo_toml.exists() {
        return Err(format!(
            "No Cargo.toml found in {}",
            manifest_dir.display()
        ));
    }

    let crate_name = read_cargo_package_name(&cargo_toml)?;

    log::info!("Building extension '{}' with cargo build --release", crate_name);

    let output = tokio::process::Command::new("cargo")
        .args(["build", "--release"])
        .current_dir(manifest_dir)
        .output()
        .await
        .map_err(|e| format!("Failed to run cargo build: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("cargo build failed:\n{}", stderr));
    }

    let binary_name = if cfg!(target_os = "windows") {
        format!("{}.exe", crate_name)
    } else {
        crate_name.clone()
    };

    let binary_path = manifest_dir.join("target").join("release").join(&binary_name);
    if !binary_path.exists() {
        return Err(format!(
            "Build succeeded but binary not found at {}",
            binary_path.display()
        ));
    }

    log::info!("Built extension binary: {}", binary_path.display());
    Ok(binary_path)
}

/// Install an extension from a local manifest (for development).
/// If the manifest has no binary for the current platform and a `Cargo.toml`
/// exists alongside it, the extension is built from source first.
#[tauri::command]
pub async fn extension_install_local(
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
    manifest_path: String,
) -> Result<InstalledExtension, String> {
    use crate::extensions::manifest::ExtensionManifest;

    // Read the manifest to get the ext_id for events before the install
    let manifest_data = std::fs::read_to_string(&manifest_path)
        .map_err(|e| format!("Failed to read manifest: {}", e))?;
    let manifest: ExtensionManifest =
        serde_json::from_str(&manifest_data)
            .map_err(|e| format!("Invalid manifest: {}", e))?;
    let ext_id = manifest.id.clone();

    lifecycle_events::emit(Some(&app), LifecycleEvent::ExtensionInstalling {
        ext_id: ext_id.clone(),
    });

    // If no binary for current platform, try building from source with cargo
    let manifest_dir = std::path::Path::new(&manifest_path)
        .parent()
        .unwrap_or(std::path::Path::new("."));
    let needs_build = manifest.binary_for_current_platform().is_none()
        && manifest_dir.join("Cargo.toml").exists();

    let binary_override = if needs_build {
        match cargo_build_extension(manifest_dir).await {
            Ok(path) => Some(path),
            Err(e) => {
                lifecycle_events::emit(Some(&app), LifecycleEvent::ExtensionError {
                    ext_id,
                    action: "building".into(),
                    message: e.clone(),
                });
                return Err(e);
            }
        }
    } else {
        None
    };

    let mut mgr = state.write().await;
    match mgr.install_extension_local(
        std::path::Path::new(&manifest_path),
        binary_override.as_deref(),
    ) {
        Ok(installed) => {
            if let Some(status) = build_extension_status(&mgr, &ext_id) {
                lifecycle_events::emit(Some(&app), LifecycleEvent::ExtensionInstalled {
                    extension: status,
                });
            }
            Ok(installed)
        }
        Err(e) => {
            lifecycle_events::emit(Some(&app), LifecycleEvent::ExtensionError {
                ext_id,
                action: "installing".into(),
                message: e.to_string(),
            });
            Err(e.to_string())
        }
    }
}

/// Search the extension marketplace.
#[tauri::command]
pub async fn extension_marketplace_search(
    state: tauri::State<'_, AppState>,
    query: String,
) -> Result<Vec<ExtensionRegistryEntry>, String> {
    let mgr = state.read().await;
    Ok(mgr.search_extension_marketplace(&query))
}

// -- Extension Resource CRUD commands --
// These delegate to the extension process via JSON-RPC `resources.*` methods.

#[derive(Debug, Deserialize)]
pub struct ResourceListParams {
    #[serde(default)]
    pub page: Option<u32>,
    #[serde(default)]
    pub page_size: Option<u32>,
    #[serde(default)]
    pub sort_by: Option<String>,
    #[serde(default)]
    pub sort_order: Option<String>,
}

/// List resources of a given type from an extension.
#[tauri::command]
pub async fn extension_resource_list(
    state: tauri::State<'_, AppState>,
    ext_id: String,
    resource_type: String,
    params: Option<ResourceListParams>,
) -> Result<Value, String> {
    let mgr = state.read().await;
    let ext = mgr
        .extensions
        .get_arc(&ext_id)
        .ok_or_else(|| format!("Extension '{}' not running", ext_id))?;
    drop(mgr);

    let mut rpc_params = serde_json::json!({ "resource_type": resource_type });
    if let Some(p) = params {
        if let Some(page) = p.page {
            rpc_params["page"] = serde_json::json!(page);
        }
        if let Some(ps) = p.page_size {
            rpc_params["page_size"] = serde_json::json!(ps);
        }
        if let Some(sb) = p.sort_by {
            rpc_params["sort_by"] = serde_json::json!(sb);
        }
        if let Some(so) = p.sort_order {
            rpc_params["sort_order"] = serde_json::json!(so);
        }
    }

    let result = ext
        .execute("__resources_list", rpc_params)
        .await
        .map_err(|e| e.to_string())?;
    Ok(result.data)
}

/// Get a single resource by ID from an extension.
#[tauri::command]
pub async fn extension_resource_get(
    state: tauri::State<'_, AppState>,
    ext_id: String,
    resource_type: String,
    resource_id: String,
) -> Result<Value, String> {
    let mgr = state.read().await;
    let ext = mgr
        .extensions
        .get_arc(&ext_id)
        .ok_or_else(|| format!("Extension '{}' not running", ext_id))?;
    drop(mgr);

    let rpc_params = serde_json::json!({
        "resource_type": resource_type,
        "id": resource_id,
    });

    let result = ext
        .execute("__resources_get", rpc_params)
        .await
        .map_err(|e| e.to_string())?;
    Ok(result.data)
}

/// Create a new resource via an extension.
#[tauri::command]
pub async fn extension_resource_create(
    state: tauri::State<'_, AppState>,
    ext_id: String,
    resource_type: String,
    data: Value,
) -> Result<Value, String> {
    let mgr = state.read().await;
    let ext = mgr
        .extensions
        .get_arc(&ext_id)
        .ok_or_else(|| format!("Extension '{}' not running", ext_id))?;
    drop(mgr);

    let rpc_params = serde_json::json!({
        "resource_type": resource_type,
        "data": data,
    });

    let result = ext
        .execute("__resources_create", rpc_params)
        .await
        .map_err(|e| e.to_string())?;
    Ok(result.data)
}

/// Update a resource via an extension.
#[tauri::command]
pub async fn extension_resource_update(
    state: tauri::State<'_, AppState>,
    ext_id: String,
    resource_type: String,
    resource_id: String,
    data: Value,
) -> Result<Value, String> {
    let mgr = state.read().await;
    let ext = mgr
        .extensions
        .get_arc(&ext_id)
        .ok_or_else(|| format!("Extension '{}' not running", ext_id))?;
    drop(mgr);

    let rpc_params = serde_json::json!({
        "resource_type": resource_type,
        "id": resource_id,
        "data": data,
    });

    let result = ext
        .execute("__resources_update", rpc_params)
        .await
        .map_err(|e| e.to_string())?;
    Ok(result.data)
}

/// Delete a resource via an extension.
#[tauri::command]
pub async fn extension_resource_delete(
    state: tauri::State<'_, AppState>,
    ext_id: String,
    resource_type: String,
    resource_id: String,
) -> Result<Value, String> {
    let mgr = state.read().await;
    let ext = mgr
        .extensions
        .get_arc(&ext_id)
        .ok_or_else(|| format!("Extension '{}' not running", ext_id))?;
    drop(mgr);

    let rpc_params = serde_json::json!({
        "resource_type": resource_type,
        "id": resource_id,
    });

    let result = ext
        .execute("__resources_delete", rpc_params)
        .await
        .map_err(|e| e.to_string())?;
    Ok(result.data)
}
