use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::extensions::registry::ExtensionInfo;
use crate::extensions::validation::validate_input;
use crate::extensions::RiskLevel;
use crate::permissions::Permission;
use crate::AppState;

use super::approval::ApprovalBridge;
use super::middleware::AuthenticatedPlugin;

/// Response for GET /v1/extensions
#[derive(Serialize)]
pub struct ListExtensionsResponse {
    pub extensions: Vec<ExtensionInfo>,
}

/// Request body for POST /v1/extensions/{ext_id}/{operation}
#[derive(Deserialize)]
pub struct CallExtensionRequest {
    #[serde(default = "default_input")]
    pub input: Value,
}

fn default_input() -> Value {
    Value::Object(serde_json::Map::new())
}

/// Response for POST /v1/extensions/{ext_id}/{operation}
#[derive(Serialize)]
pub struct CallExtensionResponse {
    pub success: bool,
    pub data: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Error response body
#[derive(Serialize)]
pub struct ExtensionErrorResponse {
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

fn error_response(status: StatusCode, error: &str, details: Option<String>) -> (StatusCode, Json<ExtensionErrorResponse>) {
    (
        status,
        Json(ExtensionErrorResponse {
            error: error.to_string(),
            details,
        }),
    )
}

/// GET /v1/extensions — list all registered extensions and their operations.
pub async fn list_extensions(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthenticatedPlugin>,
) -> Json<ListExtensionsResponse> {
    let mgr = state.read().await;
    let extensions = mgr.extensions.list();
    Json(ListExtensionsResponse { extensions })
}

/// POST /v1/extensions/{ext_id}/{operation} — execute an extension operation.
///
/// Permission checking is done in the handler (not middleware) because the
/// required permission scope is dynamic based on path parameters.
pub async fn call_extension(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedPlugin>,
    Extension(bridge): Extension<Arc<ApprovalBridge>>,
    Path((ext_id, operation)): Path<(String, String)>,
    Json(body): Json<CallExtensionRequest>,
) -> Result<Json<CallExtensionResponse>, (StatusCode, Json<ExtensionErrorResponse>)> {
    let mgr = state.read().await;

    // 1. Look up extension
    let ext = mgr.extensions.get(&ext_id).ok_or_else(|| {
        error_response(
            StatusCode::NOT_FOUND,
            "Extension not found",
            Some(format!("No extension with ID '{}'", ext_id)),
        )
    })?;

    // 2. Look up operation
    let op_def = ext
        .operations()
        .into_iter()
        .find(|op| op.name == operation)
        .ok_or_else(|| {
            error_response(
                StatusCode::NOT_FOUND,
                "Operation not found",
                Some(format!(
                    "Extension '{}' has no operation '{}'",
                    ext_id, operation
                )),
            )
        })?;

    // 3. Check permission: "ext:{ext_id}:{operation}"
    let perm_string = crate::extensions::registry::ExtensionRegistry::permission_string(&ext_id, &operation);
    let required_perm = Permission::Extension(perm_string.clone());

    if !mgr.permissions.has_permission(&auth.plugin_id, &required_perm) {
        log::warn!(
            "AUDIT DENIED plugin={} extension={} operation={} reason=missing_permission perm={}",
            auth.plugin_id,
            ext_id,
            operation,
            perm_string,
        );
        return Err(error_response(
            StatusCode::FORBIDDEN,
            "Permission denied",
            Some(format!("Plugin '{}' lacks permission '{}'", auth.plugin_id, perm_string)),
        ));
    }

    // 4. Validate input against the operation's JSON Schema
    validate_input(&op_def.input_schema, &body.input).map_err(|e| {
        error_response(
            StatusCode::BAD_REQUEST,
            "Invalid input",
            Some(e),
        )
    })?;

    // 5. Runtime approval for high-risk operations
    if op_def.risk_level == RiskLevel::High {
        let mut context = std::collections::HashMap::new();
        context.insert("extension".to_string(), ext_id.clone());
        context.insert("operation".to_string(), operation.clone());
        context.insert("risk_level".to_string(), "high".to_string());
        // Include a summary of the input for the approval dialog
        if let Value::Object(map) = &body.input {
            for (k, v) in map {
                context.insert(format!("input.{}", k), v.to_string());
            }
        }

        let request = super::approval::ApprovalRequest {
            id: uuid::Uuid::new_v4().to_string(),
            plugin_id: auth.plugin_id.clone(),
            plugin_name: auth.plugin_id.clone(),
            category: format!("extension:{}", ext_id),
            permission: perm_string.clone(),
            context,
        };

        // Drop the read lock before awaiting approval (which can take up to 60s)
        drop(mgr);

        match bridge.request_approval(request).await {
            super::approval::ApprovalDecision::Approve
            | super::approval::ApprovalDecision::ApproveOnce => {
                // Re-acquire read lock
                let mgr = state.read().await;
                let ext = mgr.extensions.get(&ext_id).ok_or_else(|| {
                    error_response(StatusCode::INTERNAL_SERVER_ERROR, "Extension disappeared", None)
                })?;

                let result = ext.execute(&operation, body.input).await.map_err(|e| {
                    log::error!(
                        "Extension error: ext={} op={} plugin={} error={}",
                        ext_id, operation, auth.plugin_id, e
                    );
                    error_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Extension execution failed",
                        Some(e.to_string()),
                    )
                })?;

                log::info!(
                    "AUDIT plugin={} extension={} operation={} status=200",
                    auth.plugin_id, ext_id, operation
                );

                return Ok(Json(CallExtensionResponse {
                    success: result.success,
                    data: result.data,
                    message: result.message,
                }));
            }
            super::approval::ApprovalDecision::Deny => {
                return Err(error_response(
                    StatusCode::FORBIDDEN,
                    "Runtime approval denied",
                    Some("User denied the high-risk operation".to_string()),
                ));
            }
        }
    }

    // 6. Execute (non-high-risk path, or low/medium risk)
    let result = ext.execute(&operation, body.input).await.map_err(|e| {
        log::error!(
            "Extension error: ext={} op={} plugin={} error={}",
            ext_id, operation, auth.plugin_id, e
        );
        error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Extension execution failed",
            Some(e.to_string()),
        )
    })?;

    log::info!(
        "AUDIT plugin={} extension={} operation={} status=200",
        auth.plugin_id, ext_id, operation
    );

    Ok(Json(CallExtensionResponse {
        success: result.success,
        data: result.data,
        message: result.message,
    }))
}
