use crate::commands::extensions::ExtensionStatus;
use crate::event_bus::cloud_event::CloudEvent;
use crate::event_bus::SharedEventBus;
use crate::plugin_manager::storage::InstalledPlugin;
use serde::Serialize;
use tauri::{Emitter, Manager};

pub const LIFECYCLE_CHANNEL: &str = "nexus://lifecycle";

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind")]
pub enum LifecycleEvent {
    // -- Plugin lifecycle --
    #[serde(rename = "plugin:starting")]
    PluginStarting { plugin_id: String },

    #[serde(rename = "plugin:started")]
    PluginStarted { plugin: InstalledPlugin },

    #[serde(rename = "plugin:stopping")]
    PluginStopping { plugin_id: String },

    #[serde(rename = "plugin:stopped")]
    PluginStopped { plugin: InstalledPlugin },

    #[serde(rename = "plugin:removing")]
    PluginRemoving { plugin_id: String },

    #[serde(rename = "plugin:removed")]
    PluginRemoved { plugin_id: String },

    #[serde(rename = "plugin:installing")]
    PluginInstalling { message: String },

    #[serde(rename = "plugin:installed")]
    PluginInstalled { plugin: InstalledPlugin },

    #[serde(rename = "plugin:error")]
    PluginError {
        plugin_id: String,
        action: String,
        message: String,
    },

    // -- Plugin update stages (replaces nexus://plugin-update) --
    #[serde(rename = "plugin:update_stage")]
    PluginUpdateStage { plugin_id: String, stage: String },

    // -- Plugin dev rebuild (replaces nexus://dev-rebuild) --
    #[serde(rename = "plugin:rebuild")]
    PluginRebuild {
        plugin_id: String,
        status: String,
        message: String,
    },

    // -- Extension lifecycle --
    #[serde(rename = "extension:enabling")]
    ExtensionEnabling { ext_id: String },

    #[serde(rename = "extension:enabled")]
    ExtensionEnabled { extension: ExtensionStatus },

    #[serde(rename = "extension:disabling")]
    ExtensionDisabling { ext_id: String },

    #[serde(rename = "extension:disabled")]
    ExtensionDisabled { extension: ExtensionStatus },

    #[serde(rename = "extension:removing")]
    ExtensionRemoving { ext_id: String },

    #[serde(rename = "extension:removed")]
    ExtensionRemoved { ext_id: String },

    #[serde(rename = "extension:installing")]
    ExtensionInstalling { ext_id: String },

    #[serde(rename = "extension:installed")]
    ExtensionInstalled { extension: ExtensionStatus },

    #[serde(rename = "extension:error")]
    ExtensionError {
        ext_id: String,
        action: String,
        message: String,
    },
}

pub fn emit(app: Option<&tauri::AppHandle>, event: LifecycleEvent) {
    if let Some(app) = app {
        let _ = app.emit(LIFECYCLE_CHANNEL, &event);

        // Bridge lifecycle events onto the CloudEvents bus.
        // Serialize the event to extract the `kind` tag and full data payload.
        if let Ok(data) = serde_json::to_value(&event) {
            let kind = data
                .get("kind")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let event_type = format!(
                "nexus.lifecycle.{}",
                kind.replace(':', ".")
            );

            let cloud_event = CloudEvent::builder()
                .source("nexus://core")
                .event_type(&event_type)
                .data(data)
                .build();

            if let Ok(ce) = cloud_event {
                if let Some(bus) = app.try_state::<SharedEventBus>() {
                    let bus: SharedEventBus = bus.inner().clone();
                    tauri::async_runtime::spawn(async move {
                        let mut b = bus.write().await;
                        b.publish(ce);
                    });
                }
            }
        }
    }
}
