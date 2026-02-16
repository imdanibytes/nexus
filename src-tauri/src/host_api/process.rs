use std::collections::HashMap;
use std::sync::Arc;

use axum::http::StatusCode;
use axum::{extract::State, Extension, Json};
use serde::{Deserialize, Serialize};
use sysinfo::System;
use utoipa::ToSchema;

use super::approval::{ApprovalBridge, ApprovalDecision, ApprovalRequest};
use super::middleware::AuthenticatedPlugin;
use crate::permissions::Permission;
use crate::AppState;

/// Maximum output size per stream (stdout/stderr) — 1 MB
const MAX_OUTPUT_BYTES: usize = 1024 * 1024;

/// Default command timeout — 30 seconds
const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// Maximum command timeout — 10 minutes
const MAX_TIMEOUT_SECS: u64 = 600;

#[derive(Serialize, ToSchema)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub cpu_usage: f32,
    pub memory: u64,
}

#[derive(Deserialize, ToSchema)]
pub struct ExecRequest {
    /// The command to execute (e.g. "git", "cargo", "ls")
    pub command: String,
    /// Arguments to pass to the command
    #[serde(default)]
    pub args: Vec<String>,
    /// Working directory (must be absolute). Defaults to home directory.
    #[serde(default)]
    pub working_dir: Option<String>,
    /// Timeout in seconds (default: 30, max: 600)
    #[serde(default)]
    pub timeout_secs: Option<u64>,
}

#[derive(Serialize, ToSchema)]
pub struct ExecResult {
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    /// Whether the command was killed due to timeout
    pub timed_out: bool,
}

#[utoipa::path(
    get,
    path = "/api/v1/process/list",
    tag = "process",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Running processes", body = Vec<ProcessInfo>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Permission denied")
    )
)]
pub async fn list_processes() -> Json<Vec<ProcessInfo>> {
    let mut sys = System::new_all();
    sys.refresh_all();

    let processes: Vec<ProcessInfo> = sys
        .processes()
        .iter()
        .map(|(pid, proc_)| ProcessInfo {
            pid: pid.as_u32(),
            name: proc_.name().to_string_lossy().to_string(),
            cpu_usage: proc_.cpu_usage(),
            memory: proc_.memory(),
        })
        .collect();

    Json(processes)
}

#[utoipa::path(
    post,
    path = "/api/v1/process/exec",
    tag = "process",
    security(("bearer_auth" = [])),
    request_body = ExecRequest,
    responses(
        (status = 200, description = "Command output", body = ExecResult),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    )
)]
pub async fn exec_command(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedPlugin>,
    Extension(bridge): Extension<Arc<ApprovalBridge>>,
    Json(req): Json<ExecRequest>,
) -> Result<Json<ExecResult>, StatusCode> {
    // Validate working directory if provided
    if let Some(ref dir) = req.working_dir {
        let path = std::path::PathBuf::from(dir);
        if !path.is_absolute() {
            return Err(StatusCode::BAD_REQUEST);
        }
        let canonical = path.canonicalize().map_err(|_| StatusCode::BAD_REQUEST)?;
        if !canonical.is_dir() {
            return Err(StatusCode::BAD_REQUEST);
        }
        // Block Nexus data directory
        let data_dir = { state.read().await.data_dir.clone() };
        if canonical.starts_with(&data_dir) {
            return Err(StatusCode::FORBIDDEN);
        }
    }

    // Always require runtime approval for command execution.
    // This is the highest-risk operation — every call gets a dialog.
    let plugin_name = {
        let mgr = state.read().await;
        mgr.storage
            .get(&auth.plugin_id)
            .map(|p| p.manifest.name.clone())
            .unwrap_or_else(|| auth.plugin_id.clone())
    };

    let mut context = HashMap::new();
    context.insert("command".to_string(), req.command.clone());
    context.insert("args".to_string(), req.args.join(" "));
    if let Some(ref dir) = req.working_dir {
        context.insert("working_dir".to_string(), dir.clone());
    }

    let approval_req = ApprovalRequest {
        id: uuid::Uuid::new_v4().to_string(),
        plugin_id: auth.plugin_id.clone(),
        plugin_name: plugin_name.clone(),
        category: "process_exec".to_string(),
        permission: Permission::ProcessExec.as_str().to_string(),
        context,
    };

    match bridge.request_approval(approval_req).await {
        ApprovalDecision::Approve | ApprovalDecision::ApproveOnce => {}
        ApprovalDecision::Deny => return Err(StatusCode::FORBIDDEN),
    }

    // Build the command
    let mut cmd = tokio::process::Command::new(&req.command);
    cmd.args(&req.args);

    if let Some(ref dir) = req.working_dir {
        cmd.current_dir(dir);
    }

    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    // Spawn
    let child = cmd.spawn().map_err(|e| {
        log::warn!("Failed to spawn command '{}': {}", req.command, e);
        StatusCode::BAD_REQUEST
    })?;

    let timeout = std::time::Duration::from_secs(
        req.timeout_secs
            .unwrap_or(DEFAULT_TIMEOUT_SECS)
            .min(MAX_TIMEOUT_SECS),
    );

    // Wait with timeout
    let result = tokio::time::timeout(timeout, child.wait_with_output()).await;

    match result {
        Ok(Ok(output)) => {
            let stdout_len = output.stdout.len().min(MAX_OUTPUT_BYTES);
            let stderr_len = output.stderr.len().min(MAX_OUTPUT_BYTES);
            let stdout = String::from_utf8_lossy(&output.stdout[..stdout_len]).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr[..stderr_len]).to_string();

            Ok(Json(ExecResult {
                exit_code: output.status.code(),
                stdout,
                stderr,
                timed_out: false,
            }))
        }
        Ok(Err(e)) => {
            log::warn!("Command '{}' failed: {}", req.command, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
        Err(_) => {
            // Timeout — process consumed by wait_with_output future,
            // tokio drops it which kills the child on Unix.
            // On Windows, the child may linger — acceptable tradeoff.
            Ok(Json(ExecResult {
                exit_code: None,
                stdout: String::new(),
                stderr: format!("Command timed out after {} seconds", timeout.as_secs()),
                timed_out: true,
            }))
        }
    }
}
