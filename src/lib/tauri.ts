import { invoke } from "@tauri-apps/api/core";
import type { InstalledPlugin, RegistryEntry, RegistrySource } from "../types/plugin";
import type { GrantedPermission, Permission } from "../types/permissions";

export async function pluginList(): Promise<InstalledPlugin[]> {
  return invoke("plugin_list");
}

export async function pluginInstall(
  manifestUrl: string
): Promise<InstalledPlugin> {
  return invoke("plugin_install", { manifestUrl });
}

export async function pluginInstallLocal(
  manifestPath: string
): Promise<InstalledPlugin> {
  return invoke("plugin_install_local", { manifestPath });
}

export async function pluginStart(pluginId: string): Promise<void> {
  return invoke("plugin_start", { pluginId });
}

export async function pluginStop(pluginId: string): Promise<void> {
  return invoke("plugin_stop", { pluginId });
}

export async function pluginRemove(pluginId: string): Promise<void> {
  return invoke("plugin_remove", { pluginId });
}

export async function pluginLogs(
  pluginId: string,
  tail?: number
): Promise<string[]> {
  return invoke("plugin_logs", { pluginId, tail });
}

export async function marketplaceSearch(
  query: string
): Promise<RegistryEntry[]> {
  return invoke("marketplace_search", { query });
}

export async function marketplaceRefresh(): Promise<void> {
  return invoke("marketplace_refresh");
}

export async function permissionGrant(
  pluginId: string,
  permissions: Permission[]
): Promise<void> {
  return invoke("permission_grant", { pluginId, permissions });
}

export async function permissionRevoke(
  pluginId: string,
  permissions: Permission[]
): Promise<void> {
  return invoke("permission_revoke", { pluginId, permissions });
}

export async function permissionList(
  pluginId: string
): Promise<GrantedPermission[]> {
  return invoke("permission_list", { pluginId });
}

export interface AppVersionInfo {
  version: string;
  name: string;
}

export async function appVersion(): Promise<AppVersionInfo> {
  return invoke("app_version");
}

export interface DockerStatus {
  installed: boolean;
  running: boolean;
  version: string | null;
  message: string;
}

export async function checkDocker(): Promise<DockerStatus> {
  return invoke("check_docker");
}

export async function openDockerDesktop(): Promise<void> {
  return invoke("open_docker_desktop");
}

// Registries
export async function registryList(): Promise<RegistrySource[]> {
  return invoke("registry_list");
}

export async function registryAdd(
  name: string,
  kind: string,
  url: string
): Promise<RegistrySource> {
  return invoke("registry_add", { name, kind, url });
}

export async function registryRemove(id: string): Promise<void> {
  return invoke("registry_remove", { id });
}

export async function registryToggle(
  id: string,
  enabled: boolean
): Promise<void> {
  return invoke("registry_toggle", { id, enabled });
}
