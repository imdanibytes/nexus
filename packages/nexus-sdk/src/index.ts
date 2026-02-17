/**
 * @imdanibytes/nexus-sdk
 *
 * TypeScript SDK for Nexus plugins. Generated HTTP client from the Host API
 * OpenAPI spec, with a hand-written convenience wrapper.
 *
 * Usage inside a plugin UI (browser):
 *
 * ```ts
 * import { NexusPlugin } from "@imdanibytes/nexus-sdk";
 *
 * const nexus = await NexusPlugin.init();
 * const info = await nexus.systemInfo();
 * const settings = await nexus.getSettings();
 * const result = await nexus.callExtension("my-ext", "my-op", { key: "val" });
 * ```
 *
 * Auth is fully transparent — the SDK handles token acquisition, refresh,
 * and retry automatically. Plugins never touch credentials.
 */

import { client } from "./client/client.gen";
import {
  systemInfo as _systemInfo,
  readFile as _readFile,
  listDir as _listDir,
  writeFile as _writeFile,
  listProcesses as _listProcesses,
  listAllContainers as _listAllContainers,
  containerStats as _containerStats,
  proxyRequest as _proxyRequest,
  getSettings as _getSettings,
  putSettings as _putSettings,
  listExtensions as _listExtensions,
  callExtension as _callExtension,
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
  CallExtensionRequest,
  CallExtensionResponse,
  ListExtensionsResponse,
  PluginExtensionView,
  PluginOperationView,
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

/** Opaque config from the plugin server. Auth details are internal. */
interface ClientConfig {
  token: string;
  apiUrl: string;
}

/**
 * Browser-side SDK for Nexus plugin UIs.
 *
 * Auth is fully transparent: the plugin server handles credential management,
 * and this class automatically retries on 401 by re-fetching config.
 * Plugins never see tokens, secrets, or auth protocol details.
 */
export class NexusPlugin {
  private configUrl: string;
  private token: string;
  private apiUrl: string;

  private constructor(config: ClientConfig, configUrl: string) {
    this.token = config.token;
    this.apiUrl = config.apiUrl;
    this.configUrl = configUrl;
  }

  /**
   * Initialize the SDK. Fetches config from the plugin server and
   * configures the HTTP client. Auth is handled automatically.
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

    const config: ClientConfig = await res.json();

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

  // ── Internal: transparent auth retry ──────────────────────

  /** Re-fetch config from the plugin server (which handles credential refresh). */
  private async _refreshConfig(): Promise<void> {
    if (!this.configUrl) {
      throw new Error("Cannot refresh in manual configuration mode");
    }

    const res = await fetch(this.configUrl);
    if (!res.ok) {
      throw new Error(`Failed to refresh config: ${res.status}`);
    }

    const config: ClientConfig = await res.json();
    this.token = config.token;
    this.apiUrl = config.apiUrl;

    client.setConfig({
      baseUrl: config.apiUrl,
      auth: config.token,
    });
  }

  /** Execute a generated SDK call with automatic 401 retry. */
  private async _withRetry<T>(
    fn: () => Promise<{ data?: T; error?: unknown; response: Response }>
  ): Promise<T> {
    let result = await fn();

    if (result.response.status === 401) {
      await this._refreshConfig();
      result = await fn();
    }

    if (result.data === undefined) {
      throw new Error(`Request failed: ${result.response.status}`);
    }

    return result.data;
  }

  // ── System ──────────────────────────────────────────────

  async systemInfo() {
    return this._withRetry(() => _systemInfo());
  }

  // ── Filesystem ──────────────────────────────────────────

  async readFile(path: string) {
    return this._withRetry(() => _readFile({ query: { path } }));
  }

  async listDir(path: string) {
    return this._withRetry(() => _listDir({ query: { path } }));
  }

  async writeFile(path: string, content: string) {
    return this._withRetry(() => _writeFile({ body: { path, content } }));
  }

  // ── Process ─────────────────────────────────────────────

  async listProcesses() {
    return this._withRetry(() => _listProcesses());
  }

  // ── Containers ────────────────────────────────────────────

  async listContainers() {
    return this._withRetry(() => _listAllContainers());
  }

  async containerStats(id: string) {
    return this._withRetry(() => _containerStats({ path: { id } }));
  }

  // ── Network ─────────────────────────────────────────────

  async proxyRequest(url: string, method: string, options?: { headers?: Record<string, string>; body?: string }) {
    return this._withRetry(() => _proxyRequest({
      body: { url, method, headers: options?.headers ?? {}, body: options?.body },
    }));
  }

  // ── Settings ────────────────────────────────────────────

  async getSettings() {
    return this._withRetry(() => _getSettings());
  }

  async saveSettings(values: Record<string, unknown>) {
    return this._withRetry(() => _putSettings({ body: values }));
  }

  // ── Extensions ──────────────────────────────────────────

  /**
   * Call an extension operation.
   *
   * ```ts
   * const result = await nexus.callExtension("my-ext", "my-op", { key: "val" });
   * ```
   */
  async callExtension(extensionId: string, operation: string, input: Record<string, unknown> = {}) {
    return this._withRetry(() => _callExtension({
      path: { ext_id: extensionId, operation },
      body: { input },
    }));
  }

  /** List extensions available to this plugin. */
  async listExtensions() {
    return this._withRetry(() => _listExtensions());
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
