pub mod cloud_event;
pub mod dispatch;
pub mod executor;
pub mod log;
pub mod retry_worker;
pub mod routing;
pub mod store;
pub mod subscription;

pub use dispatch::Dispatch;

use std::path::Path;
use std::sync::Arc;

use tokio::sync::{mpsc, RwLock};

use cloud_event::CloudEvent;
use log::{EventLog, EventLogQuery};
use routing::{RouteAction, RoutingRule, RoutingRuleStore, RoutingRuleUpdate};
use subscription::{parse_pattern, SubscriberKind, Subscription};

/// Thread-safe shared handle to the event bus.
pub type SharedEventBus = Arc<RwLock<EventBus>>;

/// Thread-safe shared handle to the durable event store.
pub type SharedEventStore = Arc<store::EventStore>;

/// In-process CloudEvents event bus with pub/sub, routing rules, and an event log.
pub struct EventBus {
    subscriptions: Vec<Subscription>,
    next_sub_id: u64,
    event_log: EventLog,
    routing_rules: RoutingRuleStore,
}

impl EventBus {
    /// Create a new event bus, loading routing rules from the given data directory.
    pub fn new(data_dir: &Path) -> Self {
        Self {
            subscriptions: Vec::new(),
            next_sub_id: 0,
            event_log: EventLog::new(),
            routing_rules: RoutingRuleStore::load(data_dir),
        }
    }

    /// Publish a CloudEvent: log it, fan out to matching subscribers, return matching routing rules.
    ///
    /// Returns the list of route actions that should be executed (caller spawns tasks).
    pub fn publish(&mut self, event: CloudEvent) -> Vec<RouteAction> {
        // Log the event
        self.event_log.push(event.clone());

        // Fan out to subscribers, cleaning up dead channels
        self.subscriptions.retain(|sub| {
            if sub.matches(&event) {
                sub.try_send(&event)
            } else {
                // Keep non-matching subscribers — they're still alive
                true
            }
        });

        // Collect matching routing rule actions
        self.routing_rules
            .matching_rules(&event)
            .into_iter()
            .map(|r| r.action.clone())
            .collect()
    }

    /// Register a new subscription. Returns the subscription ID and a receiver for events.
    pub fn subscribe(
        &mut self,
        type_pattern: &str,
        source_pattern: Option<&str>,
        kind: SubscriberKind,
    ) -> Result<(String, mpsc::UnboundedReceiver<CloudEvent>), String> {
        let tp = parse_pattern(type_pattern)?;
        let sp = source_pattern.map(parse_pattern).transpose()?;

        let (tx, rx) = mpsc::unbounded_channel();
        let sub_id = format!("sub_{}", self.next_sub_id);
        self.next_sub_id += 1;

        self.subscriptions.push(Subscription {
            id: sub_id.clone(),
            type_pattern: tp,
            source_pattern: sp,
            kind,
            sender: tx,
        });

        Ok((sub_id, rx))
    }

    /// Remove a subscription by ID.
    pub fn unsubscribe(&mut self, sub_id: &str) {
        self.subscriptions.retain(|s| s.id != sub_id);
    }

    /// Query the event log.
    pub fn query_log(&self, query: &EventLogQuery) -> Vec<&CloudEvent> {
        self.event_log.query(query)
    }

    /// Get the number of events in the log.
    pub fn log_len(&self) -> usize {
        self.event_log.len()
    }

    // -- Routing rule CRUD delegated to the store --

    pub fn list_routing_rules(&self) -> &[RoutingRule] {
        self.routing_rules.list()
    }

    pub fn get_routing_rule(&self, id: &str) -> Option<&RoutingRule> {
        self.routing_rules.get(id)
    }

    pub fn create_routing_rule(&mut self, rule: RoutingRule) -> Result<String, String> {
        self.routing_rules.create(rule)
    }

    pub fn update_routing_rule(
        &mut self,
        id: &str,
        update: RoutingRuleUpdate,
    ) -> Result<(), String> {
        self.routing_rules.update(id, update)
    }

    pub fn delete_routing_rule(&mut self, id: &str) -> Result<(), String> {
        self.routing_rules.delete(id)
    }

    /// Get subscription info for diagnostics.
    pub fn subscription_count(&self) -> usize {
        self.subscriptions.len()
    }

    /// Get active subscription IDs for an extension.
    pub fn extension_subscriptions(&self, ext_id: &str) -> Vec<String> {
        self.subscriptions
            .iter()
            .filter(
                |s| matches!(&s.kind, SubscriberKind::Extension { ext_id: id } if id == ext_id),
            )
            .map(|s| s.id.clone())
            .collect()
    }
}

/// Create a new SharedEventBus instance.
pub fn create_event_bus(data_dir: &Path) -> SharedEventBus {
    Arc::new(RwLock::new(EventBus::new(data_dir)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_bus() -> (TempDir, EventBus) {
        let tmp = TempDir::new().unwrap();
        let bus = EventBus::new(tmp.path());
        (tmp, bus)
    }

    #[test]
    fn publish_logs_and_fans_out() {
        let (_tmp, mut bus) = make_bus();

        let (_sub_id, mut rx) = bus
            .subscribe("com.github.*", None, SubscriberKind::Internal)
            .unwrap();

        let event = CloudEvent::builder()
            .source("nexus://core")
            .event_type("com.github.push")
            .build()
            .unwrap();

        let actions = bus.publish(event);
        assert!(actions.is_empty());

        let received = rx.try_recv().unwrap();
        assert_eq!(received.event_type, "com.github.push");

        assert_eq!(bus.log_len(), 1);
    }

    #[test]
    fn non_matching_subscribers_kept_alive() {
        let (_tmp, mut bus) = make_bus();

        let (_sub_id, mut rx) = bus
            .subscribe("com.github.*", None, SubscriberKind::Internal)
            .unwrap();

        let event = CloudEvent::builder()
            .source("nexus://core")
            .event_type("nexus.lifecycle.started")
            .build()
            .unwrap();

        bus.publish(event);

        assert!(rx.try_recv().is_err());
        assert_eq!(bus.subscription_count(), 1);
    }

    #[test]
    fn dead_subscribers_cleaned_up() {
        let (_tmp, mut bus) = make_bus();

        let (_sub_id, rx) = bus
            .subscribe("*", None, SubscriberKind::Internal)
            .unwrap();

        drop(rx);

        let event = CloudEvent::builder()
            .source("nexus://core")
            .event_type("test")
            .build()
            .unwrap();

        bus.publish(event);

        assert_eq!(bus.subscription_count(), 0);
    }

    #[test]
    fn routing_rules_return_actions() {
        let (_tmp, mut bus) = make_bus();

        bus.create_routing_rule(RoutingRule {
            id: String::new(),
            type_pattern: "com.github.*".into(),
            source_pattern: None,
            action: RouteAction::EmitFrontend {
                channel: "gh-events".into(),
            },
            enabled: true,
            created_by: "user".into(),
        })
        .unwrap();

        let event = CloudEvent::builder()
            .source("nexus://extension/wh")
            .event_type("com.github.push")
            .build()
            .unwrap();

        let actions = bus.publish(event);
        assert_eq!(actions.len(), 1);
        assert!(matches!(actions[0], RouteAction::EmitFrontend { .. }));
    }

    #[test]
    fn unsubscribe_removes_subscription() {
        let (_tmp, mut bus) = make_bus();

        let (sub_id, _rx) = bus
            .subscribe("*", None, SubscriberKind::Internal)
            .unwrap();

        assert_eq!(bus.subscription_count(), 1);
        bus.unsubscribe(&sub_id);
        assert_eq!(bus.subscription_count(), 0);
    }
}
