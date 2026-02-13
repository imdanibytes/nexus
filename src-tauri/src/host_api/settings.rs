use axum::{extract::State, http::StatusCode, Extension, Json};
use std::collections::HashMap;

use super::middleware::AuthenticatedPlugin;
use crate::AppState;

/// Get settings for the authenticated plugin.
///
/// Returns a flat key-value map. Defaults from the manifest are
/// applied first, then any explicitly saved overrides.
#[utoipa::path(
    get,
    path = "/api/v1/settings",
    tag = "settings",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Plugin settings", body = HashMap<String, serde_json::Value>),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn get_settings(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedPlugin>,
) -> Result<Json<HashMap<String, serde_json::Value>>, StatusCode> {
    let mgr = state.read().await;

    // Start with defaults from the manifest, then overlay stored values
    let mut values = HashMap::new();

    if let Some(plugin) = mgr.storage.get(&auth.plugin_id) {
        for def in &plugin.manifest.settings {
            if let Some(default) = &def.default {
                values.insert(def.key.clone(), default.clone());
            }
        }
    }

    // Overlay any explicitly saved values
    let stored = mgr.plugin_settings.get(&auth.plugin_id);
    for (k, v) in stored {
        values.insert(k, v);
    }

    Ok(Json(values))
}

/// Update settings for the authenticated plugin.
///
/// Accepts a flat key-value map. Values are validated against the manifest's
/// settings schema: unknown keys are rejected, value types must match.
#[utoipa::path(
    put,
    path = "/api/v1/settings",
    tag = "settings",
    security(("bearer_auth" = [])),
    request_body = HashMap<String, serde_json::Value>,
    responses(
        (status = 200, description = "Settings saved"),
        (status = 400, description = "Validation error"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn put_settings(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedPlugin>,
    Json(values): Json<HashMap<String, serde_json::Value>>,
) -> Result<StatusCode, StatusCode> {
    let mut mgr = state.write().await;

    // Validate against manifest schema
    if let Some(plugin) = mgr.storage.get(&auth.plugin_id) {
        let schema = &plugin.manifest.settings;

        for (key, value) in &values {
            let def = schema.iter().find(|d| d.key == *key);
            let def = match def {
                Some(d) => d,
                None => return Err(StatusCode::BAD_REQUEST), // Unknown key
            };

            // Type validation
            let valid = match def.setting_type.as_str() {
                "string" => value.is_string(),
                "number" => value.is_number(),
                "boolean" => value.is_boolean(),
                "select" => {
                    if let Some(s) = value.as_str() {
                        def.options
                            .as_ref()
                            .is_some_and(|opts| opts.contains(&s.to_string()))
                    } else {
                        false
                    }
                }
                _ => false,
            };

            if !valid {
                return Err(StatusCode::BAD_REQUEST);
            }
        }
    }

    mgr.plugin_settings
        .set(&auth.plugin_id, values)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::OK)
}
