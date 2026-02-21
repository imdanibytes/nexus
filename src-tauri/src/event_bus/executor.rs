use regex::Regex;
use serde_json::Value;
use std::sync::OnceLock;
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

/// Resolve route action arguments. If a template is provided, resolve
/// `{{path}}` expressions against the event. If no template, forward the
/// event's data payload as the arguments.
fn resolve_args(template: Option<Value>, event: &CloudEvent) -> Value {
    match template {
        Some(tmpl) => resolve_template(tmpl, event),
        None => event.data.clone(),
    }
}

// ---------------------------------------------------------------------------
// Template engine
// ---------------------------------------------------------------------------

fn template_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\{\{(\s*[\w.]+\s*)\}\}").unwrap())
}

/// Recursively walk a JSON value, replacing `{{path}}` in strings.
fn resolve_template(value: Value, event: &CloudEvent) -> Value {
    match value {
        Value::String(s) => resolve_string_template(&s, event),
        Value::Object(map) => {
            let resolved: serde_json::Map<String, Value> = map
                .into_iter()
                .map(|(k, v)| (k, resolve_template(v, event)))
                .collect();
            Value::Object(resolved)
        }
        Value::Array(arr) => {
            Value::Array(arr.into_iter().map(|v| resolve_template(v, event)).collect())
        }
        other => other,
    }
}

/// Resolve template expressions in a string.
///
/// If the **entire** string is a single `{{path}}` expression, return the
/// resolved value as-is (preserving its JSON type — could be object, array,
/// number, etc.). Otherwise do string interpolation, converting resolved
/// values to their string representation.
fn resolve_string_template(s: &str, event: &CloudEvent) -> Value {
    let trimmed = s.trim();

    // Fast path: entire string is one `{{path}}` — preserve type.
    if trimmed.starts_with("{{")
        && trimmed.ends_with("}}")
        && trimmed.matches("{{").count() == 1
    {
        let path = trimmed[2..trimmed.len() - 2].trim();
        return resolve_path(path, event);
    }

    // Mixed text + templates — string interpolation.
    let result = template_re().replace_all(s, |caps: &regex::Captures| {
        let path = caps[1].trim();
        match resolve_path(path, event) {
            Value::String(s) => s,
            Value::Null => String::new(),
            other => other.to_string(),
        }
    });

    Value::String(result.into_owned())
}

/// Resolve a dot-separated path against a CloudEvent.
///
/// Supported paths:
/// - `event.id`, `event.type`, `event.source`, `event.subject`, `event.time`
/// - `event.data` — entire data payload
/// - `event.data.foo.bar.baz` — nested field access into data
fn resolve_path(path: &str, event: &CloudEvent) -> Value {
    let path = path.strip_prefix("event.").unwrap_or(path);

    match path {
        "id" => Value::String(event.id.clone()),
        "type" => Value::String(event.event_type.clone()),
        "source" => Value::String(event.source.clone()),
        "subject" => match &event.subject {
            Some(s) => Value::String(s.clone()),
            None => Value::Null,
        },
        "time" => Value::String(event.time.to_rfc3339()),
        "data" => event.data.clone(),
        other if other.starts_with("data.") => {
            let field_path = &other["data.".len()..];
            let mut current = &event.data;
            for segment in field_path.split('.') {
                match current.get(segment) {
                    Some(v) => current = v,
                    None => return Value::Null,
                }
            }
            current.clone()
        }
        _ => Value::Null,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_bus::cloud_event::CloudEvent;
    use serde_json::json;

    fn test_event() -> CloudEvent {
        CloudEvent::builder()
            .source("nexus://extension/webhook-receiver")
            .event_type("com.github.issues.opened")
            .subject("wh_abc123")
            .data(json!({
                "issue": {
                    "title": "Fix the login bug",
                    "body": "Users can't log in on Safari",
                    "html_url": "https://github.com/org/repo/issues/42",
                    "number": 42
                },
                "repository": {
                    "full_name": "org/repo"
                }
            }))
            .build()
            .unwrap()
    }

    #[test]
    fn no_template_forwards_data() {
        let event = test_event();
        let result = resolve_args(None, &event);
        assert_eq!(result, event.data);
    }

    #[test]
    fn simple_string_replacement() {
        let event = test_event();
        let tmpl = json!({
            "message": "Issue: {{event.data.issue.title}}"
        });
        let result = resolve_args(Some(tmpl), &event);
        assert_eq!(result["message"], "Issue: Fix the login bug");
    }

    #[test]
    fn whole_value_preserves_type() {
        let event = test_event();
        // Entire string is one expression — should return the object, not a string
        let tmpl = json!({
            "payload": "{{event.data.issue}}"
        });
        let result = resolve_args(Some(tmpl), &event);
        assert_eq!(result["payload"]["title"], "Fix the login bug");
        assert_eq!(result["payload"]["number"], 42);
    }

    #[test]
    fn number_preserved_in_whole_value() {
        let event = test_event();
        let tmpl = json!({
            "issue_number": "{{event.data.issue.number}}"
        });
        let result = resolve_args(Some(tmpl), &event);
        assert_eq!(result["issue_number"], 42);
    }

    #[test]
    fn mixed_text_and_templates() {
        let event = test_event();
        let tmpl = json!({
            "message": "New issue #{{event.data.issue.number}} in {{event.data.repository.full_name}}: {{event.data.issue.title}}"
        });
        let result = resolve_args(Some(tmpl), &event);
        assert_eq!(
            result["message"],
            "New issue #42 in org/repo: Fix the login bug"
        );
    }

    #[test]
    fn event_metadata_paths() {
        let event = test_event();
        let tmpl = json!({
            "event_id": "{{event.id}}",
            "event_type": "{{event.type}}",
            "event_source": "{{event.source}}",
            "event_subject": "{{event.subject}}"
        });
        let result = resolve_args(Some(tmpl), &event);
        assert_eq!(result["event_type"], "com.github.issues.opened");
        assert_eq!(result["event_source"], "nexus://extension/webhook-receiver");
        assert_eq!(result["event_subject"], "wh_abc123");
    }

    #[test]
    fn missing_field_returns_null_or_empty() {
        let event = test_event();
        let tmpl = json!({
            "missing_whole": "{{event.data.nonexistent}}",
            "missing_in_text": "value: {{event.data.nonexistent}}"
        });
        let result = resolve_args(Some(tmpl), &event);
        assert_eq!(result["missing_whole"], Value::Null);
        assert_eq!(result["missing_in_text"], "value: ");
    }

    #[test]
    fn nested_template_in_array() {
        let event = test_event();
        let tmpl = json!({
            "tags": ["github", "{{event.data.repository.full_name}}"]
        });
        let result = resolve_args(Some(tmpl), &event);
        assert_eq!(result["tags"][0], "github");
        assert_eq!(result["tags"][1], "org/repo");
    }

    #[test]
    fn non_string_values_pass_through() {
        let event = test_event();
        let tmpl = json!({
            "count": 5,
            "enabled": true,
            "nothing": null
        });
        let result = resolve_args(Some(tmpl), &event);
        assert_eq!(result["count"], 5);
        assert_eq!(result["enabled"], true);
        assert_eq!(result["nothing"], Value::Null);
    }

    #[test]
    fn whitespace_in_braces_tolerated() {
        let event = test_event();
        let tmpl = json!({
            "title": "{{ event.data.issue.title }}"
        });
        let result = resolve_args(Some(tmpl), &event);
        assert_eq!(result["title"], "Fix the login bug");
    }
}
