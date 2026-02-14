use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};
use serde::Serialize;
use std::path::PathBuf;

use super::middleware::AuthenticatedPlugin;
use crate::AppState;

/// Maximum size for a single value (256 KB).
const MAX_VALUE_SIZE: usize = 256 * 1024;
/// Maximum number of keys per plugin.
const MAX_KEYS_PER_PLUGIN: usize = 1000;

fn storage_dir(data_dir: &std::path::Path, plugin_id: &str) -> PathBuf {
    data_dir.join("plugin_data").join(plugin_id)
}

/// Validate a storage key: alphanumeric, hyphens, underscores, dots. Max 128 chars.
fn validate_key(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= 128
        && key
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.')
        && !key.contains("..")
}

fn key_path(data_dir: &std::path::Path, plugin_id: &str, key: &str) -> PathBuf {
    storage_dir(data_dir, plugin_id).join(format!("{}.json", key))
}

// ── Handlers ─────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct StorageKeys {
    pub keys: Vec<String>,
}

/// List all storage keys for the authenticated plugin.
///
/// `GET /v1/storage`
pub async fn list_keys(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedPlugin>,
) -> Result<Json<StorageKeys>, StatusCode> {
    let mgr = state.read().await;
    let dir = storage_dir(&mgr.data_dir, &auth.plugin_id);

    let mut keys = Vec::new();
    if dir.exists() {
        let entries = std::fs::read_dir(&dir).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                if let Some(key) = name.strip_suffix(".json") {
                    keys.push(key.to_string());
                }
            }
        }
    }
    keys.sort();
    Ok(Json(StorageKeys { keys }))
}

/// Get a value by key.
///
/// `GET /v1/storage/:key`
pub async fn get_value(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedPlugin>,
    Path(key): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if !validate_key(&key) {
        return Err(StatusCode::BAD_REQUEST);
    }

    let mgr = state.read().await;
    let path = key_path(&mgr.data_dir, &auth.plugin_id, &key);

    if !path.exists() {
        return Err(StatusCode::NOT_FOUND);
    }

    let data = std::fs::read_to_string(&path).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let value: serde_json::Value =
        serde_json::from_str(&data).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(value))
}

/// Set a value by key.
///
/// `PUT /v1/storage/:key`
pub async fn put_value(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedPlugin>,
    Path(key): Path<String>,
    Json(value): Json<serde_json::Value>,
) -> Result<StatusCode, StatusCode> {
    if !validate_key(&key) {
        return Err(StatusCode::BAD_REQUEST);
    }

    let serialized =
        serde_json::to_string_pretty(&value).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if serialized.len() > MAX_VALUE_SIZE {
        return Err(StatusCode::PAYLOAD_TOO_LARGE);
    }

    let mgr = state.read().await;
    let dir = storage_dir(&mgr.data_dir, &auth.plugin_id);

    // Enforce key count limit (only check when creating a new key)
    let path = key_path(&mgr.data_dir, &auth.plugin_id, &key);
    if !path.exists() && dir.exists() {
        let count = std::fs::read_dir(&dir)
            .map(|entries| entries.count())
            .unwrap_or(0);
        if count >= MAX_KEYS_PER_PLUGIN {
            return Err(StatusCode::INSUFFICIENT_STORAGE);
        }
    }

    std::fs::create_dir_all(&dir).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    std::fs::write(&path, serialized).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::OK)
}

/// Delete a value by key.
///
/// `DELETE /v1/storage/:key`
pub async fn delete_value(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedPlugin>,
    Path(key): Path<String>,
) -> Result<StatusCode, StatusCode> {
    if !validate_key(&key) {
        return Err(StatusCode::BAD_REQUEST);
    }

    let mgr = state.read().await;
    let path = key_path(&mgr.data_dir, &auth.plugin_id, &key);

    if path.exists() {
        std::fs::remove_file(&path).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    Ok(StatusCode::OK)
}

/// Delete all storage for a plugin. Called during plugin uninstall.
pub fn remove_plugin_storage(data_dir: &std::path::Path, plugin_id: &str) {
    let dir = storage_dir(data_dir, plugin_id);
    if dir.exists() {
        if let Err(e) = std::fs::remove_dir_all(&dir) {
            log::warn!("Failed to remove storage for {}: {}", plugin_id, e);
        }
    }
}

/// Get total storage usage for a plugin in bytes.
pub fn plugin_storage_bytes(data_dir: &std::path::Path, plugin_id: &str) -> u64 {
    let dir = storage_dir(data_dir, plugin_id);
    if !dir.exists() {
        return 0;
    }
    std::fs::read_dir(&dir)
        .map(|entries| {
            entries
                .flatten()
                .filter_map(|e| e.metadata().ok())
                .map(|m| m.len())
                .sum()
        })
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_keys() {
        assert!(validate_key("cookies"));
        assert!(validate_key("my-data"));
        assert!(validate_key("settings_v2"));
        assert!(validate_key("com.example.state"));
        assert!(validate_key("a"));
    }

    #[test]
    fn invalid_keys() {
        assert!(!validate_key(""));
        assert!(!validate_key("../etc/passwd"));
        assert!(!validate_key("foo/bar"));
        assert!(!validate_key("a".repeat(129).as_str()));
        assert!(!validate_key("hello world"));
        assert!(!validate_key("key..traversal"));
    }

    #[test]
    fn storage_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let plugin_id = "test-plugin";
        let key = "my-key";

        let path = key_path(dir.path(), plugin_id, key);
        assert!(!path.exists());

        // Write
        let sdir = storage_dir(dir.path(), plugin_id);
        std::fs::create_dir_all(&sdir).unwrap();
        std::fs::write(&path, r#"{"hello":"world"}"#).unwrap();

        // Read
        let data = std::fs::read_to_string(&path).unwrap();
        let val: serde_json::Value = serde_json::from_str(&data).unwrap();
        assert_eq!(val["hello"], "world");

        // Delete
        std::fs::remove_file(&path).unwrap();
        assert!(!path.exists());
    }

    #[test]
    fn remove_all_storage() {
        let dir = tempfile::tempdir().unwrap();
        let plugin_id = "test-plugin";

        let sdir = storage_dir(dir.path(), plugin_id);
        std::fs::create_dir_all(&sdir).unwrap();
        std::fs::write(sdir.join("a.json"), "1").unwrap();
        std::fs::write(sdir.join("b.json"), "2").unwrap();

        remove_plugin_storage(dir.path(), plugin_id);
        assert!(!sdir.exists());
    }

    #[test]
    fn storage_bytes_calculation() {
        let dir = tempfile::tempdir().unwrap();
        let plugin_id = "test-plugin";

        assert_eq!(plugin_storage_bytes(dir.path(), plugin_id), 0);

        let sdir = storage_dir(dir.path(), plugin_id);
        std::fs::create_dir_all(&sdir).unwrap();
        std::fs::write(sdir.join("a.json"), "hello").unwrap(); // 5 bytes

        assert_eq!(plugin_storage_bytes(dir.path(), plugin_id), 5);
    }
}
