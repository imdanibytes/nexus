use std::collections::VecDeque;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::cloud_event::CloudEvent;

const DEFAULT_CAPACITY: usize = 10_000;

/// In-memory ring buffer for recent CloudEvents.
pub struct EventLog {
    events: VecDeque<CloudEvent>,
    capacity: usize,
}

impl EventLog {
    pub fn new() -> Self {
        Self {
            events: VecDeque::with_capacity(DEFAULT_CAPACITY),
            capacity: DEFAULT_CAPACITY,
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            events: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    /// Push an event into the log, evicting the oldest if at capacity.
    pub fn push(&mut self, event: CloudEvent) {
        if self.events.len() >= self.capacity {
            self.events.pop_front();
        }
        self.events.push_back(event);
    }

    /// Query events matching the given filters.
    pub fn query(&self, filter: &EventLogQuery) -> Vec<&CloudEvent> {
        let mut results: Vec<&CloudEvent> = self
            .events
            .iter()
            .filter(|e| {
                if let Some(ref t) = filter.event_type {
                    if let Ok(pattern) = glob::Pattern::new(t) {
                        if !pattern.matches(&e.event_type) {
                            return false;
                        }
                    } else if e.event_type != *t {
                        return false;
                    }
                }
                if let Some(ref source) = filter.source {
                    if e.source != *source {
                        return false;
                    }
                }
                if let Some(since) = filter.since {
                    if e.time < since {
                        return false;
                    }
                }
                if let Some(until) = filter.until {
                    if e.time > until {
                        return false;
                    }
                }
                true
            })
            .collect();

        // Most recent first
        results.reverse();

        if let Some(limit) = filter.limit {
            results.truncate(limit);
        }

        results
    }

    /// Total events currently in the log.
    pub fn len(&self) -> usize {
        self.events.len()
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

impl Default for EventLog {
    fn default() -> Self {
        Self::new()
    }
}

/// Query parameters for the event log.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct EventLogQuery {
    /// Filter by event type (supports glob patterns).
    #[serde(rename = "type", default)]
    pub event_type: Option<String>,
    /// Filter by source URI.
    #[serde(default)]
    pub source: Option<String>,
    /// Only events after this timestamp.
    #[serde(default)]
    pub since: Option<DateTime<Utc>>,
    /// Only events before this timestamp.
    #[serde(default)]
    pub until: Option<DateTime<Utc>>,
    /// Maximum number of events to return.
    #[serde(default)]
    pub limit: Option<usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_event(event_type: &str, source: &str) -> CloudEvent {
        CloudEvent::builder()
            .source(source)
            .event_type(event_type)
            .build()
            .unwrap()
    }

    #[test]
    fn ring_buffer_evicts_oldest() {
        let mut log = EventLog::with_capacity(3);
        log.push(make_event("a", "s"));
        log.push(make_event("b", "s"));
        log.push(make_event("c", "s"));
        assert_eq!(log.len(), 3);

        log.push(make_event("d", "s"));
        assert_eq!(log.len(), 3);

        let all = log.query(&EventLogQuery::default());
        assert_eq!(all.len(), 3);
        assert_eq!(all[0].event_type, "d");
        assert_eq!(all[2].event_type, "b");
    }

    #[test]
    fn query_filters_by_type() {
        let mut log = EventLog::new();
        log.push(make_event("com.github.push", "s"));
        log.push(make_event("com.github.pr", "s"));
        log.push(make_event("nexus.lifecycle.started", "s"));

        let results = log.query(&EventLogQuery {
            event_type: Some("com.github.*".to_string()),
            ..Default::default()
        });
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn query_filters_by_source() {
        let mut log = EventLog::new();
        log.push(make_event("t", "nexus://core"));
        log.push(make_event("t", "nexus://extension/wh"));

        let results = log.query(&EventLogQuery {
            source: Some("nexus://core".to_string()),
            ..Default::default()
        });
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].source, "nexus://core");
    }

    #[test]
    fn query_respects_limit() {
        let mut log = EventLog::new();
        for i in 0..100 {
            log.push(make_event(&format!("type_{}", i), "s"));
        }

        let results = log.query(&EventLogQuery {
            limit: Some(5),
            ..Default::default()
        });
        assert_eq!(results.len(), 5);
        assert_eq!(results[0].event_type, "type_99");
    }

    #[test]
    fn query_filters_by_time_range() {
        let mut log = EventLog::new();
        log.push(make_event("a", "s"));
        log.push(make_event("b", "s"));

        let future = Utc::now() + chrono::Duration::hours(1);
        let results = log.query(&EventLogQuery {
            since: Some(future),
            ..Default::default()
        });
        assert_eq!(results.len(), 0);
    }
}
