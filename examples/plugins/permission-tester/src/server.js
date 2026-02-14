const http = require("http");
const fs = require("fs");
const path = require("path");

const PORT = 80;
const NEXUS_PLUGIN_SECRET = process.env.NEXUS_PLUGIN_SECRET || "";
const NEXUS_API_URL =
  process.env.NEXUS_API_URL || "http://host.docker.internal:9600";
const NEXUS_HOST_URL =
  process.env.NEXUS_HOST_URL || "http://host.docker.internal:9600";

const publicDir = path.join(__dirname, "public");

const MIME_TYPES = {
  ".html": "text/html",
  ".css": "text/css",
  ".js": "application/javascript",
  ".json": "application/json",
};

// ── Token Management ───────────────────────────────────────────

let cachedAccessToken = null;
let tokenExpiresAt = 0;

async function getAccessToken() {
  if (cachedAccessToken && Date.now() < tokenExpiresAt - 30000) {
    return cachedAccessToken;
  }

  const res = await fetch(`${NEXUS_HOST_URL}/api/v1/auth/token`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ secret: NEXUS_PLUGIN_SECRET }),
  });

  if (!res.ok) {
    throw new Error(`Token exchange failed: ${res.status}`);
  }

  const data = await res.json();
  cachedAccessToken = data.access_token;
  tokenExpiresAt = Date.now() + data.expires_in * 1000;
  return cachedAccessToken;
}

// ── Security Audit Battery ────────────────────────────────────

async function runSecurityAudit() {
  const token = await getAccessToken();
  const headers = {
    Authorization: `Bearer ${token}`,
    "Content-Type": "application/json",
  };

  let passed = 0;
  let failed = 0;
  const lines = [];

  function section(title) {
    lines.push(`\n## ${title}\n`);
  }

  function record(name, ok, status, detail) {
    if (ok) passed++;
    else failed++;
    lines.push(`${ok ? "PASS" : "FAIL"} ${name} [${status}]${detail ? " — " + detail : ""}`);
  }

  /** Helper: call Host API, return {status, ok, data}. */
  async function api(method, path, body) {
    const url = `${NEXUS_HOST_URL}/api/v1${path}`;
    const opts = { method, headers: { ...headers } };
    if (body && (method === "POST" || method === "PUT")) {
      opts.body = JSON.stringify(body);
    }
    const res = await fetch(url, opts);
    let data;
    try {
      data = await res.json();
    } catch {
      data = await res.text().catch(() => "");
    }
    return { status: res.status, ok: res.ok, data };
  }

  /** Expect a 2xx response (permission should be granted). */
  async function expectAllow(name, method, path, body) {
    try {
      const r = await api(method, path, body);
      record(name, r.ok, r.status);
    } catch (e) {
      record(name, false, "ERR", e.message);
    }
  }

  /** Expect a non-2xx response (should be blocked). */
  async function expectDeny(name, method, path, body) {
    try {
      const r = await api(method, path, body);
      record(name, !r.ok, r.status, r.ok ? "SECURITY BREACH" : "");
    } catch (e) {
      // Network errors count as "blocked"
      record(name, true, "ERR", e.message);
    }
  }

  // ─────────────────────────────────────────────────────────────
  // 1. Authentication
  // ─────────────────────────────────────────────────────────────
  section("Authentication");

  // Valid token works
  await expectAllow("Valid bearer token", "GET", "/system/info");

  // No auth header → 401
  try {
    const r = await fetch(`${NEXUS_HOST_URL}/api/v1/system/info`);
    record("No auth header → 401", r.status === 401, r.status);
  } catch (e) {
    record("No auth header → 401", true, "ERR", e.message);
  }

  // Fake token → 401
  try {
    const r = await fetch(`${NEXUS_HOST_URL}/api/v1/system/info`, {
      headers: { Authorization: "Bearer fake-00000000-0000-0000-0000-000000000000" },
    });
    record("Fake bearer token → 401", r.status === 401, r.status);
  } catch (e) {
    record("Fake bearer token → 401", true, "ERR", e.message);
  }

  // Malformed auth header
  try {
    const r = await fetch(`${NEXUS_HOST_URL}/api/v1/system/info`, {
      headers: { Authorization: "Basic dXNlcjpwYXNz" },
    });
    record("Basic auth header → 401", r.status === 401, r.status);
  } catch (e) {
    record("Basic auth header → 401", true, "ERR", e.message);
  }

  // Token exchange with wrong secret
  try {
    const r = await fetch(`${NEXUS_HOST_URL}/api/v1/auth/token`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ secret: "wrong-secret-value" }),
    });
    record("Token exchange with wrong secret → 401", r.status === 401, r.status);
  } catch (e) {
    record("Token exchange with wrong secret → 401", true, "ERR", e.message);
  }

  // ─────────────────────────────────────────────────────────────
  // 2. Granted Permissions
  // ─────────────────────────────────────────────────────────────
  section("Granted Permissions");

  await expectAllow("system:info → GET /system/info", "GET", "/system/info");
  // fs:read and process:list may require approval — 200 or 403 are both valid
  {
    const r = await api("GET", "/fs/list?path=/tmp");
    const ok = r.status === 200 || r.status === 403;
    record("filesystem:read → GET /fs/list (200 or 403 w/ approval)", ok, r.status);
  }
  {
    const r = await api("GET", "/process/list");
    const ok = r.status === 200 || r.status === 403;
    record("process:list → GET /process/list (200 or 403)", ok, r.status);
  }
  await expectAllow("settings → GET /settings (auth only)", "GET", "/settings");

  // ─────────────────────────────────────────────────────────────
  // 3. Denied Permissions (not in manifest)
  // ─────────────────────────────────────────────────────────────
  section("Denied Permissions");

  await expectDeny("docker:read → GET /docker/containers", "GET", "/docker/containers");
  await expectDeny("docker:read → GET /docker/stats/{id}", "GET", "/docker/stats/some-container-id");

  // ─────────────────────────────────────────────────────────────
  // 4. SSRF Protection — Metadata Endpoints
  // ─────────────────────────────────────────────────────────────
  section("SSRF Protection — Metadata Endpoints");

  await expectDeny(
    "AWS metadata (169.254.169.254)",
    "POST", "/network/proxy",
    { url: "http://169.254.169.254/latest/meta-data/", method: "GET", headers: {} }
  );
  await expectDeny(
    "GCP metadata (metadata.google.internal)",
    "POST", "/network/proxy",
    { url: "http://metadata.google.internal/computeMetadata/v1/", method: "GET", headers: {} }
  );
  await expectDeny(
    "Azure metadata (169.254.169.254 + path)",
    "POST", "/network/proxy",
    { url: "http://169.254.169.254/metadata/instance?api-version=2021-02-01", method: "GET", headers: {} }
  );
  await expectDeny(
    "Alibaba metadata (100.100.100.200)",
    "POST", "/network/proxy",
    { url: "http://100.100.100.200/latest/meta-data/", method: "GET", headers: {} }
  );

  // ─────────────────────────────────────────────────────────────
  // 5. SSRF Protection — Host API Relay
  // ─────────────────────────────────────────────────────────────
  section("SSRF Protection — Host API Relay");

  await expectDeny(
    "Relay via localhost:9600",
    "POST", "/network/proxy",
    { url: "http://localhost:9600/api/v1/system/info", method: "GET", headers: {} }
  );
  await expectDeny(
    "Relay via host.docker.internal:9600",
    "POST", "/network/proxy",
    { url: "http://host.docker.internal:9600/api/v1/system/info", method: "GET", headers: {} }
  );
  await expectDeny(
    "Relay via 127.0.0.1:9600",
    "POST", "/network/proxy",
    { url: "http://127.0.0.1:9600/api/v1/system/info", method: "GET", headers: {} }
  );
  await expectDeny(
    "Relay via [::1]:9600",
    "POST", "/network/proxy",
    { url: "http://[::1]:9600/api/v1/system/info", method: "GET", headers: {} }
  );

  // ─────────────────────────────────────────────────────────────
  // 6. SSRF Protection — Scheme Abuse
  // ─────────────────────────────────────────────────────────────
  section("SSRF Protection — Scheme Abuse");

  await expectDeny(
    "file:// protocol",
    "POST", "/network/proxy",
    { url: "file:///etc/passwd", method: "GET", headers: {} }
  );
  await expectDeny(
    "gopher:// protocol (SSRF amplification)",
    "POST", "/network/proxy",
    { url: "gopher://localhost:6379/_SET%20pwned%20true", method: "GET", headers: {} }
  );
  await expectDeny(
    "ftp:// protocol",
    "POST", "/network/proxy",
    { url: "ftp://localhost/etc/passwd", method: "GET", headers: {} }
  );

  // ─────────────────────────────────────────────────────────────
  // 7. SSRF Protection — IPv6 Bypass
  // ─────────────────────────────────────────────────────────────
  section("SSRF Protection — IPv6 Bypass");

  await expectDeny(
    "IPv6 loopback [::1]:9600 → Host API",
    "POST", "/network/proxy",
    { url: "http://[::1]:9600/api/v1/system/info", method: "GET", headers: {} }
  );
  await expectDeny(
    "IPv6-mapped 127.0.0.1 (::ffff:127.0.0.1):9600",
    "POST", "/network/proxy",
    { url: "http://[::ffff:127.0.0.1]:9600/api/v1/system/info", method: "GET", headers: {} }
  );
  await expectDeny(
    "IPv6-mapped metadata (::ffff:169.254.169.254)",
    "POST", "/network/proxy",
    { url: "http://[::ffff:169.254.169.254]/latest/meta-data/", method: "GET", headers: {} }
  );
  await expectDeny(
    "IPv6 link-local (fe80::1)",
    "POST", "/network/proxy",
    { url: "http://[fe80::1]/", method: "GET", headers: {} }
  );
  await expectDeny(
    "IPv6 unique-local (fc00::1)",
    "POST", "/network/proxy",
    { url: "http://[fc00::1]/", method: "GET", headers: {} }
  );

  // ─────────────────────────────────────────────────────────────
  // 8. SSRF Protection — Internal Service Probing
  // ─────────────────────────────────────────────────────────────
  section("SSRF Protection — Internal Service Probing");

  await expectDeny(
    "Docker socket via localhost:2375",
    "POST", "/network/proxy",
    { url: "http://localhost:2375/containers/json", method: "GET", headers: {} }
  );
  await expectDeny(
    "Docker socket via host.docker.internal:2375",
    "POST", "/network/proxy",
    { url: "http://host.docker.internal:2375/containers/json", method: "GET", headers: {} }
  );
  await expectDeny(
    "Kubernetes API via localhost:6443",
    "POST", "/network/proxy",
    { url: "https://localhost:6443/api/v1/pods", method: "GET", headers: {} }
  );

  // ─────────────────────────────────────────────────────────────
  // 9. Filesystem — Path Traversal
  // ─────────────────────────────────────────────────────────────
  section("Filesystem — Path Traversal");

  await expectDeny(
    "Traversal: /tmp/../../etc/shadow",
    "GET", "/fs/read?path=" + encodeURIComponent("/tmp/../../etc/shadow")
  );
  await expectDeny(
    "Relative path write: ../../../tmp/pwned",
    "POST", "/fs/write",
    { path: "../../../tmp/pwned", content: "escaped sandbox" }
  );
  await expectDeny(
    "Null byte: /etc/passwd\\x00.txt",
    "GET", "/fs/read?path=" + encodeURIComponent("/etc/passwd\x00.txt")
  );
  await expectDeny(
    "Null byte in write path",
    "POST", "/fs/write",
    { path: "/tmp/safe\x00/etc/passwd", content: "null byte test" }
  );

  // ─────────────────────────────────────────────────────────────
  // 10. Filesystem — Data Directory Protection
  // ─────────────────────────────────────────────────────────────
  section("Filesystem — Data Directory Protection");

  // These use ~ which won't resolve in the Rust handler, but that's the
  // point — even if the path resolution fails, it should 403 not 500.
  await expectDeny(
    "Read permissions.json (data dir)",
    "GET", "/fs/read?path=" + encodeURIComponent("~/Library/Application Support/com.nexus.app/permissions.json")
  );
  await expectDeny(
    "Read plugins.json (data dir)",
    "GET", "/fs/read?path=" + encodeURIComponent("~/Library/Application Support/com.nexus.app/plugins.json")
  );
  await expectDeny(
    "Write to data dir: permissions.json",
    "POST", "/fs/write",
    { path: "~/Library/Application Support/com.nexus.app/permissions.json", content: '{"grants":{}}' }
  );

  // ─────────────────────────────────────────────────────────────
  // 11. Filesystem — Special Files & Input Validation
  // ─────────────────────────────────────────────────────────────
  section("Filesystem — Special Files & Input Validation");

  await expectDeny(
    "Device read: /dev/zero (memory exhaustion)",
    "GET", "/fs/read?path=" + encodeURIComponent("/dev/zero")
  );
  await expectDeny(
    "Device read: /dev/urandom",
    "GET", "/fs/read?path=" + encodeURIComponent("/dev/urandom")
  );
  await expectDeny(
    "Extremely long path (100KB)",
    "GET", "/fs/read?path=" + encodeURIComponent("/" + "A".repeat(100000))
  );
  await expectDeny(
    "Command injection: backticks in path",
    "GET", "/fs/read?path=" + encodeURIComponent("/tmp/`id`")
  );
  await expectDeny(
    "Command injection: $() in path",
    "GET", "/fs/read?path=" + encodeURIComponent("/tmp/$(whoami)")
  );
  await expectDeny(
    "CRLF injection in path",
    "GET", "/fs/read?path=" + encodeURIComponent("/tmp/test\r\n/etc/passwd")
  );
  await expectDeny(
    "Unicode fullwidth slash in path",
    "GET", "/fs/read?path=" + encodeURIComponent("/tmp/\uff0f../etc/passwd")
  );

  // ─────────────────────────────────────────────────────────────
  // 12. Settings — Schema Validation
  // ─────────────────────────────────────────────────────────────
  section("Settings — Schema Validation");

  await expectDeny(
    "Write unknown settings key (schema violation)",
    "PUT", "/settings",
    { admin_override: true, secret_key: "injected" }
  );

  // ─────────────────────────────────────────────────────────────
  // 13. TOCTOU — Concurrent Requests
  // ─────────────────────────────────────────────────────────────
  section("Concurrency");

  {
    const toctouPath = "/tmp/nexus-toctou-test-" + Date.now();
    const [r1, r2] = await Promise.all([
      api("GET", "/fs/read?path=" + encodeURIComponent(toctouPath)),
      api("GET", "/fs/read?path=" + encodeURIComponent(toctouPath)),
    ]);
    const bothDenied = !r1.ok && !r2.ok;
    record(
      "TOCTOU: concurrent requests to unapproved path",
      bothDenied, `${r1.status}/${r2.status}`,
      bothDenied ? "" : "one request may have leaked through"
    );
  }

  // ─────────────────────────────────────────────────────────────
  // 14. Extension IPC — Plugin → Extension → IPC → Extension
  // ─────────────────────────────────────────────────────────────
  section("Extension IPC");

  // Direct call to ipc-provider (simple extension call)
  await expectAllow(
    "ipc-provider:list_keys → direct extension call",
    "POST", "/extensions/ipc-provider/list_keys",
    { input: {} }
  );

  await expectAllow(
    "ipc-provider:get_record(alpha) → direct extension call",
    "POST", "/extensions/ipc-provider/get_record",
    { input: { key: "alpha" } }
  );

  // Call ipc-consumer which internally calls ipc-provider via IPC (full chain)
  await expectAllow(
    "ipc-consumer:fetch_record(alpha) → IPC chain test",
    "POST", "/extensions/ipc-consumer/fetch_record",
    { input: { key: "alpha" } }
  );

  await expectAllow(
    "ipc-consumer:fetch_all_keys → IPC chain test",
    "POST", "/extensions/ipc-consumer/fetch_all_keys",
    { input: {} }
  );

  await expectAllow(
    "ipc-consumer:aggregate([alpha,beta]) → multi-IPC chain test",
    "POST", "/extensions/ipc-consumer/aggregate",
    { input: { keys: ["alpha", "beta"] } }
  );

  await expectAllow(
    "ipc-consumer:discover → IPC list_extensions",
    "POST", "/extensions/ipc-consumer/discover",
    { input: {} }
  );

  // Permission boundary: undeclared extension should be denied
  await expectDeny(
    "Undeclared extension → should be 403",
    "POST", "/extensions/nonexistent-ext/some_op",
    { input: {} }
  );

  // Undeclared operation on a declared extension
  await expectDeny(
    "Undeclared operation on ipc-provider → should be denied",
    "POST", "/extensions/ipc-provider/secret_operation",
    { input: {} }
  );

  // ─────────────────────────────────────────────────────────────
  // Summary
  // ─────────────────────────────────────────────────────────────

  const total = passed + failed;
  const allGood = failed === 0;

  lines.push("");
  lines.push("════════════════════════════════════════");
  lines.push(`${allGood ? "ALL CLEAR" : "ISSUES FOUND"}: ${passed}/${total} tests passed`);
  if (!allGood) {
    lines.push(`${failed} test(s) returned 2xx when they should have been blocked.`);
  }

  return {
    content: [{ type: "text", text: lines.join("\n") }],
    is_error: false,
  };
}

// ── Server ─────────────────────────────────────────────────────

const server = http.createServer((req, res) => {
  if (req.url === "/health") {
    res.writeHead(200, { "Content-Type": "application/json" });
    res.end(JSON.stringify({ status: "ok" }));
    return;
  }

  // Config endpoint — serves a short-lived access token (never the secret)
  if (req.url === "/api/config") {
    getAccessToken()
      .then((token) => {
        res.writeHead(200, {
          "Content-Type": "application/json",
          "Access-Control-Allow-Origin": "*",
        });
        res.end(JSON.stringify({ token, apiUrl: NEXUS_API_URL }));
      })
      .catch((err) => {
        res.writeHead(500, { "Content-Type": "application/json" });
        res.end(JSON.stringify({ error: err.message }));
      });
    return;
  }

  // MCP tool call handler
  if (req.method === "POST" && req.url === "/mcp/call") {
    let body = "";
    req.on("data", (chunk) => (body += chunk));
    req.on("end", async () => {
      try {
        const { tool_name } = JSON.parse(body);
        let result;

        switch (tool_name) {
          case "run_security_audit": {
            result = await runSecurityAudit();
            break;
          }
          default:
            result = {
              content: [{ type: "text", text: `Unknown tool: ${tool_name}` }],
              is_error: true,
            };
        }

        res.writeHead(200, { "Content-Type": "application/json" });
        res.end(JSON.stringify(result));
      } catch (err) {
        res.writeHead(200, { "Content-Type": "application/json" });
        res.end(
          JSON.stringify({
            content: [{ type: "text", text: `Error: ${err.message}` }],
            is_error: true,
          })
        );
      }
    });
    return;
  }

  if (req.url === "/" || req.url === "/index.html") {
    const html = fs
      .readFileSync(path.join(publicDir, "index.html"), "utf8")
      .replace(/\{\{NEXUS_API_URL\}\}/g, NEXUS_API_URL);
    res.writeHead(200, { "Content-Type": "text/html" });
    res.end(html);
    return;
  }

  const fullPath = path.join(publicDir, req.url);
  const ext = path.extname(fullPath);
  const contentType = MIME_TYPES[ext] || "application/octet-stream";

  fs.readFile(fullPath, (err, data) => {
    if (err) {
      res.writeHead(404, { "Content-Type": "text/plain" });
      res.end("Not Found");
      return;
    }
    res.writeHead(200, { "Content-Type": contentType });
    res.end(data);
  });
});

server.listen(PORT, () => {
  console.log(`Permission Tester plugin running on port ${PORT}`);
});
