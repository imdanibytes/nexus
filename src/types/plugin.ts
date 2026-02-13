export type PluginStatus = "installing" | "running" | "stopped" | "error";

export interface UiConfig {
  port: number;
  path: string;
}

export interface HealthConfig {
  endpoint: string;
  interval_secs: number;
}

export interface SettingDef {
  key: string;
  type: "string" | "number" | "boolean" | "select";
  label: string;
  description?: string;
  default?: unknown;
  options?: string[];
}

export interface PluginManifest {
  id: string;
  name: string;
  version: string;
  description: string;
  author: string;
  license?: string;
  homepage?: string;
  icon?: string;
  image: string;
  ui: UiConfig;
  permissions: string[];
  health?: HealthConfig;
  env: Record<string, string>;
  min_nexus_version?: string;
  settings?: SettingDef[];
}

export interface InstalledPlugin {
  manifest: PluginManifest;
  container_id: string | null;
  status: PluginStatus;
  assigned_port: number;
  auth_token: string;
  installed_at: string;
}

export interface RegistryEntry {
  id: string;
  name: string;
  version: string;
  description: string;
  image: string;
  manifest_url: string;
  categories: string[];
  downloads: number;
  source?: string;
}

export type RegistryKind = "remote" | "local";

export interface RegistrySource {
  id: string;
  name: string;
  kind: RegistryKind;
  url: string;
  enabled: boolean;
}
