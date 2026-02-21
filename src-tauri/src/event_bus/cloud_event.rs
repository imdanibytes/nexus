use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// CNCF CloudEvents v1.0 compliant event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudEvent {
    pub specversion: String,
    pub id: String,
    pub source: String,
    #[serde(rename = "type")]
    pub event_type: String,
    pub time: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    #[serde(default = "default_content_type")]
    pub datacontenttype: String,
    #[serde(default)]
    pub data: Value,
    /// Extension attributes (flattened in serialization).
    #[serde(default, flatten)]
    pub extensions: HashMap<String, Value>,
}

fn default_content_type() -> String {
    "application/json".to_string()
}

impl CloudEvent {
    pub fn builder() -> CloudEventBuilder {
        CloudEventBuilder::default()
    }

    /// Look up a CloudEvents context attribute by name.
    /// Returns `None` for unknown attribute names or unset optional attributes.
    pub fn get_attr(&self, name: &str) -> Option<&str> {
        match name {
            "id" => Some(&self.id),
            "type" => Some(&self.event_type),
            "source" => Some(&self.source),
            "subject" => self.subject.as_deref(),
            "specversion" => Some(&self.specversion),
            "datacontenttype" => Some(&self.datacontenttype),
            _ => self.extensions.get(name).and_then(|v| v.as_str()),
        }
    }

    /// Validate that all required CloudEvents v1.0 fields are present and valid.
    pub fn validate(&self) -> Result<(), String> {
        if self.specversion != "1.0" {
            return Err(format!(
                "specversion must be \"1.0\", got \"{}\"",
                self.specversion
            ));
        }
        if self.id.is_empty() {
            return Err("id must not be empty".into());
        }
        if self.source.is_empty() {
            return Err("source must not be empty".into());
        }
        if self.event_type.is_empty() {
            return Err("type must not be empty".into());
        }
        Ok(())
    }
}

/// Builder for constructing CloudEvents with sensible defaults.
#[derive(Default)]
pub struct CloudEventBuilder {
    source: Option<String>,
    event_type: Option<String>,
    subject: Option<String>,
    data: Option<Value>,
    datacontenttype: Option<String>,
    extensions: HashMap<String, Value>,
}

impl CloudEventBuilder {
    pub fn source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    pub fn event_type(mut self, event_type: impl Into<String>) -> Self {
        self.event_type = Some(event_type.into());
        self
    }

    pub fn subject(mut self, subject: impl Into<String>) -> Self {
        self.subject = Some(subject.into());
        self
    }

    pub fn data(mut self, data: Value) -> Self {
        self.data = Some(data);
        self
    }

    pub fn datacontenttype(mut self, ct: impl Into<String>) -> Self {
        self.datacontenttype = Some(ct.into());
        self
    }

    pub fn extension(mut self, key: impl Into<String>, value: Value) -> Self {
        self.extensions.insert(key.into(), value);
        self
    }

    pub fn build(self) -> Result<CloudEvent, String> {
        let source = self.source.ok_or("source is required")?;
        let event_type = self.event_type.ok_or("type is required")?;

        Ok(CloudEvent {
            specversion: "1.0".to_string(),
            id: Uuid::new_v4().to_string(),
            source,
            event_type,
            time: Utc::now(),
            subject: self.subject,
            datacontenttype: self.datacontenttype.unwrap_or_else(default_content_type),
            data: self.data.unwrap_or(Value::Null),
            extensions: self.extensions,
        })
    }
}

/// Lightweight publish request from extensions — the host fills in the rest.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct PublishRequest {
    #[serde(rename = "type")]
    pub event_type: String,
    #[serde(default)]
    pub data: Value,
    #[serde(default)]
    pub subject: Option<String>,
    #[serde(default)]
    pub datacontenttype: Option<String>,
    #[serde(default)]
    pub extensions: Option<HashMap<String, Value>>,
}

impl PublishRequest {
    /// Convert into a full CloudEvent, filling in host-managed fields.
    pub fn into_cloud_event(self, source: String) -> CloudEvent {
        CloudEvent {
            specversion: "1.0".to_string(),
            id: Uuid::new_v4().to_string(),
            source,
            event_type: self.event_type,
            time: Utc::now(),
            subject: self.subject,
            datacontenttype: self.datacontenttype.unwrap_or_else(default_content_type),
            data: self.data,
            extensions: self.extensions.unwrap_or_default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_creates_valid_event() {
        let event = CloudEvent::builder()
            .source("nexus://core")
            .event_type("nexus.test.ping")
            .data(serde_json::json!({"hello": "world"}))
            .build()
            .unwrap();

        assert_eq!(event.specversion, "1.0");
        assert!(!event.id.is_empty());
        assert_eq!(event.source, "nexus://core");
        assert_eq!(event.event_type, "nexus.test.ping");
        assert_eq!(event.datacontenttype, "application/json");
        assert!(event.validate().is_ok());
    }

    #[test]
    fn builder_requires_source_and_type() {
        assert!(CloudEvent::builder().event_type("test").build().is_err());
        assert!(CloudEvent::builder().source("test").build().is_err());
    }

    #[test]
    fn validation_catches_empty_fields() {
        let mut event = CloudEvent::builder()
            .source("nexus://core")
            .event_type("test")
            .build()
            .unwrap();

        event.specversion = "2.0".to_string();
        assert!(event.validate().is_err());

        event.specversion = "1.0".to_string();
        event.source = String::new();
        assert!(event.validate().is_err());
    }

    #[test]
    fn publish_request_fills_host_fields() {
        let req = PublishRequest {
            event_type: "com.github.push".to_string(),
            data: serde_json::json!({"ref": "refs/heads/main"}),
            subject: Some("wh_abc".to_string()),
            datacontenttype: None,
            extensions: None,
        };

        let event = req.into_cloud_event("nexus://extension/webhook-receiver".to_string());
        assert_eq!(event.specversion, "1.0");
        assert_eq!(event.source, "nexus://extension/webhook-receiver");
        assert_eq!(event.event_type, "com.github.push");
        assert_eq!(event.subject.as_deref(), Some("wh_abc"));
    }

    #[test]
    fn serde_roundtrip() {
        let event = CloudEvent::builder()
            .source("nexus://core")
            .event_type("nexus.test")
            .subject("sub1")
            .data(serde_json::json!({"k": "v"}))
            .extension("traceid", serde_json::json!("abc123"))
            .build()
            .unwrap();

        let json = serde_json::to_string(&event).unwrap();
        let parsed: CloudEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.event_type, "nexus.test");
        assert_eq!(parsed.extensions.get("traceid").unwrap(), "abc123");
    }
}
