use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

use crate::extensions::validation::validate_input;
use crate::extensions::RiskLevel;
use crate::permissions::{Permission, PermissionState};
use crate::AppState;

use super::approval::ApprovalBridge;
use super::middleware::AuthenticatedPlugin;

/// A plugin's view of an extension it declared as a dependency.
#[derive(Serialize, ToSchema)]
pub struct PluginExtensionView {
    pub id: String,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub available: bool,
    pub operations: Vec<PluginOperationView>,
}

/// A plugin's view of an operation on a declared extension.
#[derive(Serialize, ToSchema)]
pub struct PluginOperationView {
    pub name: String,
    pub permitted: bool,
    pub available: bool,
}

/// Response for GET /v1/extensions
#[derive(Serialize, ToSchema)]
pub struct ListExtensionsResponse {
    pub extensions: Vec<PluginExtensionView>,
}

/// Request body for POST /v1/extensions/{ext_id}/{operation}
#[derive(Deserialize, ToSchema)]
pub struct CallExtensionRequest {
    #[serde(default = "default_input")]
    pub input: Value,
}

fn default_input() -> Value {
    Value::Object(serde_json::Map::new())
}

/// Response for POST /v1/extensions/{ext_id}/{operation}
#[derive(Serialize, ToSchema)]
pub struct CallExtensionResponse {
    pub success: bool,
    pub data: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Error response body
#[derive(Serialize, ToSchema)]
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

/// List extensions declared by the calling plugin.
///
/// Only returns extensions the plugin declared in its manifest `"extensions"` field.
/// Includes availability status (is it installed and running?) and per-operation
/// permission status. Plugins cannot see extensions they didn't declare.
#[utoipa::path(
    get,
    path = "/api/v1/extensions",
    tag = "extensions",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Extensions declared by this plugin", body = ListExtensionsResponse),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn list_extensions(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedPlugin>,
) -> Json<ListExtensionsResponse> {
    let mgr = state.read().await;

    // Get the calling plugin's declared extension dependencies
    let declared = mgr
        .storage
        .get(&auth.plugin_id)
        .map(|p| p.manifest.extensions.clone())
        .unwrap_or_default();

    let mut views = Vec::new();
    for (ext_id, deps) in &declared {
        let registered = mgr.extensions.get(ext_id);
        let available = registered.is_some();

        let (display_name, description) = registered
            .map(|e| (Some(e.display_name().to_string()), Some(e.description().to_string())))
            .unwrap_or((None, None));

        let operations = deps
            .operation_names()
            .into_iter()
            .map(|op_name| {
                let perm_str = crate::extensions::registry::ExtensionRegistry::permission_string(ext_id, &op_name);
                let permitted = mgr.permissions.has_permission(&auth.plugin_id, &Permission::Extension(perm_str));
                let op_available = available
                    && registered.is_some_and(|e| e.operations().iter().any(|o| o.name == op_name));

                PluginOperationView {
                    name: op_name,
                    permitted,
                    available: op_available,
                }
            })
            .collect();

        views.push(PluginExtensionView {
            id: ext_id.clone(),
            display_name,
            description,
            available,
            operations,
        });
    }

    views.sort_by(|a, b| a.id.cmp(&b.id));
    Json(ListExtensionsResponse { extensions: views })
}

/// Execute an extension operation.
///
/// Three-layer security model:
/// 1. PERMISSION: Does this plugin have `ext:{ext_id}:{operation}`?
/// 2. SCOPE: If the operation declares `scope_key`, is the scope value approved?
/// 3. RISK: If risk_level is high, per-invocation runtime approval.
#[utoipa::path(
    post,
    path = "/api/v1/extensions/{ext_id}/{operation}",
    tag = "extensions",
    security(("bearer_auth" = [])),
    params(
        ("ext_id" = String, Path, description = "Extension ID"),
        ("operation" = String, Path, description = "Operation name"),
    ),
    request_body = CallExtensionRequest,
    responses(
        (status = 200, description = "Operation result", body = CallExtensionResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden — extension not found, missing permission, or scope denied"),
    )
)]
pub async fn call_extension(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedPlugin>,
    Extension(bridge): Extension<Arc<ApprovalBridge>>,
    Path((ext_id, operation)): Path<(String, String)>,
    Json(body): Json<CallExtensionRequest>,
) -> Result<Json<CallExtensionResponse>, (StatusCode, Json<ExtensionErrorResponse>)> {
    let mgr = state.read().await;

    // 1. Look up extension (403 instead of 404 to avoid leaking extension existence)
    let ext = mgr.extensions.get(&ext_id).ok_or_else(|| {
        error_response(
            StatusCode::FORBIDDEN,
            "Forbidden",
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
                StatusCode::FORBIDDEN,
                "Forbidden",
                Some(format!(
                    "Extension '{}' has no operation '{}'",
                    ext_id, operation
                )),
            )
        })?;

    // 3. LAYER 1 — PERMISSION: Check "ext:{ext_id}:{operation}" with three-state model
    let perm_string = crate::extensions::registry::ExtensionRegistry::permission_string(&ext_id, &operation);
    let required_perm = Permission::Extension(perm_string.clone());

    match mgr.permissions.get_state(&auth.plugin_id, &required_perm) {
        Some(PermissionState::Active) => {
            // Proceed to scope/risk checks below
        }
        Some(PermissionState::Deferred) => {
            // JIT approval: user skipped this at install, ask now
            let plugin_name = mgr
                .storage
                .get(&auth.plugin_id)
                .map(|p| p.manifest.name.clone())
                .unwrap_or_else(|| auth.plugin_id.clone());

            let perm_desc = op_def.description.clone();
            let mut context = std::collections::HashMap::new();
            context.insert("extension".to_string(), ext_id.clone());
            context.insert("extension_display_name".to_string(), ext.display_name().to_string());
            context.insert("operation".to_string(), operation.clone());
            context.insert("operation_description".to_string(), perm_desc);
            context.insert("permission".to_string(), perm_string.clone());

            let request = super::approval::ApprovalRequest {
                id: uuid::Uuid::new_v4().to_string(),
                plugin_id: auth.plugin_id.clone(),
                plugin_name,
                category: "deferred_permission".to_string(),
                permission: perm_string.clone(),
                context,
            };

            // Drop read lock before awaiting approval
            drop(mgr);

            match bridge.request_approval(request).await {
                super::approval::ApprovalDecision::Approve => {
                    // Persist: Deferred → Active
                    let mgr = state.read().await;
                    let _ = mgr.permissions.activate(&auth.plugin_id, &required_perm);
                    drop(mgr);

                    // Clone Arc, drop lock, execute lock-free
                    let mgr = state.read().await;
                    let ext_arc = mgr.extensions.get_arc(&ext_id).ok_or_else(|| {
                        error_response(StatusCode::INTERNAL_SERVER_ERROR, "Extension disappeared", None)
                    })?;
                    let op_def = ext_arc.operations().into_iter().find(|op| op.name == operation).ok_or_else(|| {
                        error_response(StatusCode::INTERNAL_SERVER_ERROR, "Operation disappeared", None)
                    })?;
                    drop(mgr);

                    return execute_and_respond(
                        ext_arc, &ext_id, &operation, &auth.plugin_id, body.input, &op_def, &bridge, &state,
                    ).await;
                }
                super::approval::ApprovalDecision::ApproveOnce => {
                    // One-time: don't persist, state stays Deferred
                    let mgr = state.read().await;
                    let ext_arc = mgr.extensions.get_arc(&ext_id).ok_or_else(|| {
                        error_response(StatusCode::INTERNAL_SERVER_ERROR, "Extension disappeared", None)
                    })?;
                    let op_def = ext_arc.operations().into_iter().find(|op| op.name == operation).ok_or_else(|| {
                        error_response(StatusCode::INTERNAL_SERVER_ERROR, "Operation disappeared", None)
                    })?;
                    drop(mgr);

                    return execute_and_respond(
                        ext_arc, &ext_id, &operation, &auth.plugin_id, body.input, &op_def, &bridge, &state,
                    ).await;
                }
                super::approval::ApprovalDecision::Deny => {
                    // Deny: Deferred → Revoked
                    let mgr = state.read().await;
                    let _ = mgr.permissions.revoke(&auth.plugin_id, &required_perm);
                    return Err(error_response(
                        StatusCode::FORBIDDEN,
                        "Permission denied",
                        Some(format!("User denied deferred permission '{}'", perm_string)),
                    ));
                }
            }
        }
        Some(PermissionState::Revoked) | None => {
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
    }

    // 4. Validate input against the operation's JSON Schema
    validate_input(&op_def.input_schema, &body.input).map_err(|e| {
        error_response(
            StatusCode::BAD_REQUEST,
            "Invalid input",
            Some(e),
        )
    })?;

    // 5. LAYER 2 — SCOPE: If operation has a scope_key, check approved_scopes
    if let Some(ref scope_key) = op_def.scope_key {
        if let Some(scope_value) = body.input.get(scope_key).and_then(|v| v.as_str()) {
            let approved_scopes = mgr.permissions.get_approved_scopes(&auth.plugin_id, &required_perm);

            match approved_scopes {
                None => {
                    // Unrestricted — no scope checking
                }
                Some(scopes) => {
                    if !scopes.iter().any(|s| s == scope_value) {
                        // Scope not approved — request runtime approval
                        let scope_desc = op_def.scope_description.as_deref().unwrap_or(scope_key);

                        let mut context = std::collections::HashMap::new();
                        context.insert("extension".to_string(), ext_id.clone());
                        context.insert("extension_display_name".to_string(), ext.display_name().to_string());
                        context.insert("operation".to_string(), operation.clone());
                        context.insert("operation_description".to_string(), op_def.description.clone());
                        context.insert("scope_key".to_string(), scope_key.clone());
                        context.insert("scope_value".to_string(), scope_value.to_string());
                        context.insert("scope_description".to_string(), scope_desc.to_string());

                        let plugin_name = mgr
                            .storage
                            .get(&auth.plugin_id)
                            .map(|p| p.manifest.name.clone())
                            .unwrap_or_else(|| auth.plugin_id.clone());

                        let request = super::approval::ApprovalRequest {
                            id: uuid::Uuid::new_v4().to_string(),
                            plugin_id: auth.plugin_id.clone(),
                            plugin_name,
                            category: format!("extension_scope:{}", ext_id),
                            permission: perm_string.clone(),
                            context,
                        };

                        // Drop read lock before awaiting approval
                        drop(mgr);

                        match bridge.request_approval(request).await {
                            super::approval::ApprovalDecision::Approve => {
                                // Persist the approved scope
                                let mgr = state.read().await;
                                let _ = mgr.permissions.add_approved_scope(
                                    &auth.plugin_id,
                                    &required_perm,
                                    scope_value.to_string(),
                                );
                                drop(mgr);

                                // Clone Arc, drop lock, execute lock-free
                                let mgr = state.read().await;
                                let ext_arc = mgr.extensions.get_arc(&ext_id).ok_or_else(|| {
                                    error_response(StatusCode::INTERNAL_SERVER_ERROR, "Extension disappeared", None)
                                })?;
                                drop(mgr);

                                return execute_and_respond(
                                    ext_arc, &ext_id, &operation, &auth.plugin_id, body.input, &op_def, &bridge, &state,
                                ).await;
                            }
                            super::approval::ApprovalDecision::ApproveOnce => {
                                // One-time approval — don't persist, just continue
                                let mgr = state.read().await;
                                let ext_arc = mgr.extensions.get_arc(&ext_id).ok_or_else(|| {
                                    error_response(StatusCode::INTERNAL_SERVER_ERROR, "Extension disappeared", None)
                                })?;
                                drop(mgr);

                                return execute_and_respond(
                                    ext_arc, &ext_id, &operation, &auth.plugin_id, body.input, &op_def, &bridge, &state,
                                ).await;
                            }
                            super::approval::ApprovalDecision::Deny => {
                                return Err(error_response(
                                    StatusCode::FORBIDDEN,
                                    "Scope approval denied",
                                    Some(format!(
                                        "User denied access to scope '{}' = '{}'",
                                        scope_key, scope_value
                                    )),
                                ));
                            }
                        }
                    }
                }
            }
        }
    }

    // 6. LAYER 3 — RISK: Clone Arc, drop lock, execute lock-free
    let ext_arc = mgr.extensions.get_arc(&ext_id).ok_or_else(|| {
        error_response(StatusCode::INTERNAL_SERVER_ERROR, "Extension disappeared", None)
    })?;
    drop(mgr);

    execute_and_respond(
        ext_arc, &ext_id, &operation, &auth.plugin_id, body.input, &op_def, &bridge, &state,
    ).await
}

/// Execute the extension operation, handling high-risk runtime approval.
/// Takes an Arc so the read lock can be dropped before execution.
#[allow(clippy::too_many_arguments)]
async fn execute_and_respond(
    ext: Arc<dyn crate::extensions::Extension>,
    ext_id: &str,
    operation: &str,
    plugin_id: &str,
    input: Value,
    op_def: &crate::extensions::OperationDef,
    bridge: &Arc<ApprovalBridge>,
    state: &AppState,
) -> Result<Json<CallExtensionResponse>, (StatusCode, Json<ExtensionErrorResponse>)> {
    // High-risk operations need per-invocation approval
    if op_def.risk_level == RiskLevel::High {
        let mut context = std::collections::HashMap::new();
        context.insert("extension".to_string(), ext_id.to_string());
        context.insert("extension_display_name".to_string(), ext.display_name().to_string());
        context.insert("operation".to_string(), operation.to_string());
        context.insert("operation_description".to_string(), op_def.description.clone());
        context.insert("risk_level".to_string(), "high".to_string());
        if let Value::Object(map) = &input {
            for (k, v) in map {
                context.insert(format!("input.{}", k), v.to_string());
            }
        }

        let plugin_name = {
            let mgr = state.read().await;
            mgr.storage
                .get(plugin_id)
                .map(|p| p.manifest.name.clone())
                .unwrap_or_else(|| plugin_id.to_string())
        };

        let request = super::approval::ApprovalRequest {
            id: uuid::Uuid::new_v4().to_string(),
            plugin_id: plugin_id.to_string(),
            plugin_name,
            category: format!("extension:{}", ext_id),
            permission: crate::extensions::registry::ExtensionRegistry::permission_string(ext_id, operation),
            context,
        };

        match bridge.request_approval(request).await {
            super::approval::ApprovalDecision::Approve
            | super::approval::ApprovalDecision::ApproveOnce => {
                // We already have the Arc — execute directly, no lock needed
                let result = ext.execute(operation, input).await.map_err(|e| {
                    log::error!(
                        "Extension error: ext={} op={} plugin={} error={}",
                        ext_id, operation, plugin_id, e
                    );
                    error_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Extension execution failed",
                        Some(e.to_string()),
                    )
                })?;

                log::info!(
                    "AUDIT plugin={} extension={} operation={} status=200",
                    plugin_id, ext_id, operation
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

    // Non-high-risk: execute directly (lock already dropped by caller)
    let result = ext.execute(operation, input).await.map_err(|e| {
        log::error!(
            "Extension error: ext={} op={} plugin={} error={}",
            ext_id, operation, plugin_id, e
        );
        error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Extension execution failed",
            Some(e.to_string()),
        )
    })?;

    log::info!(
        "AUDIT plugin={} extension={} operation={} status=200",
        plugin_id, ext_id, operation
    );

    Ok(Json(CallExtensionResponse {
        success: result.success,
        data: result.data,
        message: result.message,
    }))
}
