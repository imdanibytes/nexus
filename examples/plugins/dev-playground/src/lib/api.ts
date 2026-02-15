// ── Nexus Host API Client ──────────────────────────────────────
// Canonical pattern for plugin → Host API communication.
//
// 1. Fetch a short-lived token from the plugin's own /api/config
// 2. Use that token for all Host API calls
// 3. On 401, refresh the token and retry once

interface Config {
  token: string;
  apiUrl: string;
}

let config: Config | null = null;

async function fetchConfig(): Promise<Config> {
  const res = await fetch("/api/config");
  if (!res.ok) throw new Error(`Config fetch failed: ${res.status}`);
  config = await res.json();
  return config!;
}

async function getConfig(): Promise<Config> {
  if (config) return config;
  return fetchConfig();
}

/** Invalidate the cached token so the next call fetches a fresh one. */
export async function refreshToken(): Promise<void> {
  config = null;
  await fetchConfig();
}

/**
 * Make an authenticated request to the Nexus Host API.
 * Automatically handles token refresh on 401.
 */
export async function api<T = unknown>(
  path: string,
  options: RequestInit = {}
): Promise<T> {
  const cfg = await getConfig();
  const url = `${cfg.apiUrl}${path}`;

  const headers: Record<string, string> = {
    Authorization: `Bearer ${cfg.token}`,
    ...(options.headers as Record<string, string>),
  };

  // Set Content-Type for requests with a body
  if (options.body && !headers["Content-Type"]) {
    headers["Content-Type"] = "application/json";
  }

  let res = await fetch(url, { ...options, headers });

  // Retry once on 401 with a fresh token
  if (res.status === 401) {
    await refreshToken();
    const freshCfg = await getConfig();
    headers.Authorization = `Bearer ${freshCfg.token}`;
    res = await fetch(url, { ...options, headers });
  }

  if (!res.ok) {
    const text = await res.text().catch(() => "");
    throw new Error(`${res.status} ${res.statusText}: ${text}`);
  }

  const contentType = res.headers.get("content-type");
  if (contentType?.includes("application/json")) {
    return res.json();
  }
  return res.text() as unknown as T;
}
