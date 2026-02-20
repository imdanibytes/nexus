use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use super::cloud_event::CloudEvent;

/// Identifies who is subscribing to events.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SubscriberKind {
    Extension { ext_id: String },
    Plugin { plugin_id: String },
    Frontend { channel: String },
    Internal,
}

/// A subscription registered on the event bus.
pub struct Subscription {
    pub id: String,
    pub type_pattern: glob::Pattern,
    pub source_pattern: Option<glob::Pattern>,
    pub kind: SubscriberKind,
    pub sender: mpsc::UnboundedSender<CloudEvent>,
}

impl Subscription {
    /// Check if a CloudEvent matches this subscription's patterns.
    pub fn matches(&self, event: &CloudEvent) -> bool {
        if !self.type_pattern.matches(&event.event_type) {
            return false;
        }
        if let Some(ref sp) = self.source_pattern {
            if !sp.matches(&event.source) {
                return false;
            }
        }
        true
    }

    /// Attempt to send an event to this subscriber. Returns false if the
    /// channel is closed (subscriber disconnected).
    pub fn try_send(&self, event: &CloudEvent) -> bool {
        self.sender.send(event.clone()).is_ok()
    }
}

/// Parse a glob pattern string, returning a descriptive error.
pub fn parse_pattern(pattern: &str) -> Result<glob::Pattern, String> {
    glob::Pattern::new(pattern)
        .map_err(|e| format!("Invalid glob pattern '{}': {}", pattern, e))
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
    fn type_pattern_matching() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let sub = Subscription {
            id: "sub_1".into(),
            type_pattern: parse_pattern("com.github.*").unwrap(),
            source_pattern: None,
            kind: SubscriberKind::Internal,
            sender: tx,
        };

        assert!(sub.matches(&make_event("com.github.push", "nexus://core")));
        assert!(sub.matches(&make_event("com.github.pull_request", "nexus://core")));
        assert!(!sub.matches(&make_event("com.gitlab.push", "nexus://core")));
        assert!(!sub.matches(&make_event(
            "nexus.lifecycle.plugin.started",
            "nexus://core"
        )));
    }

    #[test]
    fn wildcard_matches_everything() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let sub = Subscription {
            id: "sub_2".into(),
            type_pattern: parse_pattern("*").unwrap(),
            source_pattern: None,
            kind: SubscriberKind::Internal,
            sender: tx,
        };

        assert!(sub.matches(&make_event("anything.at.all", "nexus://core")));
    }

    #[test]
    fn source_pattern_filters() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let sub = Subscription {
            id: "sub_3".into(),
            type_pattern: parse_pattern("*").unwrap(),
            source_pattern: Some(parse_pattern("nexus://extension/*").unwrap()),
            kind: SubscriberKind::Extension {
                ext_id: "test".into(),
            },
            sender: tx,
        };

        assert!(sub.matches(&make_event("test", "nexus://extension/webhook-receiver")));
        assert!(!sub.matches(&make_event("test", "nexus://core")));
        assert!(!sub.matches(&make_event("test", "nexus://plugin/agent")));
    }

    #[test]
    fn closed_channel_returns_false() {
        let (tx, rx) = mpsc::unbounded_channel();
        drop(rx);
        let sub = Subscription {
            id: "sub_4".into(),
            type_pattern: parse_pattern("*").unwrap(),
            source_pattern: None,
            kind: SubscriberKind::Internal,
            sender: tx,
        };

        let event = make_event("test", "nexus://core");
        assert!(!sub.try_send(&event));
    }
}
