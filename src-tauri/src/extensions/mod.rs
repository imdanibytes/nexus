pub mod capability;
pub mod ipc;
pub mod loader;
pub mod manifest;
pub mod process;
pub mod registry;
pub mod signing;
pub mod storage;
pub mod validation;

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub use capability::Capability;
pub use ipc::IpcRouter;

/// Risk level for an extension operation, determining whether runtime approval is needed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

/// Describes a single operation that an extension exposes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationDef {
    /// Machine name, e.g. "refresh_credentials"
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// How dangerous is this operation
    pub risk_level: RiskLevel,
    /// JSON Schema for the input object (must have "type": "object" at root)
    pub input_schema: Value,
    /// Input field name used for scope checking (e.g. "repo_path").
    /// Operations without a scope_key skip scope enforcement.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope_key: Option<String>,
    /// Human-readable label for the scope (shown in approval dialogs).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope_description: Option<String>,
}

/// Successful result from executing an operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationResult {
    /// Whether the operation succeeded
    pub success: bool,
    /// Operation output data
    pub data: Value,
    /// Optional human-readable message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Errors that can occur when executing an extension operation.
#[derive(Debug, thiserror::Error)]
pub enum ExtensionError {
    #[error("Unknown operation: {0}")]
    UnknownOperation(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Command failed (exit code {exit_code}): {stderr}")]
    CommandFailed {
        exit_code: i32,
        stderr: String,
    },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Process not running")]
    ProcessNotRunning,

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("Timeout waiting for extension response")]
    Timeout,

    #[error("Signature verification failed: {0}")]
    SignatureError(String),

    #[error("{0}")]
    Other(String),
}

/// The core extension trait. Each extension implements this trait,
/// is registered in the ExtensionRegistry, and exposed via the Host API.
///
/// Extensions are pure infrastructure — they provide capabilities, not user-facing tools.
/// Only plugins expose MCP tools and UIs. The call chain is:
/// AI → MCP sidecar → Plugin MCP tool → Host API /v1/extensions/{ext}/{op} → Extension → host CLI
#[async_trait]
pub trait Extension: Send + Sync + 'static {
    /// Unique identifier, e.g. "weather" or "file_sync"
    fn id(&self) -> &str;

    /// Human-readable name, e.g. "Weather Service"
    fn display_name(&self) -> &str;

    /// Short description of what this extension provides
    fn description(&self) -> &str;

    /// List all operations this extension supports
    fn operations(&self) -> Vec<OperationDef>;

    /// Declared capabilities (shown to users at install time for informed consent).
    fn capabilities(&self) -> Vec<Capability> {
        Vec::new()
    }

    /// Execute a named operation with the given JSON input.
    /// Input will already be validated against the operation's input_schema before this is called.
    async fn execute(&self, operation: &str, input: Value) -> Result<OperationResult, ExtensionError>;

    /// Inject an IPC router so this extension can call other extensions.
    /// Default no-op — only ProcessExtension overrides this.
    fn set_ipc_router(&self, _router: Arc<dyn IpcRouter>) {}
}
