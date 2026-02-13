const http = require("http");
const fs = require("fs");
const path = require("path");

const PORT = 80;
const NEXUS_TOKEN = process.env.NEXUS_TOKEN || "";
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

const server = http.createServer((req, res) => {
  // Health check
  if (req.url === "/health") {
    res.writeHead(200, { "Content-Type": "application/json" });
    res.end(JSON.stringify({ status: "ok" }));
    return;
  }

  // Token endpoint — lets the frontend JS retrieve the auth token
  if (req.url === "/api/config") {
    res.writeHead(200, {
      "Content-Type": "application/json",
      "Access-Control-Allow-Origin": "*",
    });
    res.end(
      JSON.stringify({
        token: NEXUS_TOKEN,
        apiUrl: NEXUS_API_URL,
      })
    );
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
          case "get_system_info": {
            const resp = await fetch(`${NEXUS_HOST_URL}/api/v1/system/info`, {
              headers: { Authorization: `Bearer ${NEXUS_TOKEN}` },
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
              headers: { Authorization: `Bearer ${NEXUS_TOKEN}` },
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
