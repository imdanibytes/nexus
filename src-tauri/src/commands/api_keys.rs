use crate::api_keys::{ApiKey, ApiKeyStore};
use serde::Serialize;

#[derive(Serialize)]
pub struct GeneratedApiKey {
    pub key: ApiKey,
    pub raw: String,
}

#[tauri::command]
pub async fn api_key_list(
    store: tauri::State<'_, ApiKeyStore>,
) -> Result<Vec<ApiKey>, String> {
    Ok(store.list())
}

#[tauri::command]
pub async fn api_key_generate(
    store: tauri::State<'_, ApiKeyStore>,
    name: String,
) -> Result<GeneratedApiKey, String> {
    let (key, raw) = store.generate(&name);
    Ok(GeneratedApiKey { key, raw })
}

#[tauri::command]
pub async fn api_key_revoke(
    store: tauri::State<'_, ApiKeyStore>,
    id: String,
) -> Result<(), String> {
    if store.revoke(&id) {
        Ok(())
    } else {
        Err("API key not found".into())
    }
}

#[tauri::command]
pub async fn api_key_get_default(
    store: tauri::State<'_, ApiKeyStore>,
) -> Result<Option<String>, String> {
    Ok(store.get_default_raw())
}

#[tauri::command]
pub async fn api_key_regenerate_default(
    store: tauri::State<'_, ApiKeyStore>,
) -> Result<GeneratedApiKey, String> {
    let (key, raw) = store.regenerate_default();
    Ok(GeneratedApiKey { key, raw })
}
