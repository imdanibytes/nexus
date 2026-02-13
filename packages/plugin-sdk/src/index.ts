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
 * Convenience wrapper that fetches a short-lived access token from the
 * plugin's `/api/config` endpoint and configures the HTTP client.
 *
 * The plugin server handles the secret-to-token exchange. The browser
 * only ever sees the short-lived access token.
 *
 * On 401 (token expired), call `refreshToken()` to re-fetch from
 * the plugin server, which will exchange the secret for a new token.
 */
export class NexusPlugin {
  private configUrl: string;

  private constructor(
    public config: PluginConfig,
    configUrl: string,
  ) {
    this.configUrl = configUrl;
  }

  /**
   * Initialize the SDK. Fetches plugin config from the local server and
   * configures the HTTP client with the correct base URL and access token.
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

    return new NexusPlugin(config, configUrl);
  }

  /**
   * Configure the SDK manually (for use outside plugin iframes, e.g. tests).
   */
  static configure(apiUrl: string, token: string): NexusPlugin {
    client.setConfig({ baseUrl: apiUrl, auth: token });
    return new NexusPlugin({ token, apiUrl }, "");
  }

  /**
   * Re-fetch the access token from the plugin server. Call this on 401
   * responses — the server will exchange the secret for a fresh token.
   */
  async refreshToken(): Promise<string> {
    if (!this.configUrl) {
      throw new Error("Cannot refresh token in manual configuration mode");
    }

    const res = await fetch(this.configUrl);
    if (!res.ok) {
      throw new Error(`Failed to refresh token: ${res.status}`);
    }

    const config: PluginConfig = await res.json();
    this.config = config;

    client.setConfig({
      baseUrl: config.apiUrl,
      auth: config.token,
    });

    return config.token;
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
