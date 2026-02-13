# Nexus

A plugin-based desktop dashboard for macOS. Plugins are Docker containers that serve their own UI, communicate with the host through a REST API, and expose tools to AI assistants via the Model Context Protocol (MCP).

## How It Works

Nexus is a Tauri 2 app with a React frontend and a Rust backend. Plugins are Docker images that get pulled, started, and managed by the host. Each plugin container:

- Serves a web UI on an assigned port (rendered in the Nexus shell)
- Authenticates to the Host API with a per-plugin bearer token
- Declares permissions for what it can access (filesystem, network, processes, etc.)
- Optionally exposes MCP tools that AI assistants like Claude can call

A bundled MCP sidecar (`nexus-mcp`) bridges plugin tools to any MCP-compatible client. When plugins start or stop, tool availability updates automatically via SSE.

## Architecture

```
Claude Desktop / AI Client
        |
        | MCP (stdio)
        v
   nexus-mcp sidecar
        |
        | HTTP (localhost:9600)
        v
   Nexus Host API (Axum)
        |
        | Docker API
        v
  Plugin Containers
  [hello-world] [permission-tester] [your-plugin]
```

**Frontend:** React 19, TypeScript, Tailwind CSS, Zustand
**Backend:** Rust, Tauri 2, Axum, Bollard (Docker)
**MCP Gateway:** Rust, rmcp
**Plugin SDK:** Auto-generated TypeScript client from OpenAPI spec

## Getting Started

### Prerequisites

- macOS (Apple Silicon or Intel)
- [Docker Desktop](https://www.docker.com/products/docker-desktop/) running
- [Rust](https://rustup.rs/) (stable)
- [Node.js](https://nodejs.org/) 22+ with [pnpm](https://pnpm.io/)

### Development

```bash
# Install frontend dependencies
pnpm install

# Run in development mode (starts Vite + Tauri)
cargo tauri dev
```

The sidecar is built automatically via `beforeBuildCommand`. To build it manually:

```bash
pnpm sidecar
```

### Production Build

```bash
cargo tauri build
```

This produces a signed `.dmg` and `.app` bundle in `src-tauri/target/release/bundle/`.

## Writing a Plugin

Plugins are Docker containers with a `plugin.json` manifest. See `examples/plugins/hello-world/` for a complete example.

### Manifest (`plugin.json`)

```json
{
  "id": "com.yourname.my-plugin",
  "name": "My Plugin",
  "version": "0.1.0",
  "description": "What it does",
  "image": "my-plugin:latest",
  "ui": { "port": 80, "path": "/" },
  "permissions": ["system:info"],
  "health": { "endpoint": "/health", "interval_secs": 30 },
  "mcp": {
    "tools": [
      {
        "name": "do_something",
        "description": "Does something useful",
        "permissions": ["system:info"],
        "input_schema": { "type": "object", "properties": {} }
      }
    ]
  }
}
```

### Host API

Plugins exchange their secret (`NEXUS_PLUGIN_SECRET`) for a short-lived access token via `POST /api/v1/auth/token`, then use it as a Bearer token to call the Host API:

| Endpoint | Description |
|---|---|
| `GET /api/v1/system/info` | Host OS, hostname, uptime, CPU, memory |
| `GET /api/v1/fs/read?path=...` | Read a file |
| `GET /api/v1/fs/list?path=...` | List a directory |
| `POST /api/v1/fs/write` | Write a file |
| `GET /api/v1/process/list` | Running processes |
| `GET /api/v1/docker/containers` | Docker containers |
| `POST /api/v1/network/proxy` | Proxy an HTTP request |
| `GET /api/v1/settings` | Plugin-scoped settings |
| `PUT /api/v1/settings` | Update settings |

Full OpenAPI spec available at `GET /api/openapi.json` when the app is running.

### Plugin SDK

An auto-generated TypeScript client is published to GitHub Packages:

```bash
npm install @imdanibytes/plugin-sdk --registry=https://npm.pkg.github.com
```

## Project Structure

```
src/                    React frontend
src-tauri/              Rust backend (Tauri shell, Host API, plugin manager)
crates/mcp-sidecar/     MCP gateway sidecar
packages/plugin-sdk/    Auto-generated TypeScript SDK
examples/plugins/       Example plugins (hello-world, permission-tester)
examples/extensions/    Test extension binaries
scripts/                Build & release scripts
docs/                   Architecture docs, roadmap, release checklist
```

## Permissions

Plugins declare required permissions in their manifest. Users approve permissions at install time. Available scopes:

`system:info` `filesystem:read` `filesystem:write` `process:list` `network:proxy` `docker:read`

Runtime approval dialogs appear when a plugin accesses a resource for the first time.

## Release

Releases are automated via GitHub Actions. Pushing a `v*` tag triggers:

1. **build** - Compiles the app for both `aarch64-apple-darwin` and `x86_64-apple-darwin`
2. **build-sdk** - Validates the plugin SDK compiles
3. **publish** - Creates a GitHub Release with signed DMGs and publishes the SDK to GitHub Packages

The app includes an auto-updater that checks for new releases on startup.
