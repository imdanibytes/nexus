const http = require("http");
const fs = require("fs");
const path = require("path");

const PORT = 80;
const NEXUS_PLUGIN_SECRET = process.env.NEXUS_PLUGIN_SECRET || "";
const NEXUS_API_URL =
  process.env.NEXUS_API_URL || "http://host.docker.internal:9600";
// Server-side URL for code running inside the Docker container
const NEXUS_HOST_URL =
  process.env.NEXUS_HOST_URL || "http://host.docker.internal:9600";

const publicDir = path.join(__dirname, "public");

const MIME_TYPES = {
  ".html": "text/html",
  ".css": "text/css",
  ".js": "application/javascript",
  ".json": "application/json",
  ".png": "image/png",
  ".svg": "image/svg+xml",
};

// ── Token Management ───────────────────────────────────────────
// Exchange the plugin secret for a short-lived access token.
// The secret never leaves server-side code.

let cachedAccessToken = null;
let tokenExpiresAt = 0;

async function getAccessToken() {
  // Return cached token if still valid (with 30s buffer)
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

// ── Server ─────────────────────────────────────────────────────

const server = http.createServer((req, res) => {
  // Health check
  if (req.url === "/health") {
    res.writeHead(200, { "Content-Type": "application/json" });
    res.end(JSON.stringify({ status: "ok" }));
    return;
  }

  // Config endpoint — serves a short-lived access token to the frontend.
  // The plugin secret is NEVER exposed here.
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
        const token = await getAccessToken();
        const headers = { Authorization: `Bearer ${token}` };

        switch (tool_name) {
          case "get_system_info": {
            const resp = await fetch(`${NEXUS_HOST_URL}/api/v1/system/info`, {
              headers,
            });
            const info = await resp.json();
            result = {
              content: [{ type: "text", text: JSON.stringify(info, null, 2) }],
              is_error: false,
            };
            break;
          }
          case "get_greeting": {
            const resp = await fetch(`${NEXUS_HOST_URL}/api/v1/settings`, {
              headers,
            });
            const settings = await resp.json();
            const greeting = settings.greeting_text || "Hello";
            result = {
              content: [{ type: "text", text: greeting }],
              is_error: false,
            };
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

  // Serve index.html with NEXUS_API_URL templated in
  if (req.url === "/" || req.url === "/index.html") {
    const html = fs
      .readFileSync(path.join(publicDir, "index.html"), "utf8")
      .replace(/\{\{NEXUS_API_URL\}\}/g, NEXUS_API_URL);
    res.writeHead(200, { "Content-Type": "text/html" });
    res.end(html);
    return;
  }

  // Serve other static files
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
  console.log(`Hello World plugin running on port ${PORT}`);
});
