use axum::{
    extract::Query,
    http::StatusCode,
    response::sse::{Event, KeepAlive, Sse},
    Extension, Json,
};
use futures_util::Stream;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

use crate::event_bus::cloud_event::{CloudEvent, PublishRequest};
use crate::event_bus::executor::RouteActionExecutor;
use crate::event_bus::log::EventLogQuery;
use crate::event_bus::subscription::SubscriberKind;
use crate::event_bus::{SharedEventBus, SharedEventStore};

use super::middleware::AuthenticatedPlugin;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Serialize, ToSchema)]
pub struct PublishResponse {
    pub event_id: String,
}

#[derive(Serialize, ToSchema)]
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

#[derive(Serialize, ToSchema)]
pub struct EventLogResponse {
    pub events: Vec<EventLogEntry>,
    pub total: usize,
}

#[derive(Deserialize)]
pub struct SubscribeQuery {
    #[serde(default = "default_pattern")]
    pub type_pattern: String,
    pub source_pattern: Option<String>,
}

fn default_pattern() -> String {
    "*".to_string()
}

#[derive(Deserialize)]
pub struct LogQuery {
    #[serde(rename = "type")]
    pub event_type: Option<String>,
    pub source: Option<String>,
    pub since: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Serialize, ToSchema)]
pub struct EventErrorResponse {
    pub error: String,
}

// ---------------------------------------------------------------------------
// POST /v1/events — publish a CloudEvent
// ---------------------------------------------------------------------------

#[utoipa::path(
    post,
    path = "/api/v1/events",
    tag = "events",
    security(("bearer_auth" = [])),
    request_body = PublishRequest,
    responses(
        (status = 200, description = "Event published", body = PublishResponse),
        (status = 400, description = "Invalid event", body = EventErrorResponse),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn publish_event(
    Extension(auth): Extension<AuthenticatedPlugin>,
    Extension(event_bus): Extension<SharedEventBus>,
    Extension(executor): Extension<RouteActionExecutor>,
    Extension(event_store): Extension<SharedEventStore>,
    Json(req): Json<PublishRequest>,
) -> Result<Json<PublishResponse>, (StatusCode, Json<EventErrorResponse>)> {
    let source = format!("nexus://plugin/{}", auth.plugin_id);
    let event = req.into_cloud_event(source);
    let event_id = event.id.clone();

    let actions = {
        let mut bus = event_bus.write().await;
        bus.publish(event.clone())
    };

    if !actions.is_empty() {
        executor.execute_durable(&event_store, actions, &event);
    }

    Ok(Json(PublishResponse { event_id }))
}

// ---------------------------------------------------------------------------
// GET /v1/events/subscribe — SSE stream of matching events
// ---------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/api/v1/events/subscribe",
    tag = "events",
    security(("bearer_auth" = [])),
    params(
        ("type_pattern" = Option<String>, Query, description = "Glob pattern for event type (default: *)"),
        ("source_pattern" = Option<String>, Query, description = "Glob pattern for event source"),
    ),
    responses(
        (status = 200, description = "SSE stream of matching events"),
        (status = 400, description = "Invalid pattern", body = EventErrorResponse),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn subscribe_events(
    Extension(auth): Extension<AuthenticatedPlugin>,
    Extension(event_bus): Extension<SharedEventBus>,
    Query(params): Query<SubscribeQuery>,
) -> Result<Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>>, (StatusCode, Json<EventErrorResponse>)>
{
    let mut bus = event_bus.write().await;
    let (_sub_id, mut rx) = bus
        .subscribe(
            &params.type_pattern,
            params.source_pattern.as_deref(),
            SubscriberKind::Plugin {
                plugin_id: auth.plugin_id.clone(),
            },
        )
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(EventErrorResponse { error: e }),
            )
        })?;
    drop(bus);

    let stream = async_stream::stream! {
        while let Some(event) = rx.recv().await {
            let entry = EventLogEntry::from(&event);
            if let Ok(data) = serde_json::to_string(&entry) {
                yield Ok(Event::default()
                    .event(event.event_type.clone())
                    .data(data));
            }
        }
    };

    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}

// ---------------------------------------------------------------------------
// GET /v1/events/log — query the event log
// ---------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/api/v1/events/log",
    tag = "events",
    security(("bearer_auth" = [])),
    params(
        ("type" = Option<String>, Query, description = "Filter by event type (glob)"),
        ("source" = Option<String>, Query, description = "Filter by source (glob)"),
        ("since" = Option<String>, Query, description = "RFC 3339 timestamp lower bound"),
        ("limit" = Option<usize>, Query, description = "Max results to return"),
    ),
    responses(
        (status = 200, description = "Matching events", body = EventLogResponse),
        (status = 400, description = "Invalid query", body = EventErrorResponse),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn query_event_log(
    Extension(_auth): Extension<AuthenticatedPlugin>,
    Extension(event_bus): Extension<SharedEventBus>,
    Query(params): Query<LogQuery>,
) -> Result<Json<EventLogResponse>, (StatusCode, Json<EventErrorResponse>)> {
    let since = params
        .since
        .as_deref()
        .map(|s| {
            chrono::DateTime::parse_from_rfc3339(s)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .map_err(|e| {
                    (
                        StatusCode::BAD_REQUEST,
                        Json(EventErrorResponse {
                            error: format!("Invalid 'since' timestamp: {}", e),
                        }),
                    )
                })
        })
        .transpose()?;

    let bus = event_bus.read().await;
    let query = EventLogQuery {
        event_type: params.event_type,
        source: params.source,
        since,
        until: None,
        limit: params.limit,
    };

    let events = bus.query_log(&query);
    let total = events.len();
    let entries: Vec<EventLogEntry> = events.into_iter().map(EventLogEntry::from).collect();

    Ok(Json(EventLogResponse {
        events: entries,
        total,
    }))
}
