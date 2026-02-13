use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::Emitter;
use tokio::sync::oneshot;

/// Decision the user makes in the runtime approval dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalDecision {
    Approve,
    ApproveOnce,
    Deny,
}

/// Payload emitted to the frontend as a Tauri event.
///
/// Generic across approval categories — `category` tells the dialog what
/// kind of resource is being requested, and `context` carries category-specific
/// details (e.g. `path`, `parent_dir` for filesystem; `url`, `host` for network).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRequest {
    pub id: String,
    pub plugin_id: String,
    pub plugin_name: String,
    pub category: String,
    pub permission: String,
    pub context: HashMap<String, String>,
}

/// Bridge between Axum HTTP handlers and the Tauri frontend.
///
/// An HTTP handler creates a oneshot channel, emits an event to the frontend,
/// and awaits the user's response (with a timeout). The frontend calls a Tauri
/// command that looks up the pending channel by request ID and sends the decision.
pub struct ApprovalBridge {
    pending: Mutex<HashMap<String, oneshot::Sender<ApprovalDecision>>>,
    app_handle: tauri::AppHandle,
}

impl ApprovalBridge {
    pub fn new(app_handle: tauri::AppHandle) -> Self {
        Self {
            pending: Mutex::new(HashMap::new()),
            app_handle,
        }
    }

    /// Emit an approval request to the frontend and wait for the user's decision.
    ///
    /// Returns `Deny` on timeout (60s) or if the receiver is dropped.
    pub async fn request_approval(&self, request: ApprovalRequest) -> ApprovalDecision {
        let (tx, rx) = oneshot::channel();
        let request_id = request.id.clone();

        {
            let mut pending = self.pending.lock().expect("approval lock poisoned");
            pending.insert(request_id.clone(), tx);
        }

        // Emit event to frontend — if this fails the channel stays pending
        // and will time out, which is acceptable.
        let _ = self
            .app_handle
            .emit("nexus://runtime-approval", &request);

        // Await decision with 60s timeout. On timeout or channel drop, deny.
        let decision = tokio::time::timeout(std::time::Duration::from_secs(60), rx).await;

        // Clean up if still pending (timeout or cancellation)
        {
            let mut pending = self.pending.lock().expect("approval lock poisoned");
            pending.remove(&request_id);
        }

        match decision {
            Ok(Ok(d)) => d,
            _ => ApprovalDecision::Deny,
        }
    }

    /// Called by the Tauri command when the user clicks a button in the dialog.
    ///
    /// Returns `true` if the request was found and the decision was delivered.
    pub fn respond(&self, request_id: &str, decision: ApprovalDecision) -> bool {
        let tx = {
            let mut pending = self.pending.lock().expect("approval lock poisoned");
            pending.remove(request_id)
        };

        match tx {
            Some(sender) => sender.send(decision).is_ok(),
            None => false,
        }
    }
}
