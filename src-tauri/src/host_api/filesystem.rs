use axum::{extract::Query, extract::State, http::StatusCode, Extension, Json};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::BufRead;
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

#[derive(Deserialize, IntoParams)]
pub struct GlobQuery {
    /// Glob pattern (e.g. "**/*.ts", "src/**/*.rs")
    pub pattern: String,
    /// Base directory to search from (must be absolute)
    pub path: String,
}

#[derive(Serialize, ToSchema)]
pub struct GlobResult {
    pub pattern: String,
    pub base_path: String,
    pub matches: Vec<String>,
}

#[derive(Deserialize, IntoParams)]
pub struct GrepQuery {
    /// Regex pattern to search for
    pub pattern: String,
    /// File or directory to search in (must be absolute)
    pub path: String,
    /// Optional glob filter for file names (e.g. "*.ts")
    #[serde(default)]
    pub include: Option<String>,
    /// Number of context lines around matches (default: 0)
    #[serde(default)]
    pub context_lines: Option<usize>,
    /// Maximum number of matching files to return (default: 50)
    #[serde(default)]
    pub max_results: Option<usize>,
}

#[derive(Serialize, ToSchema)]
pub struct GrepResult {
    pub pattern: String,
    pub search_path: String,
    pub matches: Vec<GrepFileMatch>,
}

#[derive(Serialize, ToSchema)]
pub struct GrepFileMatch {
    pub path: String,
    pub lines: Vec<GrepLine>,
}

#[derive(Serialize, ToSchema)]
pub struct GrepLine {
    pub line_number: usize,
    pub content: String,
    /// Whether this line is a context line (vs a direct match)
    pub is_context: bool,
}

#[derive(Deserialize, ToSchema)]
pub struct EditRequest {
    pub path: String,
    pub old_string: String,
    pub new_string: String,
    #[serde(default)]
    pub replace_all: bool,
}

/// Normalize a path by resolving `.` and `..` components without requiring
/// the path to exist on disk. Used for write targets that don't exist yet.
pub fn normalize_path(path: &Path) -> PathBuf {
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

// ---------------------------------------------------------------------------
// Glob
// ---------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/api/v1/fs/glob",
    tag = "filesystem",
    security(("bearer_auth" = [])),
    params(GlobQuery),
    responses(
        (status = 200, description = "Matching files", body = GlobResult),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    )
)]
pub async fn glob_files(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedPlugin>,
    Extension(bridge): Extension<Arc<ApprovalBridge>>,
    Query(query): Query<GlobQuery>,
) -> Result<Json<GlobResult>, StatusCode> {
    let base = {
        let mgr = state.read().await;
        validate_read_safety(&mgr.data_dir, &query.path)?
    };

    let needs_approval = {
        let mgr = state.read().await;
        check_path_access(
            &*mgr.permissions,
            &auth.plugin_id,
            &Permission::FilesystemRead,
            &base,
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
            &base,
        )
        .await;

        match decision {
            ApprovalDecision::Approve | ApprovalDecision::ApproveOnce => {}
            ApprovalDecision::Deny => return Err(StatusCode::FORBIDDEN),
        }
    }

    if !base.is_dir() {
        return Err(StatusCode::FORBIDDEN);
    }

    // Build the full glob pattern: base_path + pattern
    let full_pattern = base.join(&query.pattern).to_string_lossy().to_string();

    let mut matches = Vec::new();
    for entry in glob::glob(&full_pattern).map_err(|_| StatusCode::BAD_REQUEST)? {
        if let Ok(path) = entry {
            matches.push(path.to_string_lossy().to_string());
        }
        // Cap results to prevent memory issues on huge repos
        if matches.len() >= 1000 {
            break;
        }
    }

    Ok(Json(GlobResult {
        pattern: query.pattern,
        base_path: base.to_string_lossy().to_string(),
        matches,
    }))
}

// ---------------------------------------------------------------------------
// Grep
// ---------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/api/v1/fs/grep",
    tag = "filesystem",
    security(("bearer_auth" = [])),
    params(GrepQuery),
    responses(
        (status = 200, description = "Search results", body = GrepResult),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    )
)]
pub async fn grep_files(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedPlugin>,
    Extension(bridge): Extension<Arc<ApprovalBridge>>,
    Query(query): Query<GrepQuery>,
) -> Result<Json<GrepResult>, StatusCode> {
    let search_path = {
        let mgr = state.read().await;
        validate_read_safety(&mgr.data_dir, &query.path)?
    };

    let needs_approval = {
        let mgr = state.read().await;
        check_path_access(
            &*mgr.permissions,
            &auth.plugin_id,
            &Permission::FilesystemRead,
            &search_path,
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
            &search_path,
        )
        .await;

        match decision {
            ApprovalDecision::Approve | ApprovalDecision::ApproveOnce => {}
            ApprovalDecision::Deny => return Err(StatusCode::FORBIDDEN),
        }
    }

    let re = regex::Regex::new(&query.pattern).map_err(|_| StatusCode::BAD_REQUEST)?;
    let include_glob = query
        .include
        .as_ref()
        .and_then(|g| glob::Pattern::new(g).ok());
    let context_lines = query.context_lines.unwrap_or(0);
    let max_results = query.max_results.unwrap_or(50);

    let mut file_matches: Vec<GrepFileMatch> = Vec::new();

    if search_path.is_file() {
        // Search a single file
        if let Some(file_match) = grep_single_file(&search_path, &re, context_lines) {
            file_matches.push(file_match);
        }
    } else if search_path.is_dir() {
        // Walk directory
        for entry in walkdir::WalkDir::new(&search_path)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| {
                // Skip hidden directories and common noise
                let name = e.file_name().to_string_lossy();
                !name.starts_with('.')
                    && name != "node_modules"
                    && name != "target"
                    && name != "__pycache__"
                    && name != "dist"
            })
        {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };

            if !entry.file_type().is_file() {
                continue;
            }

            // Apply include filter
            if let Some(ref pattern) = include_glob {
                let file_name = entry.file_name().to_string_lossy();
                if !pattern.matches(&file_name) {
                    continue;
                }
            }

            // Skip files larger than 5 MB
            if entry.metadata().map(|m| m.len()).unwrap_or(0) > MAX_READ_BYTES {
                continue;
            }

            if let Some(file_match) = grep_single_file(entry.path(), &re, context_lines) {
                file_matches.push(file_match);
                if file_matches.len() >= max_results {
                    break;
                }
            }
        }
    } else {
        return Err(StatusCode::FORBIDDEN);
    }

    Ok(Json(GrepResult {
        pattern: query.pattern,
        search_path: search_path.to_string_lossy().to_string(),
        matches: file_matches,
    }))
}

/// Search a single file for regex matches, returning matching lines with context.
pub fn grep_single_file(
    path: &Path,
    re: &regex::Regex,
    context_lines: usize,
) -> Option<GrepFileMatch> {
    let file = std::fs::File::open(path).ok()?;
    let reader = std::io::BufReader::new(file);

    let all_lines: Vec<String> = reader.lines().map_while(Result::ok).collect();
    let total = all_lines.len();

    // Find matching line numbers
    let match_indices: Vec<usize> = all_lines
        .iter()
        .enumerate()
        .filter(|(_, line)| re.is_match(line))
        .map(|(i, _)| i)
        .collect();

    if match_indices.is_empty() {
        return None;
    }

    // Build output with context
    let mut result_lines: Vec<GrepLine> = Vec::new();
    let mut included: std::collections::HashSet<usize> = std::collections::HashSet::new();

    for &idx in &match_indices {
        let start = idx.saturating_sub(context_lines);
        let end = (idx + context_lines + 1).min(total);
        for (i, line) in all_lines.iter().enumerate().take(end).skip(start) {
            if included.insert(i) {
                result_lines.push(GrepLine {
                    line_number: i + 1,
                    content: line.clone(),
                    is_context: !match_indices.contains(&i),
                });
            }
        }
    }

    result_lines.sort_by_key(|l| l.line_number);

    Some(GrepFileMatch {
        path: path.to_string_lossy().to_string(),
        lines: result_lines,
    })
}

// ---------------------------------------------------------------------------
// Edit (atomic find-and-replace)
// ---------------------------------------------------------------------------

#[utoipa::path(
    post,
    path = "/api/v1/fs/edit",
    tag = "filesystem",
    security(("bearer_auth" = [])),
    request_body = EditRequest,
    responses(
        (status = 200, description = "File edited"),
        (status = 400, description = "old_string not found or not unique"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    )
)]
pub async fn edit_file(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedPlugin>,
    Extension(bridge): Extension<Arc<ApprovalBridge>>,
    Json(req): Json<EditRequest>,
) -> Result<StatusCode, StatusCode> {
    // Validate write safety for the target path
    let validated = {
        let mgr = state.read().await;
        validate_write_safety(&mgr.data_dir, &req.path)?
    };

    // The file must already exist for edit
    let canonical = validated.canonicalize().map_err(|_| StatusCode::FORBIDDEN)?;

    // Check write permission
    let needs_approval = {
        let mgr = state.read().await;
        check_path_access(
            &*mgr.permissions,
            &auth.plugin_id,
            &Permission::FilesystemWrite,
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
            &Permission::FilesystemWrite,
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

    if req.old_string == req.new_string {
        return Err(StatusCode::BAD_REQUEST);
    }

    let new_content = if req.replace_all {
        let replaced = content.replace(&req.old_string, &req.new_string);
        if replaced == content {
            return Err(StatusCode::BAD_REQUEST); // old_string not found
        }
        replaced
    } else {
        // Ensure old_string is unique
        let count = content.matches(&req.old_string).count();
        if count == 0 {
            return Err(StatusCode::BAD_REQUEST); // not found
        }
        if count > 1 {
            return Err(StatusCode::BAD_REQUEST); // not unique
        }
        content.replacen(&req.old_string, &req.new_string, 1)
    };

    std::fs::write(&canonical, &new_content).map_err(|_| StatusCode::FORBIDDEN)?;

    Ok(StatusCode::OK)
}
