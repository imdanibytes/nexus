use crate::extensions::registry::ExtensionRegistry;
use crate::extensions::RiskLevel;
use crate::permissions::Permission;
use crate::AppState;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ExtensionOperationStatus {
    pub name: String,
    pub description: String,
    pub risk_level: RiskLevel,
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
    pub consumers: Vec<ExtensionConsumer>,
}

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
            })
            .collect();

        // Find which installed plugins consume this extension
        let mut consumers = Vec::new();
        for plugin in mgr.storage.list() {
            if let Some(ops) = plugin.manifest.extensions.get(&ext_info.id) {
                // Check if all extension permissions for this plugin are granted
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

        result.push(ExtensionStatus {
            id: ext_info.id,
            display_name: ext_info.display_name,
            description: ext_info.description,
            operations,
            consumers,
        });
    }

    Ok(result)
}
