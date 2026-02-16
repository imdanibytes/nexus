import { randomUUID } from "node:crypto";
import express from "express";
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StreamableHTTPServerTransport } from "@modelcontextprotocol/sdk/server/streamableHttp.js";
import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { StdioClientTransport } from "@modelcontextprotocol/sdk/client/stdio.js";
import {
  ListToolsRequestSchema,
  CallToolRequestSchema,
  ListResourcesRequestSchema,
  ReadResourceRequestSchema,
  ListPromptsRequestSchema,
  GetPromptRequestSchema,
} from "@modelcontextprotocol/sdk/types.js";

const PORT = 80;
const MCP_SERVER_COMMAND = process.env.MCP_SERVER_COMMAND;

if (!MCP_SERVER_COMMAND) {
  console.error("ERROR: MCP_SERVER_COMMAND environment variable is required");
  process.exit(1);
}

// Parse command string into command + args
function parseCommand(cmd) {
  const parts = cmd.trim().split(/\s+/);
  return { command: parts[0], args: parts.slice(1) };
}

const { command, args } = parseCommand(MCP_SERVER_COMMAND);

// ── Child MCP Client (stdio → child server) ──────────────────

let childClient = null;
let childTransport = null;

async function initChildClient() {
  console.log(`Spawning MCP server: ${command} ${args.join(" ")}`);

  childTransport = new StdioClientTransport({ command, args });
  childClient = new Client(
    { name: "nexus-mcp-bridge", version: "0.1.0" },
    { capabilities: {} },
  );

  await childClient.connect(childTransport);
  console.log("Child MCP client connected");

  const { tools } = await childClient.listTools();
  console.log(`Discovered ${(tools || []).length} tools`);
  for (const tool of tools || []) {
    console.log(`  - ${tool.name}: ${tool.description || "(no description)"}`);
  }
}

// ── Bridge MCP Server (StreamableHTTP → proxy to child) ──────

const transports = new Map();

function createBridgeServer() {
  const server = new McpServer(
    { name: "nexus-mcp-bridge", version: "0.1.0" },
    { capabilities: { tools: {}, resources: {}, prompts: {} } },
  );

  // Dynamic tool listing — forward from child
  server.server.setRequestHandler(
    ListToolsRequestSchema,
    async () => {
      if (!childClient) throw new Error("Child MCP client not initialized");
      return childClient.listTools();
    },
  );

  // Tool calls — forward to child
  server.server.setRequestHandler(
    CallToolRequestSchema,
    async (request) => {
      if (!childClient) throw new Error("Child MCP client not initialized");
      return childClient.callTool(request.params);
    },
  );

  // Resource listing — forward from child (best-effort)
  server.server.setRequestHandler(
    ListResourcesRequestSchema,
    async () => {
      if (!childClient) throw new Error("Child MCP client not initialized");
      try {
        return await childClient.listResources();
      } catch {
        return { resources: [] };
      }
    },
  );

  // Resource reading — forward to child
  server.server.setRequestHandler(
    ReadResourceRequestSchema,
    async (request) => {
      if (!childClient) throw new Error("Child MCP client not initialized");
      return childClient.readResource(request.params);
    },
  );

  // Prompt listing — forward from child (best-effort)
  server.server.setRequestHandler(
    ListPromptsRequestSchema,
    async () => {
      if (!childClient) throw new Error("Child MCP client not initialized");
      try {
        return await childClient.listPrompts();
      } catch {
        return { prompts: [] };
      }
    },
  );

  // Prompt retrieval — forward to child
  server.server.setRequestHandler(
    GetPromptRequestSchema,
    async (request) => {
      if (!childClient) throw new Error("Child MCP client not initialized");
      return childClient.getPrompt(request.params);
    },
  );

  return server;
}

// ── Express App ──────────────────────────────────────────────

const app = express();
app.use(express.json());

// Health check
app.get("/health", (_req, res) => {
  res.json({ status: "ok", child_connected: !!childClient });
});

// MCP endpoint — StreamableHTTP with session management
app.post("/mcp", async (req, res) => {
  const sessionId = req.headers["mcp-session-id"];

  if (sessionId && transports.has(sessionId)) {
    const transport = transports.get(sessionId);
    await transport.handleRequest(req, res, req.body);
    return;
  }

  // New session — create transport + bridge server
  const transport = new StreamableHTTPServerTransport({
    sessionIdGenerator: () => randomUUID(),
    onsessioninitialized: (sid) => {
      transports.set(sid, transport);
    },
  });

  transport.onclose = () => {
    if (transport.sessionId) {
      transports.delete(transport.sessionId);
    }
  };

  const server = createBridgeServer();
  await server.server.connect(transport);
  await transport.handleRequest(req, res, req.body);
});

// GET /mcp for SSE streams (required by streamable HTTP spec)
app.get("/mcp", async (req, res) => {
  const sessionId = req.headers["mcp-session-id"];
  if (sessionId && transports.has(sessionId)) {
    const transport = transports.get(sessionId);
    await transport.handleRequest(req, res);
    return;
  }
  res.status(400).json({ error: "No valid session" });
});

// DELETE /mcp for session cleanup
app.delete("/mcp", async (req, res) => {
  const sessionId = req.headers["mcp-session-id"];
  if (sessionId && transports.has(sessionId)) {
    const transport = transports.get(sessionId);
    await transport.handleRequest(req, res);
    return;
  }
  res.status(400).json({ error: "No valid session" });
});

// ── Lifecycle ────────────────────────────────────────────────

let httpServer;

function shutdown() {
  console.log("Shutting down...");
  if (childTransport) childTransport.close();
  for (const transport of transports.values()) {
    transport.close?.();
  }
  transports.clear();
  if (httpServer) httpServer.close(() => process.exit(0));
  setTimeout(() => process.exit(0), 3000);
}

process.on("SIGTERM", shutdown);
process.on("SIGINT", shutdown);

initChildClient()
  .then(() => {
    httpServer = app.listen(PORT, () => {
      console.log(`MCP bridge listening on port ${PORT}`);
    });
  })
  .catch((err) => {
    console.error("Failed to initialize MCP client:", err);
    process.exit(1);
  });
