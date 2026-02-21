use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use super::cloud_event::CloudEvent;

/// What happens when a routing rule matches an event.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum RouteAction {
    InvokePluginTool {
        plugin_id: String,
        tool_name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        args_template: Option<Value>,
    },
    CallExtension {
        extension_id: String,
        operation: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        args_template: Option<Value>,
    },
    EmitFrontend {
        channel: String,
    },
}

// ---------------------------------------------------------------------------
// CloudEvents Subscriptions API — Filter Dialects
// https://github.com/cloudevents/spec/blob/main/subscriptions/spec.md
// ---------------------------------------------------------------------------

/// A single filter expression per the CE Subscriptions spec.
///
/// All required dialects are supported: exact, prefix, suffix, all, any, not.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Filter {
    /// All named attributes must exactly match their values.
    Exact(HashMap<String, String>),
    /// All named attributes must start with their values.
    Prefix(HashMap<String, String>),
    /// All named attributes must end with their values.
    Suffix(HashMap<String, String>),
    /// All nested filters must match (AND).
    All(Vec<Filter>),
    /// At least one nested filter must match (OR).
    Any(Vec<Filter>),
    /// Nested filter must NOT match.
    Not(Box<Filter>),
}

impl Filter {
    /// Evaluate this filter against a CloudEvent.
    pub fn matches(&self, event: &CloudEvent) -> bool {
        match self {
            Filter::Exact(attrs) => attrs
                .iter()
                .all(|(k, v)| event.get_attr(k).is_some_and(|a| a == v)),
            Filter::Prefix(attrs) => attrs
                .iter()
                .all(|(k, v)| event.get_attr(k).is_some_and(|a| a.starts_with(v.as_str()))),
            Filter::Suffix(attrs) => attrs
                .iter()
                .all(|(k, v)| event.get_attr(k).is_some_and(|a| a.ends_with(v.as_str()))),
            Filter::All(filters) => filters.iter().all(|f| f.matches(event)),
            Filter::Any(filters) => filters.iter().any(|f| f.matches(event)),
            Filter::Not(filter) => !filter.matches(event),
        }
    }
}

/// A routing rule that matches events by CE filters and triggers an action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingRule {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// CE Subscriptions filter array. All filters must match (implicit AND).
    #[serde(default)]
    pub filters: Vec<Filter>,
    pub action: RouteAction,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_created_by")]
    pub created_by: String,
}

fn default_true() -> bool {
    true
}

fn default_created_by() -> String {
    "user".to_string()
}

impl RoutingRule {
    /// Check if this rule matches a given CloudEvent.
    /// All filters must match (per CE Subscriptions spec: implicit AND).
    pub fn matches(&self, event: &CloudEvent) -> bool {
        if !self.enabled {
            return false;
        }
        self.filters.iter().all(|f| f.matches(event))
    }
}

/// File-based persistent store for routing rules.
pub struct RoutingRuleStore {
    rules: Vec<RoutingRule>,
    path: PathBuf,
}

impl RoutingRuleStore {
    /// Load rules from disk, or create an empty store if the file doesn't exist.
    pub fn load(data_dir: &std::path::Path) -> Self {
        let path = data_dir.join("event_rules.json");
        let rules = if path.exists() {
            match std::fs::read_to_string(&path) {
                Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
                Err(e) => {
                    log::warn!("Failed to read event rules: {}", e);
                    Vec::new()
                }
            }
        } else {
            Vec::new()
        };

        Self { rules, path }
    }

    /// Persist current rules to disk.
    fn save(&self) -> Result<(), String> {
        let json = serde_json::to_string_pretty(&self.rules)
            .map_err(|e| format!("Failed to serialize rules: {}", e))?;
        std::fs::write(&self.path, json)
            .map_err(|e| format!("Failed to write rules file: {}", e))?;
        Ok(())
    }

    /// List all routing rules.
    pub fn list(&self) -> &[RoutingRule] {
        &self.rules
    }

    /// Get a rule by ID.
    pub fn get(&self, id: &str) -> Option<&RoutingRule> {
        self.rules.iter().find(|r| r.id == id)
    }

    /// Add a new routing rule. Returns the assigned ID.
    pub fn create(&mut self, mut rule: RoutingRule) -> Result<String, String> {
        if rule.id.is_empty() {
            rule.id = format!("rule_{}", Uuid::new_v4().simple());
        }

        // Validate: an enabled rule with no filters matches everything — warn but allow.
        if rule.enabled && rule.filters.is_empty() {
            log::warn!(
                "Rule '{}' has no filters and will match every event",
                rule.id
            );
        }

        let id = rule.id.clone();
        self.rules.push(rule);
        self.save()?;
        Ok(id)
    }

    /// Update an existing rule. Returns an error if not found.
    pub fn update(&mut self, id: &str, update: RoutingRuleUpdate) -> Result<(), String> {
        let rule = self
            .rules
            .iter_mut()
            .find(|r| r.id == id)
            .ok_or_else(|| format!("Rule '{}' not found", id))?;

        if let Some(name) = update.name {
            rule.name = name;
        }
        if let Some(filters) = update.filters {
            rule.filters = filters;
        }
        if let Some(action) = update.action {
            rule.action = action;
        }
        if let Some(enabled) = update.enabled {
            rule.enabled = enabled;
        }

        self.save()
    }

    /// Delete a rule by ID.
    pub fn delete(&mut self, id: &str) -> Result<(), String> {
        let len_before = self.rules.len();
        self.rules.retain(|r| r.id != id);
        if self.rules.len() == len_before {
            return Err(format!("Rule '{}' not found", id));
        }
        self.save()
    }

    /// Find all enabled rules that match a given event.
    pub fn matching_rules(&self, event: &CloudEvent) -> Vec<&RoutingRule> {
        self.rules.iter().filter(|r| r.matches(event)).collect()
    }
}

/// Partial update for a routing rule.
#[derive(Debug, Default, Deserialize)]
pub struct RoutingRuleUpdate {
    #[serde(default)]
    pub name: Option<Option<String>>,
    #[serde(default)]
    pub filters: Option<Vec<Filter>>,
    #[serde(default)]
    pub action: Option<RouteAction>,
    #[serde(default)]
    pub enabled: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_event(event_type: &str, source: &str) -> CloudEvent {
        CloudEvent::builder()
            .source(source)
            .event_type(event_type)
            .build()
            .unwrap()
    }

    fn make_event_with_subject(event_type: &str, source: &str, subject: &str) -> CloudEvent {
        CloudEvent::builder()
            .source(source)
            .event_type(event_type)
            .subject(subject)
            .build()
            .unwrap()
    }

    // -- Filter dialect tests --

    #[test]
    fn exact_filter_matches() {
        let f = Filter::Exact(HashMap::from([("type".into(), "com.github.push".into())]));
        assert!(f.matches(&make_event("com.github.push", "s")));
        assert!(!f.matches(&make_event("com.github.pr", "s")));
    }

    #[test]
    fn exact_filter_multiple_attrs() {
        let f = Filter::Exact(HashMap::from([
            ("type".into(), "com.github.push".into()),
            ("source".into(), "nexus://ext/wh".into()),
        ]));
        assert!(f.matches(&make_event("com.github.push", "nexus://ext/wh")));
        assert!(!f.matches(&make_event("com.github.push", "other")));
    }

    #[test]
    fn prefix_filter_matches() {
        let f = Filter::Prefix(HashMap::from([("type".into(), "com.github.".into())]));
        assert!(f.matches(&make_event("com.github.push", "s")));
        assert!(f.matches(&make_event("com.github.issues.opened", "s")));
        assert!(!f.matches(&make_event("com.gitlab.push", "s")));
    }

    #[test]
    fn suffix_filter_matches() {
        let f = Filter::Suffix(HashMap::from([("type".into(), ".opened".into())]));
        assert!(f.matches(&make_event("com.github.issues.opened", "s")));
        assert!(!f.matches(&make_event("com.github.issues.closed", "s")));
    }

    #[test]
    fn all_filter_requires_every_match() {
        let f = Filter::All(vec![
            Filter::Prefix(HashMap::from([("type".into(), "com.github.".into())])),
            Filter::Prefix(HashMap::from([("source".into(), "nexus://".into())])),
        ]);
        assert!(f.matches(&make_event("com.github.push", "nexus://ext/wh")));
        assert!(!f.matches(&make_event("com.github.push", "other")));
    }

    #[test]
    fn any_filter_requires_one_match() {
        let f = Filter::Any(vec![
            Filter::Exact(HashMap::from([("type".into(), "com.github.push".into())])),
            Filter::Exact(HashMap::from([("type".into(), "com.github.pr".into())])),
        ]);
        assert!(f.matches(&make_event("com.github.push", "s")));
        assert!(f.matches(&make_event("com.github.pr", "s")));
        assert!(!f.matches(&make_event("com.github.issues", "s")));
    }

    #[test]
    fn not_filter_inverts() {
        let f = Filter::Not(Box::new(Filter::Exact(HashMap::from([(
            "type".into(),
            "com.github.push".into(),
        )]))));
        assert!(!f.matches(&make_event("com.github.push", "s")));
        assert!(f.matches(&make_event("com.github.pr", "s")));
    }

    #[test]
    fn subject_filter() {
        let f = Filter::Exact(HashMap::from([("subject".into(), "wh_abc".into())]));
        assert!(f.matches(&make_event_with_subject("t", "s", "wh_abc")));
        assert!(!f.matches(&make_event_with_subject("t", "s", "wh_xyz")));
        // No subject set
        assert!(!f.matches(&make_event("t", "s")));
    }

    // -- RoutingRule tests --

    #[test]
    fn disabled_rule_never_matches() {
        let rule = RoutingRule {
            id: "r1".into(),
            name: None,
            filters: vec![],
            action: RouteAction::EmitFrontend {
                channel: "test".into(),
            },
            enabled: false,
            created_by: "user".into(),
        };
        assert!(!rule.matches(&make_event("anything", "s")));
    }

    #[test]
    fn rule_with_no_filters_matches_everything() {
        let rule = RoutingRule {
            id: "r2".into(),
            name: None,
            filters: vec![],
            action: RouteAction::EmitFrontend {
                channel: "test".into(),
            },
            enabled: true,
            created_by: "user".into(),
        };
        assert!(rule.matches(&make_event("anything", "any-source")));
    }

    #[test]
    fn rule_with_prefix_filter() {
        let rule = RoutingRule {
            id: "r3".into(),
            name: None,
            filters: vec![Filter::Prefix(HashMap::from([(
                "type".into(),
                "com.github.".into(),
            )]))],
            action: RouteAction::EmitFrontend {
                channel: "test".into(),
            },
            enabled: true,
            created_by: "user".into(),
        };
        assert!(rule.matches(&make_event("com.github.push", "s")));
        assert!(rule.matches(&make_event("com.github.issues.opened", "s")));
        assert!(!rule.matches(&make_event("com.gitlab.push", "s")));
    }

    // -- Store tests --

    #[test]
    fn store_crud_and_persistence() {
        let tmp = TempDir::new().unwrap();
        let mut store = RoutingRuleStore::load(tmp.path());
        assert_eq!(store.list().len(), 0);

        let id = store
            .create(RoutingRule {
                id: String::new(),
                name: Some("GitHub events".into()),
                filters: vec![Filter::Prefix(HashMap::from([(
                    "type".into(),
                    "com.github.".into(),
                )]))],
                action: RouteAction::EmitFrontend {
                    channel: "gh".into(),
                },
                enabled: true,
                created_by: "user".into(),
            })
            .unwrap();

        assert_eq!(store.list().len(), 1);
        assert!(store.get(&id).is_some());

        // Reload from disk
        let store2 = RoutingRuleStore::load(tmp.path());
        assert_eq!(store2.list().len(), 1);
        assert_eq!(store2.list()[0].id, id);

        // Update
        store
            .update(
                &id,
                RoutingRuleUpdate {
                    enabled: Some(false),
                    ..Default::default()
                },
            )
            .unwrap();
        assert!(!store.get(&id).unwrap().enabled);

        // Delete
        store.delete(&id).unwrap();
        assert_eq!(store.list().len(), 0);
    }

    #[test]
    fn matching_rules_returns_all_matches() {
        let tmp = TempDir::new().unwrap();
        let mut store = RoutingRuleStore::load(tmp.path());

        store
            .create(RoutingRule {
                id: String::new(),
                name: None,
                filters: vec![Filter::Prefix(HashMap::from([(
                    "type".into(),
                    "com.github.".into(),
                )]))],
                action: RouteAction::EmitFrontend {
                    channel: "a".into(),
                },
                enabled: true,
                created_by: "user".into(),
            })
            .unwrap();

        // Catch-all rule (no filters)
        store
            .create(RoutingRule {
                id: String::new(),
                name: None,
                filters: vec![],
                action: RouteAction::EmitFrontend {
                    channel: "b".into(),
                },
                enabled: true,
                created_by: "user".into(),
            })
            .unwrap();

        let event = make_event("com.github.push", "s");
        let matches = store.matching_rules(&event);
        assert_eq!(matches.len(), 2);
    }

    #[test]
    fn filter_serde_roundtrip() {
        let rule = RoutingRule {
            id: "test".into(),
            name: Some("Test rule".into()),
            filters: vec![
                Filter::Prefix(HashMap::from([("type".into(), "com.github.".into())])),
                Filter::Exact(HashMap::from([(
                    "source".into(),
                    "nexus://ext/wh".into(),
                )])),
            ],
            action: RouteAction::InvokePluginTool {
                plugin_id: "agent".into(),
                tool_name: "send_message".into(),
                args_template: None,
            },
            enabled: true,
            created_by: "user".into(),
        };

        let json = serde_json::to_string(&rule).unwrap();
        let deserialized: RoutingRule = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "test");
        assert_eq!(deserialized.filters.len(), 2);
        assert!(deserialized.matches(&make_event("com.github.push", "nexus://ext/wh")));
        assert!(!deserialized.matches(&make_event("com.github.push", "other")));
    }
}
