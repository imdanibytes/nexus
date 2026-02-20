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

// ── Meta & Credential types ──────────────────────────────────────

/** Permission entry in the self-introspection response. */
export interface MetaPermission {
  permission: string;
  state: "active" | "revoked" | "deferred";
  scopes?: string[];
}

/** Response from GET /api/v1/meta/self */
export interface MetaSelf {
  plugin_id: string;
  name: string;
  version: string;
  status: string;
  permissions: MetaPermission[];
}

/** Response from GET /api/v1/meta/stats */
export interface MetaStats {
  container_id: string;
  [key: string]: unknown;
}

/** A credential scope entry. */
export interface CredentialScope {
  id: string;
  label?: string;
  description?: string;
}

/** A credential provider in the list response. */
export interface CredentialProviderEntry {
  id: string;
  name: string;
  scopes: CredentialScope[];
}

/** Response from GET /api/v1/meta/credentials */
export interface CredentialProviderList {
  providers: CredentialProviderEntry[];
}

/** Response from POST /api/v1/meta/credentials/{ext_id} */
export interface CredentialResponse {
  provider: string;
  scope: string;
  data: Record<string, unknown>;
  expires_at?: string;
}

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

/** Options for {@link NexusPlugin.init}. */
export interface NexusInitOptions {
  /**
   * Forward `console.log/info/warn/error/debug` calls from the browser to the
   * plugin server, where they appear as Docker stdout/stderr and show up in
   * Nexus's built-in log viewer.
   *
   * Original console methods still fire (browser devtools unaffected).
   *
   * @default true
   */
  console?: boolean;
}

/** Opaque config from the plugin server. Auth details are internal. */
interface ClientConfig {
  token: string;
  apiUrl: string;
}

// ── Console forwarding internals ──────────────────────────────────

type ConsoleLevel = "log" | "info" | "warn" | "error" | "debug";
const CONSOLE_LEVELS: ConsoleLevel[] = ["log", "info", "warn", "error", "debug"];
const MAX_ARG_LENGTH = 10_240; // 10 KB per serialized arg
const FLUSH_INTERVAL = 250; // ms

interface LogEntry {
  level: ConsoleLevel;
  args: string[];
  ts: number;
}

/** Marker to prevent double-patching across multiple init() calls. */
const PATCHED = Symbol.for("nexus:console-patched");

function safeSerialize(value: unknown): string {
  const seen = new WeakSet();

  function inner(v: unknown): unknown {
    if (v === null || v === undefined) return v;
    if (typeof v === "string") return v;
    if (typeof v === "number" || typeof v === "boolean") return v;

    if (v instanceof Error) {
      return v.stack || v.message;
    }

    if (typeof v === "object") {
      if (seen.has(v as object)) return "[Circular]";
      seen.add(v as object);

      if (Array.isArray(v)) {
        return v.map(inner);
      }

      const result: Record<string, unknown> = {};
      for (const key of Object.keys(v as Record<string, unknown>)) {
        result[key] = inner((v as Record<string, unknown>)[key]);
      }
      return result;
    }

    if (typeof v === "function") return `[Function: ${v.name || "anonymous"}]`;
    if (typeof v === "symbol") return v.toString();
    if (typeof v === "bigint") return v.toString();

    return String(v);
  }

  const raw = inner(value);
  const str = typeof raw === "string" ? raw : JSON.stringify(raw);
  return str.length > MAX_ARG_LENGTH ? str.slice(0, MAX_ARG_LENGTH) + "…[truncated]" : str;
}

function patchConsole(): void {
  const g = globalThis as Record<symbol, boolean>;
  if (g[PATCHED]) return;
  g[PATCHED] = true;

  let buffer: LogEntry[] = [];
  let timer: ReturnType<typeof setTimeout> | null = null;

  function flush(): void {
    if (buffer.length === 0) return;
    const entries = buffer;
    buffer = [];
    timer = null;

    fetch("/__nexus/log", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ entries }),
      keepalive: true,
    }).catch(() => {});
  }

  function scheduleFlush(immediate: boolean): void {
    if (immediate) {
      if (timer !== null) {
        clearTimeout(timer);
        timer = null;
      }
      flush();
    } else if (timer === null) {
      timer = setTimeout(flush, FLUSH_INTERVAL);
    }
  }

  for (const level of CONSOLE_LEVELS) {
    const original = console[level];
    console[level] = (...args: unknown[]) => {
      original.apply(console, args);
      buffer.push({
        level,
        args: args.map(safeSerialize),
        ts: Date.now(),
      });
      scheduleFlush(level === "error" || level === "warn");
    };
  }
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
   * Console forwarding is enabled by default — `console.log/info/warn/error/debug`
   * calls are batched and sent to the plugin server, where they appear as Docker
   * stdout/stderr in Nexus's log viewer. Browser devtools still work normally.
   *
   * @param configUrl - Override the config endpoint (defaults to `/api/config`)
   * @param options - SDK options (e.g. `{ console: false }` to disable log forwarding)
   */
  static async init(
    configUrl = "/api/config",
    options?: NexusInitOptions,
  ): Promise<NexusPlugin> {
    if (options?.console !== false) {
      patchConsole();
    }

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

  // ── Meta (self-introspection) ─────────────────────────────

  /** Namespace for plugin metadata endpoints. */
  readonly meta = {
    /** GET /api/v1/meta/self — plugin identity and permissions. */
    self: (): Promise<MetaSelf> =>
      this._withRetry(() =>
        client.get({ url: "/api/v1/meta/self" }) as Promise<{ data?: MetaSelf; error?: unknown; response: Response }>
      ),

    /** GET /api/v1/meta/stats — container resource statistics. */
    stats: (): Promise<MetaStats> =>
      this._withRetry(() =>
        client.get({ url: "/api/v1/meta/stats" }) as Promise<{ data?: MetaStats; error?: unknown; response: Response }>
      ),

    /** GET /api/v1/meta/credentials — list available credential providers. */
    credentials: (): Promise<CredentialProviderList> =>
      this._withRetry(() =>
        client.get({ url: "/api/v1/meta/credentials" }) as Promise<{ data?: CredentialProviderList; error?: unknown; response: Response }>
      ),
  };

  // ── Credentials ────────────────────────────────────────────

  /**
   * Resolve credentials from a provider extension.
   *
   * ```ts
   * const aws = await nexus.credentials("aws-credentials", { scope: "default" });
   * ```
   */
  async credentials(
    provider: string,
    opts: { scope?: string } = {},
  ): Promise<CredentialResponse> {
    return this._withRetry(() =>
      client.post({
        url: "/api/v1/meta/credentials/{ext_id}",
        path: { ext_id: provider },
        body: { scope: opts.scope ?? "default" },
      }) as Promise<{ data?: CredentialResponse; error?: unknown; response: Response }>
    );
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
