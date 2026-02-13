use crate::extensions::capability::Capability;
use crate::extensions::registry::ExtensionRegistry;
use crate::extensions::storage::InstalledExtension;
use crate::extensions::RiskLevel;
use crate::permissions::Permission;
use crate::plugin_manager::registry::ExtensionRegistryEntry;
use crate::AppState;
use serde::Serialize;

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
            if let Some(ops) = plugin.manifest.extensions.get(&ext_info.id) {
                let all_granted = ops.iter().all(|op_name| {
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

        result.push(ExtensionStatus {
            id: ext_info.id,
            display_name: ext_info.display_name,
            description: ext_info.description,
            operations,
            capabilities: ext_info.capabilities,
            consumers,
            installed: installed_ext.is_some(),
            enabled: installed_ext.map_or(false, |e| e.enabled),
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
    manifest_url: String,
) -> Result<InstalledExtension, String> {
    // Fetch manifest
    let manifest = crate::plugin_manager::registry::fetch_extension_manifest(&manifest_url)
        .await
        .map_err(|e| e.to_string())?;

    let mut mgr = state.write().await;
    mgr.extension_loader
        .install(manifest)
        .await
        .map_err(|e| e.to_string())
}

/// Enable an installed extension (spawns process, registers in runtime).
#[tauri::command]
pub async fn extension_enable(
    state: tauri::State<'_, AppState>,
    ext_id: String,
) -> Result<(), String> {
    let mut mgr = state.write().await;
    mgr.enable_extension(&ext_id)
        .map_err(|e| e.to_string())
}

/// Disable an extension (stops process, unregisters from runtime).
#[tauri::command]
pub async fn extension_disable(
    state: tauri::State<'_, AppState>,
    ext_id: String,
) -> Result<(), String> {
    let mut mgr = state.write().await;
    mgr.disable_extension(&ext_id)
        .map_err(|e| e.to_string())
}

/// Remove an extension entirely (stop, delete files, unregister).
#[tauri::command]
pub async fn extension_remove(
    state: tauri::State<'_, AppState>,
    ext_id: String,
) -> Result<(), String> {
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

    mgr.remove_extension(&ext_id)
        .map_err(|e| e.to_string())
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

/// Install an extension from a local manifest (for development).
/// Binary path is resolved from the manifest's `binaries` field.
#[tauri::command]
pub async fn extension_install_local(
    state: tauri::State<'_, AppState>,
    manifest_path: String,
) -> Result<InstalledExtension, String> {
    let mut mgr = state.write().await;
    mgr.extension_loader
        .install_local(std::path::Path::new(&manifest_path))
        .map_err(|e| e.to_string())
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
