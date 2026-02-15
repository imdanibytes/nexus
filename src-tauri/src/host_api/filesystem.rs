use axum::{extract::Query, extract::State, http::StatusCode, Extension, Json};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use utoipa::{IntoParams, ToSchema};

use super::approval::{ApprovalBridge, ApprovalDecision, ApprovalRequest};
use super::middleware::AuthenticatedPlugin;
use crate::permissions::Permission;
use crate::AppState;

/// Maximum file size for reads (5 MB). Prevents loading huge files into memory.
const MAX_READ_BYTES: u64 = 5 * 1024 * 1024;

#[derive(Deserialize, IntoParams)]
pub struct PathQuery {
    pub path: String,
}

#[derive(Serialize, ToSchema)]
pub struct FileContent {
    pub path: String,
    pub content: String,
    pub size: u64,
}

#[derive(Serialize, ToSchema)]
pub struct DirEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
}

#[derive(Serialize, ToSchema)]
pub struct DirListing {
    pub path: String,
    pub entries: Vec<DirEntry>,
}

#[derive(Deserialize, ToSchema)]
pub struct WriteRequest {
    pub path: String,
    pub content: String,
}

/// Normalize a path by resolving `.` and `..` components without requiring
/// the path to exist on disk. Used for write targets that don't exist yet.
fn normalize_path(path: &Path) -> PathBuf {
    let mut components = Vec::new();
    for component in path.components() {
        match component {
            Component::ParentDir => {
                components.pop();
            }
            Component::CurDir => {}
            c => components.push(c),
        }
    }
    components.iter().collect()
}

// ---------------------------------------------------------------------------
// Phase 1: Safety validation (hard deny, no recourse)
// ---------------------------------------------------------------------------

/// Validate safety for a read/list path. Canonicalizes (path must exist) and
/// blocks access to the Nexus data directory.
fn validate_read_safety(data_dir: &Path, raw_path: &str) -> Result<PathBuf, StatusCode> {
    let path = PathBuf::from(raw_path);
    let canonical = path.canonicalize().map_err(|_| StatusCode::FORBIDDEN)?;

    if canonical.starts_with(data_dir) {
        return Err(StatusCode::FORBIDDEN);
    }

    Ok(canonical)
}

/// Validate safety for a write path. Normalizes (target may not exist) and
/// blocks access to the Nexus data directory.
fn validate_write_safety(data_dir: &Path, raw_path: &str) -> Result<PathBuf, StatusCode> {
    let path = PathBuf::from(raw_path);

    if !path.is_absolute() {
        return Err(StatusCode::FORBIDDEN);
    }

    let normalized = normalize_path(&path);

    if normalized.starts_with(data_dir) {
        return Err(StatusCode::FORBIDDEN);
    }

    Ok(normalized)
}

// ---------------------------------------------------------------------------
// Phase 2: Access control (may prompt user)
// ---------------------------------------------------------------------------

enum PathAccess {
    Allowed,
    NeedsApproval,
}

/// Check whether a validated path falls within the plugin's approved_paths.
/// Returns `Allowed` when the grant is unrestricted (`None`) or the path is
/// covered by an existing approved directory. Returns `NeedsApproval` otherwise.
fn check_path_access(
    permissions: &dyn crate::permissions::PermissionService,
    plugin_id: &str,
    permission: &Permission,
    validated_path: &Path,
    use_canonicalize: bool,
) -> PathAccess {
    let approved = match permissions.get_approved_paths(plugin_id, permission) {
        Some(paths) => paths,
        None => return PathAccess::Allowed, // unrestricted
    };

    let allowed = approved.iter().any(|allowed_path| {
        if use_canonicalize {
            PathBuf::from(allowed_path)
                .canonicalize()
                .map(|ap| validated_path.starts_with(&ap))
                .unwrap_or(false)
        } else {
            let ap = normalize_path(Path::new(allowed_path));
            validated_path.starts_with(&ap)
        }
    });

    if allowed {
        PathAccess::Allowed
    } else {
        PathAccess::NeedsApproval
    }
}

/// Build and send an approval request for a filesystem path, await the decision.
async fn request_fs_approval(
    bridge: &ApprovalBridge,
    plugin_id: &str,
    plugin_name: &str,
    permission: &Permission,
    validated_path: &Path,
) -> ApprovalDecision {
    let parent_dir = validated_path
        .parent()
        .unwrap_or(validated_path)
        .to_string_lossy()
        .to_string();

    let mut context = HashMap::new();
    context.insert("path".to_string(), validated_path.to_string_lossy().to_string());
    context.insert("parent_dir".to_string(), parent_dir);

    let request = ApprovalRequest {
        id: uuid::Uuid::new_v4().to_string(),
        plugin_id: plugin_id.to_string(),
        plugin_name: plugin_name.to_string(),
        category: "filesystem".to_string(),
        permission: serde_json::to_value(permission)
            .ok()
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_default(),
        context,
    };

    bridge.request_approval(request).await
}

/// Resolve the human-readable plugin name from state.
async fn plugin_display_name(state: &AppState, plugin_id: &str) -> String {
    let mgr = state.read().await;
    mgr.storage
        .get(plugin_id)
        .map(|p| p.manifest.name.clone())
        .unwrap_or_else(|| plugin_id.to_string())
}

#[utoipa::path(
    get,
    path = "/api/v1/fs/read",
    tag = "filesystem",
    security(("bearer_auth" = [])),
    params(PathQuery),
    responses(
        (status = 200, description = "File content", body = FileContent),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    )
)]
pub async fn read_file(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedPlugin>,
    Extension(bridge): Extension<Arc<ApprovalBridge>>,
    Query(query): Query<PathQuery>,
) -> Result<Json<FileContent>, StatusCode> {
    // Phase 1: safety (hard deny)
    let canonical = {
        let mgr = state.read().await;
        validate_read_safety(&mgr.data_dir, &query.path)?
    };

    // Phase 2: access control
    let needs_approval = {
        let mgr = state.read().await;
        check_path_access(
            &*mgr.permissions,
            &auth.plugin_id,
            &Permission::FilesystemRead,
            &canonical,
            true,
        )
    };

    if matches!(needs_approval, PathAccess::NeedsApproval) {
        let name = plugin_display_name(&state, &auth.plugin_id).await;
        let decision = request_fs_approval(
            &bridge,
            &auth.plugin_id,
            &name,
            &Permission::FilesystemRead,
            &canonical,
        )
        .await;

        match decision {
            ApprovalDecision::Approve | ApprovalDecision::ApproveOnce => {}
            ApprovalDecision::Deny => return Err(StatusCode::FORBIDDEN),
        }
    }

    if !canonical.is_file() {
        return Err(StatusCode::FORBIDDEN);
    }

    let metadata = std::fs::metadata(&canonical).map_err(|_| StatusCode::FORBIDDEN)?;

    if metadata.len() > MAX_READ_BYTES {
        return Err(StatusCode::FORBIDDEN);
    }

    let content = std::fs::read_to_string(&canonical).map_err(|_| StatusCode::FORBIDDEN)?;

    Ok(Json(FileContent {
        path: canonical.to_string_lossy().to_string(),
        content,
        size: metadata.len(),
    }))
}

#[utoipa::path(
    get,
    path = "/api/v1/fs/list",
    tag = "filesystem",
    security(("bearer_auth" = [])),
    params(PathQuery),
    responses(
        (status = 200, description = "Directory listing", body = DirListing),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    )
)]
pub async fn list_dir(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedPlugin>,
    Extension(bridge): Extension<Arc<ApprovalBridge>>,
    Query(query): Query<PathQuery>,
) -> Result<Json<DirListing>, StatusCode> {
    let canonical = {
        let mgr = state.read().await;
        validate_read_safety(&mgr.data_dir, &query.path)?
    };

    let needs_approval = {
        let mgr = state.read().await;
        check_path_access(
            &*mgr.permissions,
            &auth.plugin_id,
            &Permission::FilesystemRead,
            &canonical,
            true,
        )
    };

    if matches!(needs_approval, PathAccess::NeedsApproval) {
        let name = plugin_display_name(&state, &auth.plugin_id).await;
        let decision = request_fs_approval(
            &bridge,
            &auth.plugin_id,
            &name,
            &Permission::FilesystemRead,
            &canonical,
        )
        .await;

        match decision {
            ApprovalDecision::Approve | ApprovalDecision::ApproveOnce => {}
            ApprovalDecision::Deny => return Err(StatusCode::FORBIDDEN),
        }
    }

    if !canonical.is_dir() {
        return Err(StatusCode::FORBIDDEN);
    }

    let mut entries = Vec::new();
    let read_dir = std::fs::read_dir(&canonical).map_err(|_| StatusCode::FORBIDDEN)?;

    for entry in read_dir.flatten() {
        let metadata = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };

        entries.push(DirEntry {
            name: entry.file_name().to_string_lossy().to_string(),
            path: entry.path().to_string_lossy().to_string(),
            is_dir: metadata.is_dir(),
            size: metadata.len(),
        });
    }

    Ok(Json(DirListing {
        path: canonical.to_string_lossy().to_string(),
        entries,
    }))
}

#[utoipa::path(
    post,
    path = "/api/v1/fs/write",
    tag = "filesystem",
    security(("bearer_auth" = [])),
    request_body = WriteRequest,
    responses(
        (status = 200, description = "File written"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    )
)]
pub async fn write_file(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedPlugin>,
    Extension(bridge): Extension<Arc<ApprovalBridge>>,
    Json(req): Json<WriteRequest>,
) -> Result<StatusCode, StatusCode> {
    let validated = {
        let mgr = state.read().await;
        validate_write_safety(&mgr.data_dir, &req.path)?
    };

    let needs_approval = {
        let mgr = state.read().await;
        check_path_access(
            &*mgr.permissions,
            &auth.plugin_id,
            &Permission::FilesystemWrite,
            &validated,
            false,
        )
    };

    if matches!(needs_approval, PathAccess::NeedsApproval) {
        let name = plugin_display_name(&state, &auth.plugin_id).await;
        let decision = request_fs_approval(
            &bridge,
            &auth.plugin_id,
            &name,
            &Permission::FilesystemWrite,
            &validated,
        )
        .await;

        match decision {
            ApprovalDecision::Approve | ApprovalDecision::ApproveOnce => {}
            ApprovalDecision::Deny => return Err(StatusCode::FORBIDDEN),
        }
    }

    if let Some(parent) = validated.parent() {
        std::fs::create_dir_all(parent).map_err(|_| StatusCode::FORBIDDEN)?;
    }

    std::fs::write(&validated, &req.content).map_err(|_| StatusCode::FORBIDDEN)?;

    Ok(StatusCode::OK)
}
