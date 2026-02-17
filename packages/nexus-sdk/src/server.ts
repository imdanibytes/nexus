/**
 * @imdanibytes/nexus-sdk/server
 *
 * Server-side SDK for Nexus plugins. Handles auth transparently —
 * plugins never touch credentials or know the auth protocol.
 *
 * Usage inside a plugin server (Node.js):
 *
 * ```ts
 * import { NexusServer } from "@imdanibytes/nexus-sdk/server";
 *
 * const nexus = new NexusServer();
 * const settings = await nexus.getSettings();
 * const file = await nexus.readFile("/some/path");
 * const result = await nexus.callExtension("my-ext", "my-op", { key: "val" });
 * ```
 *
 * Reads NEXUS_OAUTH_CLIENT_ID, NEXUS_OAUTH_CLIENT_SECRET, NEXUS_HOST_URL,
 * and NEXUS_API_URL from environment variables automatically.
 */

export interface NexusServerOptions {
  /** Override NEXUS_OAUTH_CLIENT_ID env var. */
  clientId?: string;
  /** Override NEXUS_OAUTH_CLIENT_SECRET env var. */
  clientSecret?: string;
  /** Override NEXUS_HOST_URL env var (used for auth endpoint). */
  hostUrl?: string;
  /** Override NEXUS_API_URL env var (used as base for API calls). */
  apiUrl?: string;
  /** Milliseconds before expiry to trigger proactive refresh. Default: 30000. */
  refreshBuffer?: number;
}

interface TokenResponse {
  access_token: string;
  token_type: string;
  expires_in: number;
  refresh_token?: string;
}

export class NexusServer {
  private clientId: string;
  private clientSecret: string;
  private hostUrl: string;
  readonly apiUrl: string;
  private refreshBuffer: number;

  private accessToken: string | null = null;
  private refreshToken: string | null = null;
  private expiresAt = 0;

  /** Prevent concurrent token requests. */
  private pendingAuth: Promise<string> | null = null;

  constructor(options?: NexusServerOptions) {
    this.clientId =
      options?.clientId || process.env.NEXUS_OAUTH_CLIENT_ID || "";
    this.clientSecret =
      options?.clientSecret || process.env.NEXUS_OAUTH_CLIENT_SECRET || "";
    this.hostUrl =
      options?.hostUrl ||
      process.env.NEXUS_HOST_URL ||
      "http://host.docker.internal:9600";
    this.apiUrl =
      options?.apiUrl ||
      process.env.NEXUS_API_URL ||
      "http://host.docker.internal:9600";
    this.refreshBuffer = options?.refreshBuffer ?? 30_000;
  }

  // ── Auth (internal) ───────────────────────────────────────

  /** Get a valid access token. Safe to call concurrently. */
  async getAccessToken(): Promise<string> {
    if (this.accessToken && Date.now() < this.expiresAt - this.refreshBuffer) {
      return this.accessToken;
    }

    if (this.pendingAuth) return this.pendingAuth;

    this.pendingAuth = this._acquireToken().finally(() => {
      this.pendingAuth = null;
    });

    return this.pendingAuth;
  }

  /**
   * Opaque config for the browser SDK. Returns whatever the browser needs
   * to make authenticated calls — no auth protocol details leak.
   */
  getClientConfig(): { token: string; apiUrl: string } {
    return {
      token: this.accessToken || "",
      apiUrl: this.apiUrl,
    };
  }

  // ── Authenticated fetch ───────────────────────────────────

  /**
   * Fetch with automatic auth. Relative paths resolve against `apiUrl`.
   * Low-level escape hatch for endpoints not yet wrapped as typed methods.
   */
  async fetch(path: string, init?: RequestInit): Promise<Response> {
    const token = await this.getAccessToken();
    const url = path.startsWith("http") ? path : `${this.apiUrl}${path}`;
    const headers = new Headers(init?.headers);
    headers.set("Authorization", `Bearer ${token}`);
    return globalThis.fetch(url, { ...init, headers });
  }

  // ── Typed API methods ─────────────────────────────────────

  /** GET /api/v1/system/info */
  async systemInfo(): Promise<Record<string, unknown>> {
    return this._get("/api/v1/system/info");
  }

  /** GET /api/v1/settings — returns plugin settings as key-value map. */
  async getSettings(): Promise<Record<string, unknown>> {
    return this._get("/api/v1/settings");
  }

  /** PUT /api/v1/settings — update plugin settings. */
  async saveSettings(values: Record<string, unknown>): Promise<void> {
    const res = await this.fetch("/api/v1/settings", {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(values),
    });
    if (!res.ok) {
      throw new Error(`saveSettings failed: ${res.status}`);
    }
  }

  /** GET /api/v1/fs/read?path=... */
  async readFile(path: string): Promise<{ path: string; content: string }> {
    return this._get(`/api/v1/fs/read?path=${encodeURIComponent(path)}`);
  }

  /** GET /api/v1/fs/list?path=... */
  async listDir(path: string): Promise<{ path: string; entries: unknown[] }> {
    return this._get(`/api/v1/fs/list?path=${encodeURIComponent(path)}`);
  }

  /** POST /api/v1/fs/write */
  async writeFile(path: string, content: string): Promise<void> {
    const res = await this.fetch("/api/v1/fs/write", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ path, content }),
    });
    if (!res.ok) {
      throw new Error(`writeFile failed: ${res.status}`);
    }
  }

  /** GET /api/v1/process/list */
  async listProcesses(): Promise<unknown[]> {
    return this._get("/api/v1/process/list");
  }

  /** GET /api/v1/docker/containers */
  async listContainers(): Promise<unknown[]> {
    return this._get("/api/v1/docker/containers");
  }

  /** POST /api/v1/network/proxy */
  async proxyRequest(
    url: string,
    method: string,
    options?: { headers?: Record<string, string>; body?: string }
  ): Promise<{ status: number; headers: Record<string, string>; body: string }> {
    const res = await this.fetch("/api/v1/network/proxy", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        url,
        method,
        headers: options?.headers ?? {},
        body: options?.body,
      }),
    });
    if (!res.ok) {
      throw new Error(`proxyRequest failed: ${res.status}`);
    }
    return res.json();
  }

  // ── Extensions ────────────────────────────────────────────

  /**
   * Call an extension operation.
   *
   * ```ts
   * const result = await nexus.callExtension("my-ext", "my-operation", {
   *   key: "value",
   * });
   * ```
   */
  async callExtension(
    extensionId: string,
    operation: string,
    input: Record<string, unknown> = {},
  ): Promise<{ success: boolean; data: unknown; message?: string }> {
    const res = await this.fetch(
      `/api/v1/extensions/${encodeURIComponent(extensionId)}/${encodeURIComponent(operation)}`,
      {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ input }),
      },
    );
    if (!res.ok) {
      const body = await res.text().catch(() => "");
      throw new Error(`callExtension(${extensionId}/${operation}) failed: ${res.status} ${body}`);
    }
    return res.json();
  }

  /** List extensions available to this plugin. */
  async listExtensions(): Promise<unknown[]> {
    return this._get("/api/v1/extensions");
  }

  // ── Internal helpers ──────────────────────────────────────

  private async _get<T>(path: string): Promise<T> {
    const res = await this.fetch(path);
    if (!res.ok) {
      throw new Error(`GET ${path} failed: ${res.status}`);
    }
    return res.json() as Promise<T>;
  }

  private async _acquireToken(): Promise<string> {
    if (this.refreshToken) {
      try {
        const token = await this._refreshGrant();
        if (token) return token;
      } catch {
        this.refreshToken = null;
      }
    }

    return this._clientCredentialsGrant();
  }

  private async _clientCredentialsGrant(): Promise<string> {
    const res = await globalThis.fetch(`${this.hostUrl}/oauth/token`, {
      method: "POST",
      headers: { "Content-Type": "application/x-www-form-urlencoded" },
      body: new URLSearchParams({
        grant_type: "client_credentials",
        client_id: this.clientId,
        client_secret: this.clientSecret,
      }),
    });

    if (!res.ok) {
      const body = await res.text().catch(() => "");
      throw new Error(`Auth failed: ${res.status} ${body}`);
    }

    return this._handleTokenResponse(await res.json());
  }

  private async _refreshGrant(): Promise<string | null> {
    const res = await globalThis.fetch(`${this.hostUrl}/oauth/token`, {
      method: "POST",
      headers: { "Content-Type": "application/x-www-form-urlencoded" },
      body: new URLSearchParams({
        grant_type: "refresh_token",
        client_id: this.clientId,
        refresh_token: this.refreshToken!,
      }),
    });

    if (!res.ok) return null;

    return this._handleTokenResponse(await res.json());
  }

  private _handleTokenResponse(data: TokenResponse): string {
    this.accessToken = data.access_token;
    if (data.refresh_token) {
      this.refreshToken = data.refresh_token;
    }
    this.expiresAt = Date.now() + data.expires_in * 1000;
    return this.accessToken;
  }
}
