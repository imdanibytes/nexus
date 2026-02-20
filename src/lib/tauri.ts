import { invoke } from "@tauri-apps/api/core";
import type { InstalledPlugin, PluginManifest, RegistryEntry, RegistrySource } from "../types/plugin";
import type { ApprovalDecision, GrantedPermission, Permission } from "../types/permissions";
import type { McpSettings, McpToolStatus } from "../types/mcp";
import type { AvailableUpdate } from "../types/updates";
import type { ClassifiedTool, PluginMetadata } from "../types/mcp_wrap";

export async function pluginList(): Promise<InstalledPlugin[]> {
  return invoke("plugin_list");
}

export async function pluginPreviewRemote(
  manifestUrl: string
): Promise<PluginManifest> {
  return invoke("plugin_preview_remote", { manifestUrl });
}

export async function pluginPreviewLocal(
  manifestPath: string
): Promise<PluginManifest> {
  return invoke("plugin_preview_local", { manifestPath });
}

export async function pluginInstall(
  manifestUrl: string,
  approvedPermissions: string[],
  deferredPermissions?: string[],
  buildContext?: string
): Promise<InstalledPlugin> {
  return invoke("plugin_install", {
    manifestUrl,
    approvedPermissions,
    deferredPermissions: deferredPermissions ?? [],
    buildContext: buildContext ?? null,
  });
}

export async function pluginInstallLocal(
  manifestPath: string,
  approvedPermissions: string[],
  deferredPermissions?: string[]
): Promise<InstalledPlugin> {
  return invoke("plugin_install_local", {
    manifestPath,
    approvedPermissions,
    deferredPermissions: deferredPermissions ?? [],
  });
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

export async function pluginSyncStatus(): Promise<InstalledPlugin[]> {
  return invoke("plugin_sync_status");
}

export async function checkImageAvailable(image: string): Promise<boolean> {
  return invoke("check_image_available", { image });
}

export async function checkUrlReachable(url: string): Promise<boolean> {
  return invoke("check_url_reachable", { url });
}

export async function pluginLogs(
  pluginId: string,
  tail?: number
): Promise<string[]> {
  return invoke("plugin_logs", { pluginId, tail });
}

export async function pluginGetSettings(
  pluginId: string
): Promise<Record<string, unknown>> {
  return invoke("plugin_get_settings", { pluginId });
}

export async function pluginSaveSettings(
  pluginId: string,
  values: Record<string, unknown>
): Promise<void> {
  return invoke("plugin_save_settings", { pluginId, values });
}

export async function marketplaceSearch(
  query: string
): Promise<RegistryEntry[]> {
  return invoke("marketplace_search", { query });
}

export async function marketplaceRefresh(): Promise<void> {
  return invoke("marketplace_refresh");
}

export async function marketplaceLoad(): Promise<void> {
  return invoke("marketplace_load");
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

export async function permissionUnrevoke(
  pluginId: string,
  permissions: Permission[]
): Promise<void> {
  return invoke("permission_unrevoke", { pluginId, permissions });
}

export async function permissionList(
  pluginId: string
): Promise<GrantedPermission[]> {
  return invoke("permission_list", { pluginId });
}

export async function permissionRemovePath(
  pluginId: string,
  permission: Permission,
  path: string
): Promise<void> {
  return invoke("permission_remove_path", { pluginId, permission, path });
}

export async function runtimeApprovalRespond(
  requestId: string,
  decision: ApprovalDecision,
  pluginId: string,
  category: string,
  context: Record<string, string>
): Promise<void> {
  return invoke("runtime_approval_respond", {
    requestId,
    decision,
    pluginId,
    category,
    context,
  });
}

export interface AppVersionInfo {
  version: string;
  name: string;
  commit: string | null;
}

export async function appVersion(): Promise<AppVersionInfo> {
  return invoke("app_version");
}

export interface EngineStatus {
  engine_id: string;
  installed: boolean;
  running: boolean;
  version: string | null;
  socket: string;
  message: string;
}

export async function checkEngine(): Promise<EngineStatus> {
  return invoke("check_engine");
}

// Resources

export interface ResourceUsage {
  cpu_percent: number;
  memory_mb: number;
}

export async function containerResourceUsage(): Promise<ResourceUsage> {
  return invoke("container_resource_usage");
}

export interface ResourceQuotas {
  cpu_percent: number | null;
  memory_mb: number | null;
}

export async function getResourceQuotas(): Promise<ResourceQuotas> {
  return invoke("get_resource_quotas");
}

export async function saveResourceQuotas(
  cpuPercent: number | null,
  memoryMb: number | null
): Promise<void> {
  return invoke("save_resource_quotas", {
    cpu_percent: cpuPercent,
    memory_mb: memoryMb,
  });
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

// Extensions

import type {
  ExtensionManifest,
  ExtensionRegistryEntry,
  ExtensionStatus,
  InstalledExtension,
} from "../types/extension";

export async function extensionList(): Promise<ExtensionStatus[]> {
  return invoke("extension_list");
}

export async function extensionInstall(
  manifestUrl: string
): Promise<InstalledExtension> {
  return invoke("extension_install", { manifestUrl });
}

export async function extensionInstallLocal(
  manifestPath: string
): Promise<InstalledExtension> {
  return invoke("extension_install_local", { manifestPath });
}

export async function extensionEnable(extId: string): Promise<void> {
  return invoke("extension_enable", { extId });
}

export async function extensionDisable(extId: string): Promise<void> {
  return invoke("extension_disable", { extId });
}

export async function extensionRemove(extId: string): Promise<void> {
  return invoke("extension_remove", { extId });
}

export async function extensionPreview(
  manifestUrl: string
): Promise<ExtensionManifest> {
  return invoke("extension_preview", { manifestUrl });
}

export async function extensionMarketplaceSearch(
  query: string
): Promise<ExtensionRegistryEntry[]> {
  return invoke("extension_marketplace_search", { query });
}

export async function permissionRemoveScope(
  pluginId: string,
  permission: Permission,
  scope: string
): Promise<void> {
  return invoke("permission_remove_scope", { pluginId, permission, scope });
}

// MCP Gateway

export async function mcpGetSettings(): Promise<McpSettings> {
  return invoke("mcp_get_settings");
}

export async function mcpSetEnabled(
  scope: string,
  enabled: boolean
): Promise<void> {
  return invoke("mcp_set_enabled", { scope, enabled });
}

export async function mcpListTools(): Promise<McpToolStatus[]> {
  return invoke("mcp_list_tools");
}

export async function mcpConfigSnippet(): Promise<Record<string, unknown>> {
  return invoke("mcp_config_snippet");
}

// Updates

export async function checkUpdates(): Promise<AvailableUpdate[]> {
  return invoke("check_updates");
}

export async function getCachedUpdates(): Promise<AvailableUpdate[]> {
  return invoke("get_cached_updates");
}

export async function dismissUpdate(itemId: string, version: string): Promise<void> {
  return invoke("dismiss_update", { itemId, version });
}

export async function updatePlugin(
  manifestUrl: string,
  expectedDigest: string | null,
  buildContext?: string | null
): Promise<InstalledPlugin> {
  return invoke("update_plugin", {
    manifestUrl,
    expectedDigest,
    buildContext: buildContext ?? null,
  });
}

export async function updateExtension(
  manifestUrl: string
): Promise<InstalledExtension> {
  return invoke("update_extension", { manifestUrl });
}

export async function updateExtensionForceKey(
  manifestUrl: string
): Promise<InstalledExtension> {
  return invoke("update_extension_force_key", { manifestUrl });
}

export async function lastUpdateCheck(): Promise<string | null> {
  return invoke("last_update_check");
}

export async function setLanguage(language: string): Promise<void> {
  return invoke("set_language", { language });
}

export async function setTheme(theme: string): Promise<void> {
  return invoke("set_theme", { theme });
}

export async function getUpdateCheckInterval(): Promise<number> {
  return invoke("get_update_check_interval");
}

export async function setUpdateCheckInterval(minutes: number): Promise<void> {
  return invoke("set_update_check_interval", { minutes });
}

// Plugin storage

export async function pluginStorageInfo(pluginId: string): Promise<number> {
  return invoke("plugin_storage_info", { pluginId });
}

export async function pluginClearStorage(pluginId: string): Promise<void> {
  return invoke("plugin_clear_storage", { pluginId });
}

export async function pluginDevModeToggle(
  pluginId: string,
  enabled: boolean
): Promise<void> {
  return invoke("plugin_dev_mode_toggle", { pluginId, enabled });
}

export async function pluginRebuild(pluginId: string): Promise<void> {
  return invoke("plugin_rebuild", { pluginId });
}

// App Updater

export interface AppUpdateInfo {
  version: string;
  body: string | null;
  date: string | null;
}

export async function checkAppUpdate(): Promise<AppUpdateInfo | null> {
  return invoke("check_app_update");
}

export async function downloadAppUpdate(): Promise<void> {
  return invoke("download_app_update");
}

export async function getUpdateChannel(): Promise<string> {
  return invoke("get_update_channel");
}

export async function setUpdateChannel(channel: string): Promise<void> {
  return invoke("set_update_channel", { channel });
}

// OAuth

import type { OAuthClientInfo } from "../types/oauth";

export async function oauthListClients(): Promise<OAuthClientInfo[]> {
  return invoke("oauth_list_clients");
}

export async function oauthRevokeClient(clientId: string): Promise<void> {
  return invoke("oauth_revoke_client", { clientId });
}

// API Keys

export interface ApiKey {
  id: string;
  name: string;
  key_hash: string;
  prefix: string;
  created_at: string;
  last_used_at: string | null;
}

export interface GeneratedApiKey {
  key: ApiKey;
  raw: string;
}

export async function apiKeyList(): Promise<ApiKey[]> {
  return invoke("api_key_list");
}

export async function apiKeyGenerate(name: string): Promise<GeneratedApiKey> {
  return invoke("api_key_generate", { name });
}

export async function apiKeyRevoke(id: string): Promise<void> {
  return invoke("api_key_revoke", { id });
}

export async function apiKeyGetDefault(): Promise<string | null> {
  return invoke("api_key_get_default");
}

export async function apiKeyRegenerateDefault(): Promise<GeneratedApiKey> {
  return invoke("api_key_regenerate_default");
}

// Extension Resources

export async function extensionResourceList(
  extId: string,
  resourceType: string,
  params?: { page?: number; page_size?: number; sort_by?: string; sort_order?: string }
): Promise<{ items: Record<string, unknown>[]; total: number }> {
  return invoke("extension_resource_list", { extId, resourceType, params });
}

export async function extensionResourceGet(
  extId: string,
  resourceType: string,
  resourceId: string
): Promise<Record<string, unknown>> {
  return invoke("extension_resource_get", { extId, resourceType, resourceId });
}

export async function extensionResourceCreate(
  extId: string,
  resourceType: string,
  data: Record<string, unknown>
): Promise<Record<string, unknown>> {
  return invoke("extension_resource_create", { extId, resourceType, data });
}

export async function extensionResourceUpdate(
  extId: string,
  resourceType: string,
  resourceId: string,
  data: Record<string, unknown>
): Promise<Record<string, unknown>> {
  return invoke("extension_resource_update", { extId, resourceType, resourceId, data });
}

export async function extensionResourceDelete(
  extId: string,
  resourceType: string,
  resourceId: string
): Promise<Record<string, unknown>> {
  return invoke("extension_resource_delete", { extId, resourceType, resourceId });
}

// MCP Wrap

export async function mcpDiscoverTools(
  command: string
): Promise<ClassifiedTool[]> {
  return invoke("mcp_discover_tools", { command });
}

export async function mcpSuggestMetadata(
  command: string
): Promise<PluginMetadata> {
  return invoke("mcp_suggest_metadata", { command });
}

export async function mcpGenerateAndInstall(
  command: string,
  tools: ClassifiedTool[],
  metadata: PluginMetadata,
  approvedPermissions: Permission[],
  deferredPermissions: Permission[]
): Promise<InstalledPlugin> {
  return invoke("mcp_generate_and_install", {
    command,
    tools,
    metadata,
    approvedPermissions,
    deferredPermissions,
  });
}
