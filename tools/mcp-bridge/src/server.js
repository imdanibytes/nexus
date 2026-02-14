import http from "node:http";
import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { StdioClientTransport } from "@modelcontextprotocol/sdk/client/stdio.js";

const PORT = 80;
const MCP_SERVER_COMMAND = process.env.MCP_SERVER_COMMAND;

if (!MCP_SERVER_COMMAND) {
  console.error("ERROR: MCP_SERVER_COMMAND environment variable is required");
  process.exit(1);
}

// Parse command string into command + args
// "npx -y @modelcontextprotocol/server-weather" → { command: "npx", args: ["-y", ...] }
function parseCommand(cmd) {
  const parts = cmd.trim().split(/\s+/);
  return { command: parts[0], args: parts.slice(1) };
}

const { command, args } = parseCommand(MCP_SERVER_COMMAND);

// ── MCP Client ──────────────────────────────────────────────────

let mcpClient = null;
let transport = null;
let toolsCache = [];

async function initMcpClient() {
  console.log(`Spawning MCP server: ${command} ${args.join(" ")}`);

  transport = new StdioClientTransport({ command, args });

  mcpClient = new Client(
    { name: "nexus-mcp-bridge", version: "0.1.0" },
    { capabilities: {} },
  );

  await mcpClient.connect(transport);
  console.log("MCP client connected");

  const result = await mcpClient.listTools();
  toolsCache = result.tools || [];
  console.log(`Discovered ${toolsCache.length} tools`);
  for (const tool of toolsCache) {
    console.log(`  - ${tool.name}: ${tool.description || "(no description)"}`);
  }
}

// ── HTTP Server ─────────────────────────────────────────────────

function readBody(req) {
  return new Promise((resolve, reject) => {
    let data = "";
    req.on("data", (chunk) => (data += chunk));
    req.on("end", () => resolve(data));
    req.on("error", reject);
  });
}

const server = http.createServer(async (req, res) => {
  // Health check
  if (req.url === "/health") {
    res.writeHead(200, { "Content-Type": "application/json" });
    res.end(JSON.stringify({ status: "ok", tools: toolsCache.length }));
    return;
  }

  // MCP tool call — bridged from Nexus Host API
  if (req.method === "POST" && req.url === "/mcp/call") {
    try {
      const body = JSON.parse(await readBody(req));
      const { tool_name, arguments: toolArgs } = body;

      if (!mcpClient) {
        throw new Error("MCP client not initialized");
      }

      const result = await mcpClient.callTool({
        name: tool_name,
        arguments: toolArgs || {},
      });

      // Translate MCP SDK response → Nexus format (isError → is_error)
      res.writeHead(200, { "Content-Type": "application/json" });
      res.end(
        JSON.stringify({
          content: result.content || [],
          is_error: result.isError || false,
        }),
      );
    } catch (err) {
      console.error("MCP call error:", err);
      res.writeHead(200, { "Content-Type": "application/json" });
      res.end(
        JSON.stringify({
          content: [{ type: "text", text: `Bridge error: ${err.message}` }],
          is_error: true,
        }),
      );
    }
    return;
  }

  res.writeHead(404);
  res.end("Not Found");
});

// ── Lifecycle ───────────────────────────────────────────────────

process.on("SIGTERM", () => {
  console.log("SIGTERM received, shutting down...");
  if (transport) transport.close();
  server.close(() => process.exit(0));
});

process.on("SIGINT", () => {
  console.log("SIGINT received, shutting down...");
  if (transport) transport.close();
  server.close(() => process.exit(0));
});

initMcpClient()
  .then(() => {
    server.listen(PORT, () => {
      console.log(`MCP bridge listening on port ${PORT}`);
    });
  })
  .catch((err) => {
    console.error("Failed to initialize MCP client:", err);
    process.exit(1);
  });
