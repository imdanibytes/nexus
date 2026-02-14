#!/usr/bin/env node

import { spawn } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import readline from "node:readline";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const VERSION = "0.1.0";

// ── Argument Parsing ────────────────────────────────────────────

const rawArgs = process.argv.slice(2);

function parseFlag(flag) {
  const idx = rawArgs.indexOf(flag);
  if (idx === -1 || idx + 1 >= rawArgs.length) return null;
  return rawArgs[idx + 1];
}

const flags = {
  id: parseFlag("--id"),
  name: parseFlag("--name"),
  description: parseFlag("--description") || parseFlag("--desc"),
  author: parseFlag("--author"),
  yes: rawArgs.includes("-y") || rawArgs.includes("--yes"),
};

// Filter out flags to get the positional MCP command
const positional = rawArgs.filter((a, i) => {
  if (a.startsWith("-")) return false;
  if (i > 0 && rawArgs[i - 1].startsWith("--")) return false;
  return true;
});

if (positional.length === 0 || rawArgs.includes("--help") || rawArgs.includes("-h")) {
  console.log(`
mcp-wrap v${VERSION} — Wrap MCP servers as Nexus headless plugins

Usage:
  mcp-wrap [options] "<mcp-server-command>"

Options:
  --id <id>           Plugin ID (e.g., com.example.weather)
  --name <name>       Display name
  --description <d>   Plugin description
  --author <author>   Author name
  -y, --yes           Accept default permissions without review

Example:
  mcp-wrap "npx -y @modelcontextprotocol/server-everything"
  mcp-wrap --id com.nexus.fs --name "Filesystem" --author "Me" -y "npx -y @modelcontextprotocol/server-filesystem"

Supported runtimes (MVP):
  npx, node

Not yet supported:
  uvx, python, binary paths
`);
  process.exit(0);
}

const mcpCommand = positional[0];

// ── Runtime Detection ───────────────────────────────────────────

function detectRuntime(cmd) {
  const binary = cmd.trim().split(/\s+/)[0];
  if (binary === "npx" || binary === "node") return "node";
  if (binary === "uvx" || binary === "python" || binary === "python3") return "python";
  return "unknown";
}

const runtime = detectRuntime(mcpCommand);
if (runtime !== "node") {
  console.error(
    `\nOnly Node.js-based MCP servers are supported in this MVP.`,
  );
  console.error(`  Your command starts with: ${mcpCommand.split(/\s+/)[0]}`);
  console.error(`  Supported: npx, node`);
  console.error(`  Not yet supported: uvx, python\n`);
  process.exit(1);
}

// ── Tool Discovery ──────────────────────────────────────────────

function parseCommand(cmd) {
  const parts = cmd.trim().split(/\s+/);
  return { command: parts[0], args: parts.slice(1) };
}

async function discoverTools(mcpCmd) {
  const { command, args: cmdArgs } = parseCommand(mcpCmd);
  console.log(`\nSpawning MCP server: ${command} ${cmdArgs.join(" ")}`);

  return new Promise((resolve, reject) => {
    const proc = spawn(command, cmdArgs, {
      stdio: ["pipe", "pipe", "inherit"],
    });

    let buffer = "";
    let tools = null;
    let phase = "init"; // init → tools → done

    const timeout = setTimeout(() => {
      proc.kill();
      reject(new Error("Timed out waiting for MCP server response (10s)"));
    }, 10000);

    proc.stdout.on("data", (chunk) => {
      buffer += chunk.toString();

      // Process complete JSON-RPC messages (newline-delimited)
      let newlineIdx;
      while ((newlineIdx = buffer.indexOf("\n")) !== -1) {
        const line = buffer.slice(0, newlineIdx).trim();
        buffer = buffer.slice(newlineIdx + 1);

        if (!line) continue;

        try {
          const msg = JSON.parse(line);

          if (phase === "init" && msg.id === 1 && msg.result) {
            // Initialize response — send initialized notification then tools/list
            phase = "tools";
            proc.stdin.write(
              JSON.stringify({
                jsonrpc: "2.0",
                method: "notifications/initialized",
              }) + "\n",
            );
            proc.stdin.write(
              JSON.stringify({
                jsonrpc: "2.0",
                id: 2,
                method: "tools/list",
                params: {},
              }) + "\n",
            );
          } else if (phase === "tools" && msg.id === 2 && msg.result) {
            tools = msg.result.tools || [];
            clearTimeout(timeout);
            proc.kill();
            resolve(tools);
          }
        } catch {
          // Ignore non-JSON output (server logs, etc.)
        }
      }
    });

    proc.on("error", (err) => {
      clearTimeout(timeout);
      reject(new Error(`Failed to spawn MCP server: ${err.message}`));
    });

    proc.on("close", (code) => {
      clearTimeout(timeout);
      if (tools === null) {
        reject(
          new Error(
            `MCP server exited (code ${code}) before returning tools`,
          ),
        );
      }
    });

    // Send initialize request
    const initReq = {
      jsonrpc: "2.0",
      id: 1,
      method: "initialize",
      params: {
        protocolVersion: "2024-11-05",
        capabilities: {},
        clientInfo: { name: "mcp-wrap", version: VERSION },
      },
    };
    proc.stdin.write(JSON.stringify(initReq) + "\n");
  });
}

// ── Permission Inference ────────────────────────────────────────

const PERMISSION_PATTERNS = [
  {
    patterns: [/\bfile\b/i, /\bread\b/i, /\bpath\b/i, /\bdirectory\b/i, /\bfolder\b/i, /\bls\b/i, /\blist_dir/i, /\bget_file/i],
    permission: "filesystem:read",
  },
  {
    patterns: [/\bwrite\b/i, /\bsave\b/i, /\bcreate\b/i, /\bdelete\b/i, /\bremove\b/i, /\bmkdir\b/i, /\brename\b/i, /\bmove\b/i],
    permission: "filesystem:write",
  },
  {
    patterns: [/\bfetch\b/i, /\brequest\b/i, /\bhttp\b/i, /\burl\b/i, /\bdownload\b/i, /\bapi\b/i, /\bwebhook\b/i],
    permission: "network:internet",
  },
];

const HIGH_RISK_PATTERNS = [
  /\bexec\b/i, /\brun\b/i, /\bshell\b/i, /\bcommand\b/i, /\beval\b/i,
  /\bspawn\b/i, /\bsystem\b/i, /\bbash\b/i,
];

function classifyTool(tool) {
  const text = `${tool.name} ${tool.description || ""}`;
  const permissions = new Set();
  let highRisk = false;

  for (const { patterns, permission } of PERMISSION_PATTERNS) {
    if (patterns.some((p) => p.test(text))) {
      permissions.add(permission);
    }
  }

  if (HIGH_RISK_PATTERNS.some((p) => p.test(text))) {
    highRisk = true;
  }

  return {
    name: tool.name,
    description: tool.description || "",
    inputSchema: tool.inputSchema || { type: "object", properties: {} },
    permissions: [...permissions],
    requires_approval: true,
    highRisk,
  };
}

// ── Interactive Prompts ─────────────────────────────────────────
// Single readline interface for the entire session to avoid
// closing stdin between prompts.

let rl = null;

function getRL() {
  if (!rl) {
    rl = readline.createInterface({
      input: process.stdin,
      output: process.stdout,
    });
  }
  return rl;
}

function closeRL() {
  if (rl) {
    rl.close();
    rl = null;
  }
}

function ask(q) {
  return new Promise((resolve) => getRL().question(q, resolve));
}

async function reviewTools(classified) {
  console.log(`\nDiscovered ${classified.length} tools:\n`);

  const maxNameLen = Math.max(...classified.map((t) => t.name.length));

  for (let i = 0; i < classified.length; i++) {
    const t = classified[i];
    const num = String(i + 1).padStart(2);
    const name = t.name.padEnd(maxNameLen + 2);
    const perms =
      t.permissions.length > 0
        ? `[${t.permissions.join(", ")}]`
        : "[no permissions]";
    const risk = t.highRisk ? "  !! HIGH RISK" : "";
    console.log(`  ${num}. ${name} ${perms}  requires_approval: true${risk}`);
  }

  if (flags.yes) {
    console.log("\n  --yes flag: accepting defaults");
    return classified;
  }

  const answer = await ask("\nAccept defaults? (Y/n) ");

  if (answer.toLowerCase() === "n") {
    return editTools(classified);
  }
  return classified;
}

async function editTools(classified) {
  console.log(
    "\nEdit mode. For each tool, press Enter to keep defaults or type new values.",
  );
  console.log(
    "Permissions: filesystem:read, filesystem:write, network:internet, network:local, system:info",
  );
  console.log("Type 'skip' to exclude a tool from the plugin.\n");

  const result = [];
  for (const tool of classified) {
    console.log(`  ${tool.name} (${tool.description})`);
    const permsInput = await ask(
      `    permissions [${tool.permissions.join(", ") || "none"}]: `,
    );
    if (permsInput.toLowerCase() === "skip") {
      console.log(`    -> Skipped\n`);
      continue;
    }
    const approvalInput = await ask(
      `    requires_approval [true]: `,
    );

    const newPerms =
      permsInput.trim() === ""
        ? tool.permissions
        : permsInput
            .split(",")
            .map((s) => s.trim())
            .filter(Boolean);
    const newApproval =
      approvalInput.trim() === "" ? true : approvalInput.trim() !== "false";

    result.push({ ...tool, permissions: newPerms, requires_approval: newApproval });
    console.log();
  }

  return result;
}

// ── Plugin Generation ───────────────────────────────────────────

function extractNpmPackage(cmd) {
  // "npx -y @modelcontextprotocol/server-everything" → "@modelcontextprotocol/server-everything"
  // "npx @org/pkg --flag value" → "@org/pkg"
  const parts = cmd.trim().split(/\s+/);
  for (const part of parts.slice(1)) {
    // Skip flags
    if (part.startsWith("-")) continue;
    // This looks like a package name
    if (part.startsWith("@") || /^[a-z]/.test(part)) {
      return part;
    }
  }
  return null;
}

async function promptMetadata() {
  // Use CLI flags if all required fields are provided
  if (flags.id && flags.name) {
    return {
      id: flags.id,
      name: flags.name,
      description: flags.description || "",
      author: flags.author || "",
    };
  }

  console.log("\nPlugin metadata:\n");

  const id = flags.id || await ask("  Plugin ID (e.g., com.example.weather): ");
  const name = flags.name || await ask("  Display name (e.g., Weather Tools): ");
  const description = flags.description || await ask("  Description: ");
  const author = flags.author || await ask("  Author: ");

  return {
    id: id.trim(),
    name: name.trim(),
    description: description.trim(),
    author: author.trim(),
  };
}

function generatePlugin(tools, metadata, mcpCmd) {
  const pluginDir = path.join(process.cwd(), metadata.id);

  if (fs.existsSync(pluginDir)) {
    console.error(`\nERROR: Directory already exists: ${pluginDir}`);
    process.exit(1);
  }

  fs.mkdirSync(path.join(pluginDir, "src"), { recursive: true });

  // Collect plugin-level permissions (union of all tool permissions)
  const allPermissions = [
    ...new Set(tools.flatMap((t) => t.permissions)),
  ];

  // plugin.json — headless manifest
  const manifest = {
    id: metadata.id,
    name: metadata.name,
    version: "0.1.0",
    description: metadata.description,
    author: metadata.author,
    license: "MIT",
    image: `nexus-mcp-${metadata.id.replace(/\./g, "-")}:latest`,
    ui: null,
    permissions: allPermissions,
    health: {
      endpoint: "/health",
      interval_secs: 30,
    },
    env: {
      MCP_SERVER_COMMAND: mcpCmd,
    },
    mcp: {
      tools: tools.map((t) => ({
        name: t.name,
        description: t.description,
        permissions: t.permissions,
        input_schema: t.inputSchema,
        requires_approval: t.requires_approval,
      })),
    },
  };

  fs.writeFileSync(
    path.join(pluginDir, "plugin.json"),
    JSON.stringify(manifest, null, 2) + "\n",
  );

  // package.json — bridge + MCP server as dependencies
  const npmPkg = extractNpmPackage(mcpCmd);
  const deps = {
    "@modelcontextprotocol/sdk": "^1.12.1",
  };
  if (npmPkg) {
    deps[npmPkg] = "*";
  }

  const pkg = {
    name: metadata.id,
    version: "0.1.0",
    description: metadata.description,
    type: "module",
    main: "src/server.js",
    scripts: { start: "node src/server.js" },
    dependencies: deps,
  };

  fs.writeFileSync(
    path.join(pluginDir, "package.json"),
    JSON.stringify(pkg, null, 2) + "\n",
  );

  // src/server.js — copy bridge code
  const bridgeSrc = path.resolve(__dirname, "../../mcp-bridge/src/server.js");
  fs.copyFileSync(bridgeSrc, path.join(pluginDir, "src/server.js"));

  // Dockerfile
  const dockerfile = `FROM node:20-alpine

WORKDIR /app

COPY package.json package-lock.json* ./
RUN npm install --production

COPY src/ ./src/

EXPOSE 80

CMD ["node", "src/server.js"]
`;

  fs.writeFileSync(path.join(pluginDir, "Dockerfile"), dockerfile);

  return pluginDir;
}

// ── Main ────────────────────────────────────────────────────────

async function main() {
  console.log(`mcp-wrap v${VERSION}`);

  // 1. Discover tools
  let rawTools;
  try {
    rawTools = await discoverTools(mcpCommand);
  } catch (err) {
    console.error(`\nFailed to discover tools: ${err.message}`);
    process.exit(1);
  }

  if (rawTools.length === 0) {
    console.error("\nMCP server reported 0 tools. Nothing to wrap.");
    process.exit(1);
  }

  // 2. Classify with permission inference
  const classified = rawTools.map(classifyTool);

  // 3. Interactive review
  const reviewed = await reviewTools(classified);

  if (reviewed.length === 0) {
    console.error("\nAll tools were skipped. Nothing to generate.");
    process.exit(1);
  }

  // 4. Metadata
  const metadata = await promptMetadata();
  if (!metadata.id || !metadata.name) {
    console.error("\nPlugin ID and name are required.");
    process.exit(1);
  }

  // 5. Generate
  closeRL();
  const pluginDir = generatePlugin(reviewed, metadata, mcpCommand);

  console.log(`\nPlugin generated: ${pluginDir}/`);
  console.log(`\n  plugin.json  — ${reviewed.length} MCP tools with permission model`);
  console.log(`  Dockerfile   — Node 20 Alpine`);
  console.log(`  src/server.js — MCP bridge server`);
  console.log(`\nNext steps:`);
  console.log(`  1. Open Nexus`);
  console.log(`  2. Install from local manifest: ${path.join(pluginDir, "plugin.json")}`);
  console.log(`  3. Approve permissions and start the plugin`);
  console.log(`  4. MCP tools will appear in Claude automatically\n`);
}

main().catch((err) => {
  console.error("Fatal:", err);
  process.exit(1);
});
