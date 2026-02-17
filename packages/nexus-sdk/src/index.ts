/**
 * @imdanibytes/nexus-sdk
 *
 * TypeScript SDK for Nexus plugins. Generated HTTP client from the Host API
 * OpenAPI spec, with a hand-written convenience wrapper.
 *
 * Usage inside a plugin:
 *
 * ```ts
 * import { NexusPlugin } from "@imdanibytes/nexus-sdk";
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

/** Payload shape for host → plugin system events via postMessage. */
export interface NexusHostEvent {
  type: "nexus:system";
  event: string;
  data: unknown;
}

/** Callback for host system events. */
export type NexusEventHandler = (event: string, data: unknown) => void;

// Re-export raw generated SDK for advanced use
export * as sdk from "./client/sdk.gen";
export { client } from "./client/client.gen";

interface PluginConfig {
  token: string;
  apiUrl: string;
  /** OAuth refresh token (enables direct token refresh without re-auth). */
  refreshToken?: string;
  /** OAuth client ID (required for refresh_token grant). */
  clientId?: string;
}

/**
 * Convenience wrapper that fetches an OAuth access token from the
 * plugin's `/api/config` endpoint and configures the HTTP client.
 *
 * The plugin server authenticates via OAuth 2.1 client_credentials grant.
 * The browser only ever sees the access token and optional refresh token.
 *
 * On 401 (token expired), call `refreshToken()` which will use the
 * OAuth refresh_token grant if available, otherwise re-fetch from
 * the plugin server.
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
   * Refresh the access token. Tries OAuth refresh_token grant first
   * (if refresh token and client ID are available), then falls back
   * to re-fetching from the plugin server's `/api/config` endpoint.
   */
  async refreshToken(): Promise<string> {
    // Try OAuth refresh_token grant directly
    if (this.config.refreshToken && this.config.clientId) {
      const origin = this.config.apiUrl.replace(/\/api\/?$/, "");
      const res = await fetch(`${origin}/oauth/token`, {
        method: "POST",
        headers: { "Content-Type": "application/x-www-form-urlencoded" },
        body: new URLSearchParams({
          grant_type: "refresh_token",
          client_id: this.config.clientId,
          refresh_token: this.config.refreshToken,
        }),
      });

      if (res.ok) {
        const data = await res.json();
        this.config.token = data.access_token;
        if (data.refresh_token) {
          this.config.refreshToken = data.refresh_token;
        }
        client.setConfig({
          baseUrl: this.config.apiUrl,
          auth: this.config.token,
        });
        return this.config.token;
      }
    }

    // Fall back to plugin server config endpoint
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

  // ── Host Events ──────────────────────────────────────────

  /**
   * Listen for system events pushed from the Nexus host via postMessage.
   *
   * Events include:
   * - `language_changed` — `{ language: string }`
   *
   * Returns an unsubscribe function.
   *
   * ```ts
   * const off = nexus.onHostEvent((event, data) => {
   *   if (event === "language_changed") {
   *     console.log("New language:", data.language);
   *   }
   * });
   * // later: off();
   * ```
   */
  onHostEvent(handler: NexusEventHandler): () => void {
    const listener = (e: MessageEvent) => {
      const msg = e.data;
      if (msg && typeof msg === "object" && msg.type === "nexus:system") {
        handler(msg.event, msg.data);
      }
    };
    window.addEventListener("message", listener);
    return () => window.removeEventListener("message", listener);
  }
}
