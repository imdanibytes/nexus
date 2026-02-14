use std::time::Duration;

use serde::Serialize;

use super::{ExtensionError, OperationResult};
use crate::AppState;

/// Timeout for IPC calls between extensions.
const IPC_TIMEOUT: Duration = Duration::from_secs(10);

/// Trait for routing IPC calls between extensions.
/// Implemented by AppIpcRouter which holds the application state.
pub trait IpcRouter: Send + Sync + 'static {
    /// Call an operation on another extension.
    /// `caller_id` is the extension making the call (for dependency enforcement).
    fn call(
        &self,
        caller_id: &str,
        target_id: &str,
        operation: &str,
        input: serde_json::Value,
    ) -> Result<OperationResult, ExtensionError>;

    /// List all registered extensions (id + display_name).
    fn list_extensions(&self) -> Vec<IpcExtensionInfo>;
}

/// Minimal extension info returned by list_extensions IPC call.
#[derive(Debug, Clone, Serialize)]
pub struct IpcExtensionInfo {
    pub id: String,
    pub display_name: String,
}

/// IPC router backed by the application state.
pub struct AppIpcRouter {
    state: AppState,
}

impl AppIpcRouter {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

impl IpcRouter for AppIpcRouter {
    fn call(
        &self,
        caller_id: &str,
        target_id: &str,
        operation: &str,
        input: serde_json::Value,
    ) -> Result<OperationResult, ExtensionError> {
        // Reject self-calls (would deadlock — same ProcessHandle mutex)
        if caller_id == target_id {
            return Err(ExtensionError::Other(
                "Extension cannot call itself via IPC (would deadlock)".into(),
            ));
        }

        // block_in_place tells tokio this thread will block. It runs the closure
        // on the SAME thread (no migration), so the caller's Mutex<ProcessHandle>
        // guard on the stack stays valid.
        tokio::task::block_in_place(|| {
            let rt = tokio::runtime::Handle::current();

            // Acquire read lock, get Arc clone, drop lock
            let ext = rt.block_on(async {
                let mgr = self.state.read().await;

                // Dependency enforcement: check caller's manifest
                // Look up caller in the extension loader's storage for the manifest
                if let Some(installed) = mgr.extension_loader.storage.get(caller_id) {
                    if !installed.manifest.extension_dependencies.contains(&target_id.to_string()) {
                        return Err(ExtensionError::Other(format!(
                            "Extension '{}' is not declared as a dependency of '{}'. \
                             Add it to extension_dependencies in the manifest.",
                            target_id, caller_id,
                        )));
                    }
                }

                mgr.extensions.get_arc(target_id).ok_or_else(|| {
                    ExtensionError::Other(format!(
                        "Target extension '{}' not found or not running",
                        target_id,
                    ))
                })
            })?;

            // Execute with timeout — lock is dropped, only holding the Arc
            rt.block_on(async {
                match tokio::time::timeout(IPC_TIMEOUT, ext.execute(operation, input)).await {
                    Ok(result) => result,
                    Err(_) => Err(ExtensionError::Timeout),
                }
            })
        })
    }

    fn list_extensions(&self) -> Vec<IpcExtensionInfo> {
        tokio::task::block_in_place(|| {
            let rt = tokio::runtime::Handle::current();
            rt.block_on(async {
                let mgr = self.state.read().await;
                mgr.extensions
                    .list()
                    .into_iter()
                    .map(|info| IpcExtensionInfo {
                        id: info.id,
                        display_name: info.display_name,
                    })
                    .collect()
            })
        })
    }
}
