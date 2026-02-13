/**
 * @nexus/plugin-sdk
 *
 * TypeScript SDK for Nexus plugins. Auto-generated from the Host API
 * OpenAPI spec, with a convenience wrapper for automatic configuration.
 *
 * Usage inside a plugin:
 *
 * ```ts
 * import { NexusPlugin } from "@nexus/plugin-sdk";
 *
 * const nexus = await NexusPlugin.init();
 * const info = await nexus.systemInfo();
 * const settings = await nexus.getSettings();
 * ```
 */

import { client } from "./client/client.gen";
import {
  systemInfo as _systemInfo,
  readFile as _readFile,
  listDir as _listDir,
  writeFile as _writeFile,
  listProcesses as _listProcesses,
  listContainers as _listContainers,
  containerStats as _containerStats,
  proxyRequest as _proxyRequest,
  getSettings as _getSettings,
  putSettings as _putSettings,
} from "./client/sdk.gen";

export type {
  SystemInfo,
  FileContent,
  DirListing,
  DirEntry,
  ProcessInfo,
  ContainerInfo,
  ProxyRequest,
  ProxyResponse,
  WriteRequest,
} from "./client/types.gen";

// Re-export raw generated SDK for advanced use
export * as sdk from "./client/sdk.gen";
export { client } from "./client/client.gen";

interface PluginConfig {
  token: string;
  apiUrl: string;
}

/**
 * Convenience wrapper that auto-discovers NEXUS_TOKEN and NEXUS_API_URL
 * by calling the plugin's own `/api/config` endpoint, then configures
 * the underlying HTTP client.
 */
export class NexusPlugin {
  private constructor(public readonly config: PluginConfig) {}

  /**
   * Initialize the SDK. Fetches plugin config from the local server and
   * configures the HTTP client with the correct base URL and auth token.
   *
   * @param configUrl - Override the config endpoint (defaults to `/api/config`)
   */
  static async init(configUrl = "/api/config"): Promise<NexusPlugin> {
    const res = await fetch(configUrl);
    if (!res.ok) {
      throw new Error(
        `Failed to fetch plugin config from ${configUrl}: ${res.status}`
      );
    }

    const config: PluginConfig = await res.json();

    client.setConfig({
      baseUrl: config.apiUrl,
      auth: config.token,
    });

    return new NexusPlugin(config);
  }

  /**
   * Configure the SDK manually (for use outside plugin iframes, e.g. tests).
   */
  static configure(apiUrl: string, token: string): NexusPlugin {
    client.setConfig({ baseUrl: apiUrl, auth: token });
    return new NexusPlugin({ token, apiUrl });
  }

  // ── System ──────────────────────────────────────────────

  async systemInfo() {
    const { data } = await _systemInfo();
    return data!;
  }

  // ── Filesystem ──────────────────────────────────────────

  async readFile(path: string) {
    const { data } = await _readFile({ query: { path } });
    return data!;
  }

  async listDir(path: string) {
    const { data } = await _listDir({ query: { path } });
    return data!;
  }

  async writeFile(path: string, content: string) {
    await _writeFile({ body: { path, content } });
  }

  // ── Process ─────────────────────────────────────────────

  async listProcesses() {
    const { data } = await _listProcesses();
    return data!;
  }

  // ── Docker ──────────────────────────────────────────────

  async listContainers() {
    const { data } = await _listContainers();
    return data!;
  }

  async containerStats(id: string) {
    const { data } = await _containerStats({ path: { id } });
    return data!;
  }

  // ── Network ─────────────────────────────────────────────

  async proxyRequest(url: string, method: string, options?: { headers?: Record<string, string>; body?: string }) {
    const { data } = await _proxyRequest({
      body: { url, method, headers: options?.headers ?? {}, body: options?.body },
    });
    return data!;
  }

  // ── Settings ────────────────────────────────────────────

  async getSettings() {
    const { data } = await _getSettings();
    return data!;
  }

  async saveSettings(values: Record<string, unknown>) {
    await _putSettings({ body: values });
  }
}
