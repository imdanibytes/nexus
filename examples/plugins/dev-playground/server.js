const http = require("http");
const fs = require("fs");
const path = require("path");

const PORT = 80;
const NEXUS_PLUGIN_SECRET = process.env.NEXUS_PLUGIN_SECRET || "";
const NEXUS_API_URL =
  process.env.NEXUS_API_URL || "http://host.docker.internal:9600";
const NEXUS_HOST_URL =
  process.env.NEXUS_HOST_URL || "http://host.docker.internal:9600";

const distDir = path.join(__dirname, "dist");

const MIME_TYPES = {
  ".html": "text/html",
  ".css": "text/css",
  ".js": "application/javascript",
  ".json": "application/json",
  ".png": "image/png",
  ".svg": "image/svg+xml",
  ".woff": "font/woff",
  ".woff2": "font/woff2",
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

// ── Server ─────────────────────────────────────────────────────

const server = http.createServer((req, res) => {
  // Health check
  if (req.url === "/health") {
    res.writeHead(200, { "Content-Type": "application/json" });
    res.end(JSON.stringify({ status: "ok" }));
    return;
  }

  // Config endpoint — serves a short-lived access token to the frontend
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
          case "list_storage_keys": {
            const resp = await fetch(`${NEXUS_HOST_URL}/api/v1/storage`, {
              headers,
            });
            const keys = await resp.json();
            result = {
              content: [{ type: "text", text: JSON.stringify(keys, null, 2) }],
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

  // SPA: serve index.html for root or unknown paths (client-side routing)
  let filePath = req.url === "/" ? "/index.html" : req.url;
  // Strip query params
  filePath = filePath.split("?")[0];
  const fullPath = path.join(distDir, filePath);
  const ext = path.extname(fullPath);
  const contentType = MIME_TYPES[ext] || "application/octet-stream";

  fs.readFile(fullPath, (err, data) => {
    if (err) {
      // SPA fallback: serve index.html for non-asset paths
      if (!ext || ext === ".html") {
        fs.readFile(path.join(distDir, "index.html"), (err2, html) => {
          if (err2) {
            res.writeHead(404, { "Content-Type": "text/plain" });
            res.end("Not Found");
            return;
          }
          res.writeHead(200, { "Content-Type": "text/html" });
          res.end(html);
        });
        return;
      }
      res.writeHead(404, { "Content-Type": "text/plain" });
      res.end("Not Found");
      return;
    }
    res.writeHead(200, { "Content-Type": contentType });
    res.end(data);
  });
});

server.listen(PORT, () => {
  console.log(`Dev Playground plugin running on port ${PORT}`);
});
