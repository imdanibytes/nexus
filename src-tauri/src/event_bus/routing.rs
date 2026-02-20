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

/// A routing rule that matches events by pattern and triggers an action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingRule {
    pub id: String,
    pub type_pattern: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_pattern: Option<String>,
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
    pub fn matches(&self, event: &CloudEvent) -> bool {
        if !self.enabled {
            return false;
        }

        let type_ok = match glob::Pattern::new(&self.type_pattern) {
            Ok(p) => p.matches(&event.event_type),
            Err(_) => self.type_pattern == event.event_type,
        };
        if !type_ok {
            return false;
        }

        if let Some(ref sp) = self.source_pattern {
            let source_ok = match glob::Pattern::new(sp) {
                Ok(p) => p.matches(&event.source),
                Err(_) => *sp == event.source,
            };
            if !source_ok {
                return false;
            }
        }

        true
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

        // Validate the type pattern
        glob::Pattern::new(&rule.type_pattern)
            .map_err(|e| format!("Invalid type_pattern: {}", e))?;

        if let Some(ref sp) = rule.source_pattern {
            glob::Pattern::new(sp).map_err(|e| format!("Invalid source_pattern: {}", e))?;
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

        if let Some(tp) = update.type_pattern {
            glob::Pattern::new(&tp).map_err(|e| format!("Invalid type_pattern: {}", e))?;
            rule.type_pattern = tp;
        }
        if let Some(sp) = update.source_pattern {
            if let Some(ref pat) = sp {
                glob::Pattern::new(pat)
                    .map_err(|e| format!("Invalid source_pattern: {}", e))?;
            }
            rule.source_pattern = sp;
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
    pub type_pattern: Option<String>,
    #[serde(default)]
    pub source_pattern: Option<Option<String>>,
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

    #[test]
    fn rule_matches_by_type_pattern() {
        let rule = RoutingRule {
            id: "r1".into(),
            type_pattern: "com.github.*".into(),
            source_pattern: None,
            action: RouteAction::EmitFrontend {
                channel: "test".into(),
            },
            enabled: true,
            created_by: "user".into(),
        };

        assert!(rule.matches(&make_event("com.github.push", "s")));
        assert!(rule.matches(&make_event("com.github.pr", "s")));
        assert!(!rule.matches(&make_event("com.gitlab.push", "s")));
    }

    #[test]
    fn disabled_rule_never_matches() {
        let rule = RoutingRule {
            id: "r2".into(),
            type_pattern: "*".into(),
            source_pattern: None,
            action: RouteAction::EmitFrontend {
                channel: "test".into(),
            },
            enabled: false,
            created_by: "user".into(),
        };

        assert!(!rule.matches(&make_event("anything", "s")));
    }

    #[test]
    fn source_pattern_filters() {
        let rule = RoutingRule {
            id: "r3".into(),
            type_pattern: "*".into(),
            source_pattern: Some("nexus://extension/*".into()),
            action: RouteAction::EmitFrontend {
                channel: "test".into(),
            },
            enabled: true,
            created_by: "user".into(),
        };

        assert!(rule.matches(&make_event("t", "nexus://extension/wh")));
        assert!(!rule.matches(&make_event("t", "nexus://core")));
    }

    #[test]
    fn store_crud_and_persistence() {
        let tmp = TempDir::new().unwrap();
        let mut store = RoutingRuleStore::load(tmp.path());
        assert_eq!(store.list().len(), 0);

        let id = store
            .create(RoutingRule {
                id: String::new(),
                type_pattern: "com.github.*".into(),
                source_pattern: None,
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
                type_pattern: "com.github.*".into(),
                source_pattern: None,
                action: RouteAction::EmitFrontend {
                    channel: "a".into(),
                },
                enabled: true,
                created_by: "user".into(),
            })
            .unwrap();

        store
            .create(RoutingRule {
                id: String::new(),
                type_pattern: "*".into(),
                source_pattern: None,
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
    fn invalid_pattern_rejected() {
        let tmp = TempDir::new().unwrap();
        let mut store = RoutingRuleStore::load(tmp.path());

        let result = store.create(RoutingRule {
            id: String::new(),
            type_pattern: "[invalid".into(),
            source_pattern: None,
            action: RouteAction::EmitFrontend {
                channel: "x".into(),
            },
            enabled: true,
            created_by: "user".into(),
        });
        assert!(result.is_err());
    }
}
