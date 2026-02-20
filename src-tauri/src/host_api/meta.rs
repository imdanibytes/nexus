use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

use crate::extensions::Capability;
use crate::permissions::{Permission, PermissionState};
use crate::AppState;

use super::approval::ApprovalBridge;
use super::middleware::AuthenticatedPlugin;

// ── Response types ──────────────────────────────────────────────

/// Permission entry in the self-introspection response.
#[derive(Serialize, ToSchema)]
pub struct MetaPermission {
    pub permission: String,
    pub state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scopes: Option<Vec<String>>,
}

/// Response for GET /v1/meta/self
#[derive(Serialize, ToSchema)]
pub struct MetaSelf {
    pub plugin_id: String,
    pub name: String,
    pub version: String,
    pub status: String,
    pub permissions: Vec<MetaPermission>,
}

/// Response for GET /v1/meta/stats
#[derive(Serialize, ToSchema)]
pub struct MetaStats {
    pub container_id: String,
    #[serde(flatten)]
    pub stats: Value,
}

/// A credential scope entry.
#[derive(Serialize, ToSchema)]
pub struct CredentialScope {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// A credential provider in the list response.
#[derive(Serialize, ToSchema)]
pub struct CredentialProvider {
    pub id: String,
    pub name: String,
    pub scopes: Vec<CredentialScope>,
}

/// Response for GET /v1/meta/credentials
#[derive(Serialize, ToSchema)]
pub struct CredentialProviderList {
    pub providers: Vec<CredentialProvider>,
}

/// Request body for POST /v1/meta/credentials/{ext_id}
#[derive(Deserialize, ToSchema)]
pub struct CredentialRequest {
    #[serde(default = "default_scope")]
    pub scope: String,
}

fn default_scope() -> String {
    "default".to_string()
}

/// Response for POST /v1/meta/credentials/{ext_id}
#[derive(Serialize, ToSchema)]
pub struct CredentialResponse {
    pub provider: String,
    pub scope: String,
    pub data: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
}

/// Error response body
#[derive(Serialize, ToSchema)]
pub struct MetaErrorResponse {
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

fn error_response(
    status: StatusCode,
    error: &str,
    details: Option<String>,
) -> (StatusCode, Json<MetaErrorResponse>) {
    (
        status,
        Json(MetaErrorResponse {
            error: error.to_string(),
            details,
        }),
    )
}

// ── Helpers ─────────────────────────────────────────────────────

/// Build the credential permission string for an extension.
fn credential_permission_string(ext_id: &str) -> String {
    format!("credential:{}", ext_id)
}

/// Check if an extension has the `credential_provider` capability.
fn is_credential_provider(ext: &dyn crate::extensions::Extension) -> bool {
    ext.capabilities()
        .iter()
        .any(|c| matches!(c, Capability::CredentialProvider))
}

// ── Handlers ────────────────────────────────────────────────────

/// Plugin self-introspection.
///
/// Returns the calling plugin's identity, version, status, and permissions
/// with their current states and approved scopes.
#[utoipa::path(
    get,
    path = "/api/v1/meta/self",
    tag = "meta",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Plugin identity and permissions", body = MetaSelf),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn meta_self(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedPlugin>,
) -> Result<Json<MetaSelf>, (StatusCode, Json<MetaErrorResponse>)> {
    let mgr = state.read().await;

    let plugin = mgr.storage.get(&auth.plugin_id).ok_or_else(|| {
        error_response(
            StatusCode::NOT_FOUND,
            "Plugin not found",
            Some(format!("No plugin with ID '{}'", auth.plugin_id)),
        )
    })?;

    let grants = mgr.permissions.get_grants(&auth.plugin_id);
    let permissions: Vec<MetaPermission> = grants
        .into_iter()
        .map(|g| MetaPermission {
            permission: g.permission.as_str().to_string(),
            state: match g.state {
                PermissionState::Active => "active".to_string(),
                PermissionState::Revoked => "revoked".to_string(),
                PermissionState::Deferred => "deferred".to_string(),
            },
            scopes: g.approved_scopes,
        })
        .collect();

    Ok(Json(MetaSelf {
        plugin_id: auth.plugin_id.clone(),
        name: plugin.manifest.name.clone(),
        version: plugin.manifest.version.clone(),
        status: format!("{:?}", plugin.status).to_lowercase(),
        permissions,
    }))
}

/// Plugin container stats.
///
/// Returns CPU, memory, and network statistics for the calling plugin's container.
#[utoipa::path(
    get,
    path = "/api/v1/meta/stats",
    tag = "meta",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Container resource statistics", body = MetaStats),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Container not found"),
    )
)]
pub async fn meta_stats(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedPlugin>,
) -> Result<Json<MetaStats>, (StatusCode, Json<MetaErrorResponse>)> {
    let mgr = state.read().await;

    let plugin = mgr.storage.get(&auth.plugin_id).ok_or_else(|| {
        error_response(
            StatusCode::NOT_FOUND,
            "Plugin not found",
            Some(format!("No plugin with ID '{}'", auth.plugin_id)),
        )
    })?;

    let container_id = plugin.container_id.clone().ok_or_else(|| {
        error_response(
            StatusCode::NOT_FOUND,
            "No container",
            Some("Plugin has no running container".to_string()),
        )
    })?;

    let runtime = mgr.runtime.clone();
    drop(mgr);

    let stats = runtime
        .container_stats_raw(&container_id)
        .await
        .map_err(|e| {
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Stats unavailable",
                Some(e.to_string()),
            )
        })?;

    Ok(Json(MetaStats {
        container_id,
        stats,
    }))
}

/// List available credential providers.
///
/// Returns all enabled extensions with the `credential_provider` capability
/// that the calling plugin has a `credential:{ext_id}` permission for
/// (any state including deferred).
#[utoipa::path(
    get,
    path = "/api/v1/meta/credentials",
    tag = "meta",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Available credential providers", body = CredentialProviderList),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn meta_credentials_list(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedPlugin>,
) -> Json<CredentialProviderList> {
    let mgr = state.read().await;

    let all_extensions = mgr.extensions.list();
    let mut providers = Vec::new();

    for ext_info in &all_extensions {
        // Must be a credential provider
        if !ext_info
            .capabilities
            .iter()
            .any(|c| matches!(c, Capability::CredentialProvider))
        {
            continue;
        }

        // Plugin must have a credential permission for this extension (any state)
        let perm_string = credential_permission_string(&ext_info.id);
        let perm = Permission::Credential(perm_string);
        if mgr.permissions.get_state(&auth.plugin_id, &perm).is_none() {
            continue;
        }

        // Call list_scopes to get available scopes
        let scopes = if let Some(ext) = mgr.extensions.get_arc(&ext_info.id) {
            match ext.execute("list_scopes", Value::Object(Default::default())).await {
                Ok(result) if result.success => {
                    result
                        .data
                        .get("scopes")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|s| {
                                    Some(CredentialScope {
                                        id: s.get("id")?.as_str()?.to_string(),
                                        label: s.get("label").and_then(|v| v.as_str()).map(String::from),
                                        description: s
                                            .get("description")
                                            .and_then(|v| v.as_str())
                                            .map(String::from),
                                    })
                                })
                                .collect()
                        })
                        .unwrap_or_default()
                }
                _ => Vec::new(),
            }
        } else {
            Vec::new()
        };

        providers.push(CredentialProvider {
            id: ext_info.id.clone(),
            name: ext_info.display_name.clone(),
            scopes,
        });
    }

    providers.sort_by(|a, b| a.id.cmp(&b.id));
    Json(CredentialProviderList { providers })
}

/// Resolve credentials from a provider.
///
/// Permission model mirrors extension operations:
/// 1. Check `credential:{ext_id}` permission (Active/Deferred/Revoked)
/// 2. Check scope approval for the requested scope
/// 3. Call the extension's `resolve` operation
#[utoipa::path(
    post,
    path = "/api/v1/meta/credentials/{ext_id}",
    tag = "meta",
    security(("bearer_auth" = [])),
    params(
        ("ext_id" = String, Path, description = "Credential provider extension ID"),
    ),
    request_body = CredentialRequest,
    responses(
        (status = 200, description = "Resolved credentials", body = CredentialResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden — missing permission or scope denied"),
    )
)]
pub async fn meta_credentials_resolve(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedPlugin>,
    Extension(bridge): Extension<Arc<ApprovalBridge>>,
    Path(ext_id): Path<String>,
    Json(body): Json<CredentialRequest>,
) -> Result<Json<CredentialResponse>, (StatusCode, Json<MetaErrorResponse>)> {
    let mgr = state.read().await;

    // 1. Look up extension and verify it's a credential provider
    let ext = mgr.extensions.get(&ext_id).ok_or_else(|| {
        error_response(
            StatusCode::FORBIDDEN,
            "Forbidden",
            Some(format!("No credential provider with ID '{}'", ext_id)),
        )
    })?;

    if !is_credential_provider(ext) {
        return Err(error_response(
            StatusCode::FORBIDDEN,
            "Forbidden",
            Some(format!("Extension '{}' is not a credential provider", ext_id)),
        ));
    }

    // 2. PERMISSION CHECK: credential:{ext_id}
    let perm_string = credential_permission_string(&ext_id);
    let required_perm = Permission::Credential(perm_string.clone());

    match mgr.permissions.get_state(&auth.plugin_id, &required_perm) {
        Some(PermissionState::Active) => {
            // Proceed to scope check below
        }
        Some(PermissionState::Deferred) => {
            // JIT approval: user skipped this at install, ask now
            let plugin_name = mgr
                .storage
                .get(&auth.plugin_id)
                .map(|p| p.manifest.name.clone())
                .unwrap_or_else(|| auth.plugin_id.clone());

            let ext_display = ext.display_name().to_string();

            let mut context = std::collections::HashMap::new();
            context.insert("extension".to_string(), ext_id.clone());
            context.insert("extension_display_name".to_string(), ext_display);
            context.insert("permission".to_string(), perm_string.clone());
            context.insert("scope".to_string(), body.scope.clone());

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
                    let mgr = state.read().await;
                    let _ = mgr.permissions.activate(&auth.plugin_id, &required_perm);
                    drop(mgr);

                    // Continue to scope + resolve below
                    return resolve_credential(
                        &state, &bridge, &auth.plugin_id, &ext_id, &body.scope, &required_perm,
                    )
                    .await;
                }
                super::approval::ApprovalDecision::ApproveOnce => {
                    // One-time: don't persist, state stays Deferred
                    return resolve_credential(
                        &state, &bridge, &auth.plugin_id, &ext_id, &body.scope, &required_perm,
                    )
                    .await;
                }
                super::approval::ApprovalDecision::Deny => {
                    let mgr = state.read().await;
                    let _ = mgr.permissions.revoke(&auth.plugin_id, &required_perm);
                    return Err(error_response(
                        StatusCode::FORBIDDEN,
                        "Permission denied",
                        Some(format!(
                            "User denied deferred permission '{}'",
                            perm_string
                        )),
                    ));
                }
            }
        }
        Some(PermissionState::Revoked) | None => {
            log::warn!(
                "AUDIT DENIED plugin={} credential_provider={} scope={} reason=missing_permission perm={}",
                auth.plugin_id, ext_id, body.scope, perm_string,
            );
            return Err(error_response(
                StatusCode::FORBIDDEN,
                "Permission denied",
                Some(format!(
                    "Plugin '{}' lacks permission '{}'",
                    auth.plugin_id, perm_string
                )),
            ));
        }
    }

    // Permission is Active — proceed to scope check + resolve
    drop(mgr);

    resolve_credential(
        &state, &bridge, &auth.plugin_id, &ext_id, &body.scope, &required_perm,
    )
    .await
}

/// Scope check + extension execution for credential resolution.
async fn resolve_credential(
    state: &AppState,
    bridge: &Arc<ApprovalBridge>,
    plugin_id: &str,
    ext_id: &str,
    scope: &str,
    perm: &Permission,
) -> Result<Json<CredentialResponse>, (StatusCode, Json<MetaErrorResponse>)> {
    let mgr = state.read().await;

    // SCOPE CHECK
    let approved_scopes = mgr.permissions.get_approved_scopes(plugin_id, perm);
    match approved_scopes {
        None => {
            // Unrestricted — no scope checking
        }
        Some(ref scopes) => {
            if !scopes.iter().any(|s| s == scope) {
                // Scope not approved — request runtime approval
                let plugin_name = mgr
                    .storage
                    .get(plugin_id)
                    .map(|p| p.manifest.name.clone())
                    .unwrap_or_else(|| plugin_id.to_string());

                let ext = mgr.extensions.get(ext_id).ok_or_else(|| {
                    error_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Extension disappeared",
                        None,
                    )
                })?;

                let mut context = std::collections::HashMap::new();
                context.insert("extension".to_string(), ext_id.to_string());
                context.insert(
                    "extension_display_name".to_string(),
                    ext.display_name().to_string(),
                );
                context.insert("scope_key".to_string(), "scope".to_string());
                context.insert("scope_value".to_string(), scope.to_string());
                context.insert(
                    "scope_description".to_string(),
                    "credential scope".to_string(),
                );

                let request = super::approval::ApprovalRequest {
                    id: uuid::Uuid::new_v4().to_string(),
                    plugin_id: plugin_id.to_string(),
                    plugin_name,
                    category: format!("credential_scope:{}", ext_id),
                    permission: perm.as_str().to_string(),
                    context,
                };

                // Drop read lock before awaiting approval
                drop(mgr);

                match bridge.request_approval(request).await {
                    super::approval::ApprovalDecision::Approve => {
                        let mgr = state.read().await;
                        let _ = mgr
                            .permissions
                            .add_approved_scope(plugin_id, perm, scope.to_string());
                        drop(mgr);
                    }
                    super::approval::ApprovalDecision::ApproveOnce => {
                        // Don't persist, just continue
                    }
                    super::approval::ApprovalDecision::Deny => {
                        return Err(error_response(
                            StatusCode::FORBIDDEN,
                            "Scope approval denied",
                            Some(format!(
                                "User denied access to credential scope '{}'",
                                scope
                            )),
                        ));
                    }
                }

                // Re-acquire lock and execute
                let mgr = state.read().await;
                let ext_arc = mgr.extensions.get_arc(ext_id).ok_or_else(|| {
                    error_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Extension disappeared",
                        None,
                    )
                })?;
                drop(mgr);

                return execute_resolve(ext_arc, ext_id, plugin_id, scope).await;
            }
        }
    }

    // Scope approved (or unrestricted) — execute
    let ext_arc = mgr.extensions.get_arc(ext_id).ok_or_else(|| {
        error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Extension disappeared",
            None,
        )
    })?;
    drop(mgr);

    execute_resolve(ext_arc, ext_id, plugin_id, scope).await
}

/// Execute the credential provider's `resolve` operation and return the response.
async fn execute_resolve(
    ext: Arc<dyn crate::extensions::Extension>,
    ext_id: &str,
    plugin_id: &str,
    scope: &str,
) -> Result<Json<CredentialResponse>, (StatusCode, Json<MetaErrorResponse>)> {
    let input = serde_json::json!({ "scope": scope });

    let result = ext.execute("resolve", input).await.map_err(|e| {
        log::error!(
            "Credential resolve error: ext={} plugin={} scope={} error={}",
            ext_id,
            plugin_id,
            scope,
            e
        );
        error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Credential resolution failed",
            Some(e.to_string()),
        )
    })?;

    if !result.success {
        log::warn!(
            "AUDIT CREDENTIAL_FAILED plugin={} provider={} scope={} message={:?}",
            plugin_id,
            ext_id,
            scope,
            result.message,
        );
        return Err(error_response(
            StatusCode::BAD_GATEWAY,
            "Credential provider error",
            result.message,
        ));
    }

    log::info!(
        "AUDIT CREDENTIAL plugin={} provider={} scope={} status=200",
        plugin_id,
        ext_id,
        scope,
    );

    // Extract fields from the OperationResult data
    let expires_at = result
        .data
        .get("expires_at")
        .and_then(|v| v.as_str())
        .map(String::from);

    // The credential payload is the whole data object (provider-defined shape)
    Ok(Json(CredentialResponse {
        provider: ext_id.to_string(),
        scope: scope.to_string(),
        data: result.data,
        expires_at,
    }))
}
