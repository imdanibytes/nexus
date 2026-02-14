# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Dev Commands

All commands use `just` (see `justfile` at root). Run `just` to list them.

```bash
just dev              # Run app in dev mode (Vite HMR + Tauri)
just check            # Fast Rust compile check (no codegen)
just clippy           # Rust lints
just typecheck        # TypeScript type check (tsc --noEmit)
just lint             # All lints: clippy + tsc + eslint
just build-sidecar    # Build the MCP sidecar binary
just build-app        # Signed production build (.app + .dmg), reads .env.local for signing creds
just build-all        # Sidecar + extensions + app
```

Tests:
```bash
cd src-tauri && cargo test                    # All Rust tests (unit + integration)
cd src-tauri && cargo test test_name          # Single test by name
pnpm lint                                     # ESLint on frontend
```

SDK regeneration (after changing Host API endpoints):
```bash
pnpm sdk              # Export OpenAPI spec → generate TS client → compile
```

Version sync (all Cargo.toml + package.json + tauri.conf.json):
```bash
just sync-version 0.7.0
```

## Architecture

### Overview

Tauri v2 desktop app. Rust backend serves an Axum HTTP server (Host API) on `127.0.0.1:9600`. React frontend communicates with Rust via Tauri IPC commands. Plugins are Docker containers. Extensions are native binaries communicating via JSON-RPC over stdin/stdout.

```
AI Client → nexus-mcp sidecar (stdio) → Host API (:9600) → Plugin containers / Extension processes
                                                          ↕
                                              React frontend (Tauri webview)
```

### Backend (src-tauri/src/)

- **`lib.rs`** — App entry point. Creates `PluginManager`, wires extension IPC, spawns Host API server. `AppState = Arc<RwLock<PluginManager>>`.
- **`host_api/`** — Axum server with three route groups:
  - **Auth routes** (public) — `POST /api/v1/auth/token` — plugins exchange secret for session token
  - **MCP routes** (gateway auth) — `/api/v1/mcp/{tools,call,events}` — sidecar uses these
  - **Authenticated routes** — everything else (system, fs, process, docker, network, extensions, settings, storage)
  - `middleware.rs` — auth middleware extracts Bearer tokens from SessionStore
  - `approval.rs` — generic `ApprovalBridge` using oneshot channels + Tauri events for runtime permission dialogs
  - `network.rs` — HTTP proxy with SSRF protection and IPv6 canonicalization
- **`plugin_manager/`** — Docker lifecycle (pull, create, start, stop, remove), health checks, manifest validation, registry fetching
- **`permissions/`** — Permission checking and storage
  - `checker.rs` — maps request paths to required permissions. **Paths are post-strip** (no `/api` prefix — Axum `.nest()` strips it)
  - `store.rs` — persistence with approved_paths management
- **`extensions/`** — Native binary extension system
  - `mod.rs` — `Extension` trait, `OperationDef`, `RiskLevel` (serde `rename_all = "lowercase"`)
  - `process.rs` — spawns extension binaries, JSON-RPC protocol over stdin/stdout
  - `ipc.rs` — extension-to-extension IPC routing
  - `manifest.rs` — extension manifest parsing
  - `signing.rs` — ed25519 signature verification
- **`mcp_wrap/`** — Wraps arbitrary MCP servers as Nexus plugins (discovery, classification, code generation)
- **`commands/`** — Tauri IPC command handlers (one file per domain). These are the bridge between frontend `invoke()` calls and backend logic.

### Frontend (src/)

- **`App.tsx`** — Top-level router (view-based, not URL-based). Views: plugins, marketplace, settings, plugin-detail, extension-marketplace, extension-detail
- **`stores/appStore.ts`** — Single Zustand store for all app state. `busyPlugins` is `Record<string, PluginAction>`, NOT a Map.
- **`lib/tauri.ts`** — Typed wrappers around `invoke()` for every Tauri command
- **`types/`** — TypeScript type definitions matching Rust structs
- **`components/`** — Organized by feature: layout, plugins, marketplace, settings, extensions, permissions

### MCP Sidecar (crates/mcp-sidecar/)

Standalone Rust binary using `rmcp`. Bridges MCP stdio protocol to Host API HTTP endpoints. Built automatically by `beforeBuildCommand` in tauri.conf.json. Build manually with `just build-sidecar`.

### Plugin SDK (packages/plugin-sdk/)

Auto-generated TypeScript client from the OpenAPI spec. Published to GitHub Packages as `@imdanibytes/plugin-sdk`.

## Key Patterns

### Axum nested router path stripping
`.nest("/api", router)` strips the `/api` prefix before middleware sees the path. Permission patterns in `checker.rs` match against `/v1/...`, not `/api/v1/...`.

### Plugin auth flow
Plugin containers get `NEXUS_PLUGIN_SECRET` env var → POST to `/api/v1/auth/token` → receive short-lived Bearer token (15 min TTL) → use for all subsequent API calls.

### Extension manifest casing
Enums use `#[serde(rename_all = "snake_case")]`. Manifest JSON must use lowercase: `risk_level: "low"`, capability type: `"network_http"`. PascalCase fails deserialization.

### Runtime approval
Generic `ApprovalBridge` in `approval.rs` — creates a oneshot channel, emits a Tauri event to the frontend, frontend calls `runtime_approval_respond` command, bridge resolves the future. Used for permission dialogs and extension operation approval.

### Frontend state
Single Zustand store, no React Router. Navigation is `setView("marketplace")`. All Tauri commands are called through typed wrappers in `lib/tauri.ts`.

## Release Process

Push a `v*` tag to trigger CI:
1. `_build-app.yml` — cross-compile for aarch64 + x86_64 macOS
2. `_build-sdk.yml` — validate SDK compiles
3. `release.yml` — create GitHub Release with signed DMGs, publish SDK

Pre-release validation: `just release-dry-run v0.7.0`
