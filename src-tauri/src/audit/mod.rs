//! Security audit subsystem.
//!
//! This module implements a **durable, structured security audit log** for Nexus.
//! Every state-changing operation AND every external access through the MCP gateway
//! is recorded here for compliance, incident investigation, and forensic analysis.
//!
//! **Design principle**: This is a security audit system, not an activity feed.
//! All access from external interfaces (MCP tool calls from AI clients) is logged
//! regardless of whether the tool is read-only or mutating — because in a security
//! context, *reading* your plugin inventory, filesystem, or configuration is just
//! as significant as modifying it.
//!
//! Follows [OWASP Logging Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Logging_Cheat_Sheet.html)
//! and [NIST SP 800-53 AU controls](https://nvlpubs.nist.gov/nistpubs/Legacy/SP/nistspecialpublication800-92.pdf)
//! for the "who, what, where, when, outcome" audit record structure. Severity levels
//! align with OWASP's INFO / WARN / CRITICAL classification.
//!
//! The write path is non-blocking: callers push entries into a bounded mpsc channel
//! via [`writer::AuditWriter`], and a background task batch-inserts into SQLite.
//! Entries are retained for 30 days and automatically pruned.

pub mod store;
pub mod writer;

use serde::{Deserialize, Serialize};

/// The identity that initiated the audited operation.
///
/// Security audit logs must attribute every action to an actor. The granularity
/// here distinguishes between internal system automation, interactive user actions
/// through the Tauri frontend, external AI clients accessing the MCP gateway, and
/// plugin-initiated operations through the Host API.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditActor {
    /// Internal system operations (background tasks, scheduled cleanup, etc.)
    System,
    /// Interactive user actions through the Tauri frontend UI.
    User,
    /// External AI client accessing Nexus through the MCP gateway.
    /// All MCP tool calls — read-only and mutating — are attributed to this actor.
    McpClient,
    /// A plugin performing operations through the Host API.
    Plugin { id: String },
}

impl AuditActor {
    pub fn as_str(&self) -> String {
        match self {
            AuditActor::System => "system".to_string(),
            AuditActor::User => "user".to_string(),
            AuditActor::McpClient => "mcp_client".to_string(),
            AuditActor::Plugin { id } => format!("plugin:{}", id),
        }
    }
}

/// Severity level of the audited operation (OWASP classification).
///
/// Severity is determined by the nature of the operation, not its outcome.
/// A failed `plugin.install` is still `Warn` because the *attempt* is what
/// matters for the audit trail. Severity helps operators quickly triage
/// which entries need attention during incident investigation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditSeverity {
    /// Routine operations — listing, reading, querying, successful starts/stops,
    /// settings changes. The bread-and-butter of normal operation.
    Info,
    /// Operations that change system state — plugin/extension lifecycle, registry
    /// changes, updates. Worth reviewing during incident investigation.
    Warn,
    /// Security-sensitive operations — permission changes, credential management,
    /// host command execution, destructive operations. Always review these.
    Critical,
}

impl AuditSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            AuditSeverity::Info => "info",
            AuditSeverity::Warn => "warn",
            AuditSeverity::Critical => "critical",
        }
    }
}

/// Outcome of the audited operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditResult {
    Success,
    Failure,
}

impl AuditResult {
    pub fn as_str(&self) -> &'static str {
        match self {
            AuditResult::Success => "success",
            AuditResult::Failure => "failure",
        }
    }
}

/// A structured audit entry sent through the writer channel.
///
/// Each entry captures who did what to which target, whether it succeeded, and
/// optional machine-readable details. The timestamp is assigned by SQLite on insert.
///
/// Fields map to OWASP/NIST "who, what, where, when, outcome":
/// - **who**: `actor` (category) + `source_id` (specific identity)
/// - **what**: `action` + `details`
/// - **where**: `subject` (target entity)
/// - **when**: timestamp (DB-generated)
/// - **outcome**: `result` + `severity`
#[derive(Debug, Clone)]
pub struct AuditEntry {
    pub actor: AuditActor,
    /// Specific identity of the source — MCP session/client ID, plugin ID,
    /// extension ID, or None for system/local user operations.
    pub source_id: Option<String>,
    pub severity: AuditSeverity,
    pub action: String,
    pub subject: Option<String>,
    pub result: AuditResult,
    pub details: Option<serde_json::Value>,
}

/// A row returned from audit log queries (includes DB-generated fields).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogRow {
    pub id: i64,
    pub timestamp: String,
    pub actor: String,
    pub source_id: Option<String>,
    pub severity: String,
    pub action: String,
    pub subject: Option<String>,
    pub result: String,
    pub details: Option<serde_json::Value>,
}

/// Query parameters for filtering the audit log.
#[derive(Debug, Default, Deserialize)]
pub struct AuditQuery {
    pub action: Option<String>,
    pub actor: Option<String>,
    pub source_id: Option<String>,
    pub severity: Option<String>,
    pub subject: Option<String>,
    pub result: Option<String>,
    pub since: Option<String>,
    pub until: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}
