import { describe, it, before, after } from "node:test";
import assert from "node:assert/strict";
import http from "node:http";
import { NexusServer } from "../src/server.js";

/** Tiny mock OAuth server that issues predictable tokens. */
function createMockServer() {
  let callCount = 0;
  let lastGrantType = "";

  const server = http.createServer((req, res) => {
    if (req.url === "/oauth/token" && req.method === "POST") {
      let body = "";
      req.on("data", (c) => (body += c));
      req.on("end", () => {
        callCount++;
        const params = new URLSearchParams(body);
        lastGrantType = params.get("grant_type") || "";

        if (lastGrantType === "client_credentials") {
          const clientId = params.get("client_id");
          const clientSecret = params.get("client_secret");
          if (clientId !== "test-id" || clientSecret !== "test-secret") {
            res.writeHead(401, { "Content-Type": "application/json" });
            res.end(JSON.stringify({ error: "invalid_client" }));
            return;
          }
        }

        if (lastGrantType === "refresh_token") {
          const rt = params.get("refresh_token");
          if (rt === "expired-rt") {
            res.writeHead(400, { "Content-Type": "application/json" });
            res.end(JSON.stringify({ error: "invalid_grant" }));
            return;
          }
        }

        res.writeHead(200, { "Content-Type": "application/json" });
        res.end(
          JSON.stringify({
            access_token: `access-${callCount}`,
            token_type: "Bearer",
            expires_in: 3600,
            refresh_token: `refresh-${callCount}`,
          })
        );
      });
      return;
    }

    // Echo endpoint for testing fetch wrapper
    if (req.url?.startsWith("/api/v1/")) {
      const auth = req.headers.authorization || "";
      res.writeHead(200, { "Content-Type": "application/json" });
      res.end(JSON.stringify({ echo: true, path: req.url, auth }));
      return;
    }

    res.writeHead(404);
    res.end();
  });

  return {
    server,
    getCallCount: () => callCount,
    getLastGrantType: () => lastGrantType,
    reset: () => {
      callCount = 0;
      lastGrantType = "";
    },
  };
}

describe("NexusServer", () => {
  let mock: ReturnType<typeof createMockServer>;
  let port: number;
  let baseUrl: string;

  before(async () => {
    mock = createMockServer();
    await new Promise<void>((resolve) => {
      mock.server.listen(0, "127.0.0.1", () => {
        const addr = mock.server.address() as { port: number };
        port = addr.port;
        baseUrl = `http://127.0.0.1:${port}`;
        resolve();
      });
    });
  });

  after(() => {
    mock.server.close();
  });

  it("authenticates via client_credentials on first call", async () => {
    mock.reset();
    const nexus = new NexusServer({
      clientId: "test-id",
      clientSecret: "test-secret",
      hostUrl: baseUrl,
      apiUrl: baseUrl,
    });

    const token = await nexus.getAccessToken();
    assert.equal(token, "access-1");
    assert.equal(mock.getCallCount(), 1);
    assert.equal(mock.getLastGrantType(), "client_credentials");
  });

  it("returns cached token on subsequent calls", async () => {
    mock.reset();
    const nexus = new NexusServer({
      clientId: "test-id",
      clientSecret: "test-secret",
      hostUrl: baseUrl,
      apiUrl: baseUrl,
    });

    await nexus.getAccessToken();
    await nexus.getAccessToken();
    await nexus.getAccessToken();
    assert.equal(mock.getCallCount(), 1, "should only hit the server once");
  });

  it("deduplicates concurrent token requests", async () => {
    mock.reset();
    const nexus = new NexusServer({
      clientId: "test-id",
      clientSecret: "test-secret",
      hostUrl: baseUrl,
      apiUrl: baseUrl,
    });

    const [t1, t2, t3] = await Promise.all([
      nexus.getAccessToken(),
      nexus.getAccessToken(),
      nexus.getAccessToken(),
    ]);

    assert.equal(t1, t2);
    assert.equal(t2, t3);
    assert.equal(mock.getCallCount(), 1, "concurrent calls should deduplicate");
  });

  it("fetch() adds Bearer auth header", async () => {
    mock.reset();
    const nexus = new NexusServer({
      clientId: "test-id",
      clientSecret: "test-secret",
      hostUrl: baseUrl,
      apiUrl: baseUrl,
    });

    const res = await nexus.fetch("/api/v1/settings");
    const body = await res.json() as { auth: string };
    assert.equal(body.auth, "Bearer access-1");
  });

  it("getClientConfig() returns opaque config", async () => {
    mock.reset();
    const nexus = new NexusServer({
      clientId: "test-id",
      clientSecret: "test-secret",
      hostUrl: baseUrl,
      apiUrl: baseUrl,
    });

    await nexus.getAccessToken();
    const config = nexus.getClientConfig();

    assert.equal(config.token, "access-1");
    assert.equal(config.apiUrl, baseUrl);
    // Must NOT contain secrets or auth protocol details
    assert.equal("clientId" in config, false);
    assert.equal("clientSecret" in config, false);
    assert.equal("refreshToken" in config, false);
  });

  it("typed getSettings() works", async () => {
    mock.reset();
    const nexus = new NexusServer({
      clientId: "test-id",
      clientSecret: "test-secret",
      hostUrl: baseUrl,
      apiUrl: baseUrl,
    });

    const result = await nexus.getSettings();
    assert.equal((result as Record<string, unknown>).echo, true);
    assert.equal((result as Record<string, unknown>).path, "/api/v1/settings");
  });

  it("throws on bad credentials", async () => {
    mock.reset();
    const nexus = new NexusServer({
      clientId: "wrong",
      clientSecret: "wrong",
      hostUrl: baseUrl,
      apiUrl: baseUrl,
    });

    await assert.rejects(
      () => nexus.getAccessToken(),
      (err: Error) => err.message.includes("Auth failed"),
    );
  });
});
