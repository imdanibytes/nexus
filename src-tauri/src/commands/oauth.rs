use std::sync::Arc;

use crate::oauth::store::OAuthStore;
use crate::oauth::types::OAuthClientInfo;

#[tauri::command]
pub async fn oauth_list_clients(
    store: tauri::State<'_, Arc<OAuthStore>>,
) -> Result<Vec<OAuthClientInfo>, String> {
    Ok(store.list_clients().iter().map(OAuthClientInfo::from).collect())
}

#[tauri::command]
pub async fn oauth_revoke_client(
    store: tauri::State<'_, Arc<OAuthStore>>,
    client_id: String,
) -> Result<(), String> {
    store.revoke_client(&client_id);
    Ok(())
}
