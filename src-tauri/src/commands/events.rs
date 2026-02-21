use serde::Serialize;
use serde_json::Value;

use crate::event_bus::cloud_event::CloudEvent;
use crate::event_bus::log::EventLogQuery;
use crate::event_bus::routing::{Filter, RouteAction, RoutingRule, RoutingRuleUpdate};
use crate::event_bus::SharedEventBus;

// -- Event Log commands --

#[derive(Debug, Serialize)]
pub struct EventLogEntry {
    pub id: String,
    pub source: String,
    #[serde(rename = "type")]
    pub event_type: String,
    pub time: String,
    pub subject: Option<String>,
    pub data: Value,
}

impl From<&CloudEvent> for EventLogEntry {
    fn from(e: &CloudEvent) -> Self {
        Self {
            id: e.id.clone(),
            source: e.source.clone(),
            event_type: e.event_type.clone(),
            time: e.time.to_rfc3339(),
            subject: e.subject.clone(),
            data: e.data.clone(),
        }
    }
}

/// Query the event log.
#[tauri::command]
pub async fn event_log_query(
    event_bus: tauri::State<'_, SharedEventBus>,
    event_type: Option<String>,
    source: Option<String>,
    since: Option<String>,
    limit: Option<usize>,
) -> Result<Vec<EventLogEntry>, String> {
    let since_dt = since
        .as_deref()
        .map(|s| {
            chrono::DateTime::parse_from_rfc3339(s)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .map_err(|e| format!("Invalid 'since' timestamp: {}", e))
        })
        .transpose()?;

    let bus = event_bus.read().await;
    let query = EventLogQuery {
        event_type,
        source,
        since: since_dt,
        until: None,
        limit,
    };

    let events = bus.query_log(&query);
    Ok(events.into_iter().map(EventLogEntry::from).collect())
}

/// Get the total number of events in the log.
#[tauri::command]
pub async fn event_log_count(
    event_bus: tauri::State<'_, SharedEventBus>,
) -> Result<usize, String> {
    let bus = event_bus.read().await;
    Ok(bus.log_len())
}

// -- Routing Rule commands --

/// List all routing rules.
#[tauri::command]
pub async fn routing_rule_list(
    event_bus: tauri::State<'_, SharedEventBus>,
) -> Result<Vec<RoutingRule>, String> {
    let bus = event_bus.read().await;
    Ok(bus.list_routing_rules().to_vec())
}

/// Get a routing rule by ID.
#[tauri::command]
pub async fn routing_rule_get(
    event_bus: tauri::State<'_, SharedEventBus>,
    rule_id: String,
) -> Result<RoutingRule, String> {
    let bus = event_bus.read().await;
    bus.get_routing_rule(&rule_id)
        .cloned()
        .ok_or_else(|| format!("Rule '{}' not found", rule_id))
}

/// Create a new routing rule with CE Subscriptions-compatible filters.
#[tauri::command]
pub async fn routing_rule_create(
    event_bus: tauri::State<'_, SharedEventBus>,
    name: Option<String>,
    filters: Vec<Filter>,
    action: RouteAction,
) -> Result<String, String> {
    let mut bus = event_bus.write().await;
    bus.create_routing_rule(RoutingRule {
        id: String::new(),
        name,
        filters,
        action,
        enabled: true,
        created_by: "user".to_string(),
    })
}

/// Update a routing rule.
#[tauri::command]
pub async fn routing_rule_update(
    event_bus: tauri::State<'_, SharedEventBus>,
    rule_id: String,
    name: Option<Option<String>>,
    filters: Option<Vec<Filter>>,
    action: Option<RouteAction>,
    enabled: Option<bool>,
) -> Result<(), String> {
    let mut bus = event_bus.write().await;
    bus.update_routing_rule(
        &rule_id,
        RoutingRuleUpdate {
            name,
            filters,
            action,
            enabled,
        },
    )
}

/// Delete a routing rule.
#[tauri::command]
pub async fn routing_rule_delete(
    event_bus: tauri::State<'_, SharedEventBus>,
    rule_id: String,
) -> Result<(), String> {
    let mut bus = event_bus.write().await;
    bus.delete_routing_rule(&rule_id)
}
