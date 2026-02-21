use serde_json::Value;
use tauri::Emitter;

use super::cloud_event::CloudEvent;
use super::routing::RouteAction;
use super::store::EventStore;
use crate::AppState;

/// Executes route actions triggered by event bus routing rules.
///
/// Supports two dispatch modes:
/// - **Durable** (`execute_durable`): persists the event and delivery rows to SQLite.
///   A background retry worker picks them up, executes, and tracks completion.
/// - **Fire-and-forget** (`execute`): spawns tokio tasks directly. No persistence or retry.
///   Kept as a fallback if the store is unavailable.
#[derive(Clone)]
pub struct RouteActionExecutor {
    state: AppState,
    app_handle: tauri::AppHandle,
}

impl RouteActionExecutor {
    pub fn new(state: AppState, app_handle: tauri::AppHandle) -> Self {
        Self { state, app_handle }
    }

    /// Persist the event and create delivery rows for each action.
    /// The retry worker will pick them up and execute them with tracking.
    pub fn execute_durable(
        &self,
        store: &EventStore,
        actions: Vec<RouteAction>,
        event: &CloudEvent,
    ) {
        if actions.is_empty() {
            return;
        }

        if let Err(e) = store.insert_event(event) {
            log::error!("Failed to persist event {}: {}", event.id, e);
            // Fall back to fire-and-forget
            self.execute(actions, event.clone());
            return;
        }

        if let Err(e) = store.insert_deliveries(&event.id, actions.clone()) {
            log::error!(
                "Failed to create deliveries for event {}: {}",
                event.id,
                e
            );
            // Event is persisted but deliveries aren't — fall back
            self.execute(actions, event.clone());
        }
    }

    /// Fire-and-forget dispatch. Each action runs as an independent tokio task.
    /// Failures are logged but not retried.
    pub fn execute(&self, actions: Vec<RouteAction>, event: CloudEvent) {
        for action in actions {
            let state = self.state.clone();
            let app_handle = self.app_handle.clone();
            let event = event.clone();
            tokio::spawn(async move {
                let label = action_label(&action);
                match execute_one(state, &app_handle, action, &event).await {
                    Ok(()) => log::info!("Route action executed: {}", label),
                    Err(e) => log::error!("Route action failed: {} — {}", label, e),
                }
            });
        }
    }

    /// Execute a single action and return the result. Used by the retry worker.
    pub async fn execute_single(
        &self,
        action: RouteAction,
        event: CloudEvent,
    ) -> Result<(), String> {
        execute_one(self.state.clone(), &self.app_handle, action, &event).await
    }
}

fn action_label(action: &RouteAction) -> String {
    match action {
        RouteAction::InvokePluginTool {
            plugin_id,
            tool_name,
            ..
        } => format!("InvokePluginTool({}.{})", plugin_id, tool_name),
        RouteAction::CallExtension {
            extension_id,
            operation,
            ..
        } => format!("CallExtension({}.{})", extension_id, operation),
        RouteAction::EmitFrontend { channel } => format!("EmitFrontend({})", channel),
    }
}

async fn execute_one(
    state: AppState,
    app_handle: &tauri::AppHandle,
    action: RouteAction,
    event: &CloudEvent,
) -> Result<(), String> {
    match action {
        RouteAction::InvokePluginTool {
            plugin_id,
            tool_name,
            args_template,
        } => {
            let args = resolve_args(args_template, event);
            let mgr = state.read().await;
            mgr.mcp_clients
                .call_tool(&plugin_id, &tool_name, args.as_object().cloned())
                .await
                .map(|_| ())
        }
        RouteAction::CallExtension {
            extension_id,
            operation,
            args_template,
        } => {
            let args = resolve_args(args_template, event);
            let ext = {
                let mgr = state.read().await;
                mgr.extensions.get_arc(&extension_id).ok_or_else(|| {
                    format!("Extension '{}' not found or not running", extension_id)
                })?
            };
            ext.execute(&operation, args)
                .await
                .map(|_| ())
                .map_err(|e| e.to_string())
        }
        RouteAction::EmitFrontend { channel } => {
            let data = serde_json::to_value(event).unwrap_or(Value::Null);
            app_handle
                .emit(&channel, data)
                .map_err(|e| format!("Tauri emit failed: {}", e))
        }
    }
}

/// Resolve route action arguments. If a template is provided, use it directly.
/// If no template, forward the event's data payload as the arguments.
fn resolve_args(template: Option<Value>, event: &CloudEvent) -> Value {
    template.unwrap_or_else(|| event.data.clone())
}
