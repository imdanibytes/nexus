import type { InstalledPlugin } from "./plugin";
import type { ExtensionStatus } from "./extension";

// Plugin lifecycle
interface PluginStarting {
  kind: "plugin:starting";
  plugin_id: string;
}

interface PluginStarted {
  kind: "plugin:started";
  plugin: InstalledPlugin;
}

interface PluginStopping {
  kind: "plugin:stopping";
  plugin_id: string;
}

interface PluginStopped {
  kind: "plugin:stopped";
  plugin: InstalledPlugin;
}

interface PluginRemoving {
  kind: "plugin:removing";
  plugin_id: string;
}

interface PluginRemoved {
  kind: "plugin:removed";
  plugin_id: string;
}

interface PluginInstalling {
  kind: "plugin:installing";
  message: string;
}

interface PluginInstalled {
  kind: "plugin:installed";
  plugin: InstalledPlugin;
}

interface PluginError {
  kind: "plugin:error";
  plugin_id: string;
  action: string;
  message: string;
}

interface PluginUpdateStage {
  kind: "plugin:update_stage";
  plugin_id: string;
  stage: string;
}

interface PluginRebuild {
  kind: "plugin:rebuild";
  plugin_id: string;
  status: string;
  message: string;
}

// Extension lifecycle
interface ExtensionEnabling {
  kind: "extension:enabling";
  ext_id: string;
}

interface ExtensionEnabled {
  kind: "extension:enabled";
  extension: ExtensionStatus;
}

interface ExtensionDisabling {
  kind: "extension:disabling";
  ext_id: string;
}

interface ExtensionDisabled {
  kind: "extension:disabled";
  extension: ExtensionStatus;
}

interface ExtensionRemoving {
  kind: "extension:removing";
  ext_id: string;
}

interface ExtensionRemoved {
  kind: "extension:removed";
  ext_id: string;
}

interface ExtensionInstalling {
  kind: "extension:installing";
  ext_id: string;
}

interface ExtensionInstalled {
  kind: "extension:installed";
  extension: ExtensionStatus;
}

interface ExtensionError {
  kind: "extension:error";
  ext_id: string;
  action: string;
  message: string;
}

export type LifecycleEvent =
  | PluginStarting
  | PluginStarted
  | PluginStopping
  | PluginStopped
  | PluginRemoving
  | PluginRemoved
  | PluginInstalling
  | PluginInstalled
  | PluginError
  | PluginUpdateStage
  | PluginRebuild
  | ExtensionEnabling
  | ExtensionEnabled
  | ExtensionDisabling
  | ExtensionDisabled
  | ExtensionRemoving
  | ExtensionRemoved
  | ExtensionInstalling
  | ExtensionInstalled
  | ExtensionError;
