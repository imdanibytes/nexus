use crate::plugin_manager::registry::{RegistryKind, RegistrySource, RegistryTrust};
use crate::AppState;

#[tauri::command]
pub async fn registry_list(state: tauri::State<'_, AppState>) -> Result<Vec<RegistrySource>, String> {
    let mgr = state.read().await;
    Ok(mgr.registry_store.list().to_vec())
}

#[tauri::command]
pub async fn registry_add(
    state: tauri::State<'_, AppState>,
    name: String,
    kind: String,
    url: String,
) -> Result<RegistrySource, String> {
    let mut mgr = state.write().await;

    let registry_kind = match kind.as_str() {
        "remote" => RegistryKind::Remote,
        "local" => RegistryKind::Local,
        _ => return Err(format!("Invalid registry kind: {}. Use 'remote' or 'local'", kind)),
    };

    // Generate a slug-style ID from the name
    let id = name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string();

    let source = RegistrySource {
        id,
        name,
        kind: registry_kind,
        url,
        enabled: true,
        trust: RegistryTrust::Community,
    };

    mgr.registry_store.add(source.clone()).map_err(|e| e.to_string())?;
    Ok(source)
}

/// Built-in registries that cannot be removed by the user.
const PROTECTED_REGISTRIES: &[&str] = &["nexus-community", "nexus-mcp-local"];

#[tauri::command]
pub async fn registry_remove(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    if PROTECTED_REGISTRIES.contains(&id.as_str()) {
        return Err(format!("Cannot remove built-in registry '{}'", id));
    }
    let mut mgr = state.write().await;
    mgr.registry_store.remove(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn registry_toggle(
    state: tauri::State<'_, AppState>,
    id: String,
    enabled: bool,
) -> Result<(), String> {
    let mut mgr = state.write().await;
    mgr.registry_store.toggle(&id, enabled).map_err(|e| e.to_string())
}
