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
just build-app        # Signed production build (.app + .dmg), reads .env.local for signing creds
just build-all        # Extensions + app
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
AI Client → MCP (Streamable HTTP) → Host API (:9600/mcp) → Plugin containers / Extension processes
                                                          ↕
                                              React frontend (Tauri webview)
```

### Backend (src-tauri/src/)

- **`lib.rs`** — App entry point. Creates `PluginManager`, wires extension IPC, spawns Host API server. `AppState = Arc<RwLock<PluginManager>>`.
- **`host_api/`** — Axum server with three route groups:
  - **Auth routes** (public) — `POST /api/v1/auth/token` — plugins exchange secret for session token
  - **MCP routes** (gateway auth) — `/mcp` (Streamable HTTP) + `/api/v1/mcp/{tools,call,events}` (legacy)
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

### MCP Gateway

Native Streamable HTTP MCP server at `/mcp`. AI clients connect directly via HTTP — no sidecar binary needed. Gateway token auth via `X-Nexus-Gateway-Token` header.

### Nexus SDK (packages/nexus-sdk/)

TypeScript SDK for plugins. `src/client/` is auto-generated from the OpenAPI spec; `src/index.ts` is the hand-written L2 wrapper (NexusPlugin class, host event bridge). Published to GitHub Packages as `@imdanibytes/nexus-sdk`. See `packages/nexus-sdk/CLAUDE.md` for the generated vs hand-written boundary.

## Internationalization (i18n)

All user-facing strings live in `src/i18n/locales/{lang}/` as JSON files, one per namespace. **Never hardcode UI strings in components.**

### Structure

```
src/i18n/
  index.ts           # i18next init, resource imports, LANGUAGES array
  types.ts           # TypeScript module augmentation
  context/           # Translation context — intent/description per key (NOT loaded at runtime)
    common.json
    plugins.json
    settings.json
    permissions.json
  locales/
    en/              # English (source of truth)
      common.json    # Nav, actions, status badges, errors, confirmations, time
      plugins.json   # Marketplace, viewport, overlays, MCP wrap wizard, extensions
      settings.json  # All settings tabs, registries, help
      permissions.json # Install dialog, runtime approval, permission list
    ja/              # Japanese
    es/              # Spanish
    ko/              # Korean
    zh/              # Chinese (Simplified)
    de/              # German
```

### When modifying UI strings

1. **Adding a string**: Add the key to `en/*.json` first, then add the same key to ALL other locale files. Add a context entry to `context/*.json` describing the intent (button label, heading, toast, etc.).
2. **Removing a string**: Remove from ALL locale files and the context file.
3. **Changing a string**: Update `en/*.json`, then update all translations. Update context if the intent changed.
4. **Adding a locale**: Create `locales/{code}/` with all 4 namespace files, add imports to `index.ts`, add to `LANGUAGES` array.

### Context files (`src/i18n/context/`)

Each key maps to a description of how the string is used — button label, toast notification, dialog heading, badge text, etc. This matters for translation: "Save" as a button is different from "Save" as a noun in many languages. Translators (human or AI) should read the context file before translating.

### Rules

- Use `useTranslation("namespace")` — the namespace determines which JSON file is used
- Cross-namespace access: `t("common:action.save")` with colon prefix (types are relaxed for this)
- Interpolation: `{{variable}}` syntax — e.g. `t("error.startFailed", { error: msg })`
- Plurals: `_one` / `_other` suffixes — e.g. `toolCount_one`, `toolCount_other`
- HTML in strings: use `<Trans>` component or `dangerouslySetInnerHTML` with `t()`
- Language switcher persists to `localStorage` key `nexus-language` and notifies plugins via `postMessage`

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

## Registry CLI (nexus-registry)

Rust CLI in `src-tauri/crates/nexus-registry/` for managing the plugin/extension registry at `https://github.com/imdanibytes/registry`. **Always use this instead of manually editing registry YAML.**

```bash
# Install
cd src-tauri && cargo install --path crates/nexus-registry

# Add a new plugin (interactive if flags omitted, auto-fetches manifest SHA256)
nexus-registry add plugin \
  --id com.yourname.my-plugin \
  --name "My Plugin" \
  --version 1.0.0 \
  --author yourname \
  --image ghcr.io/yourname/nexus-plugin-name:1.0.0 \
  --manifest-url https://raw.githubusercontent.com/yourname/nexus-plugin-name/main/plugin.json \
  --categories "productivity,ai-tools"

# Validate all registry YAML against schemas
nexus-registry validate /path/to/registry

# Build index.json from YAML sources
nexus-registry build /path/to/registry

# Publish to remote registry (clones, validates, branches, pushes, opens PR with auto-merge)
nexus-registry publish \
  --registry https://github.com/imdanibytes/registry.git \
  --package plugins/com.yourname.my-plugin.yaml
```

**Updating an existing entry** (version bumps — the most common operation):

```bash
cd ~/workspace/registry
nexus-registry update \
  --id com.nexus.cookie-jar \
  --version 0.6.0 \
  --image ghcr.io/imdanibytes/nexus-plugin-cookie-jar:0.6.0 \
  --image-digest "sha256:..."
# Auto-fetches manifest_sha256 from manifest_url. Preserves all other fields.
```

### Registry update workflow (version bump)

1. Tag the plugin repo: `git tag -a v0.6.0 -m "v0.6.0" && git push origin main v0.6.0`
2. Wait for CI to build the Docker image — grab the digest from the GitHub Actions summary
3. `cd ~/workspace/registry && nexus-registry update --id <id> --version <ver> --image <image:tag> --image-digest <sha256:...>`
4. Commit and push to registry (or use `nexus-registry publish` for new entries)

## Release Process

Push a `v*` tag to trigger CI:
1. `_build-app.yml` — cross-compile for aarch64 + x86_64 macOS
2. `_build-sdk.yml` — validate SDK compiles
3. `release.yml` — create GitHub Release with signed DMGs, publish SDK

Pre-release validation: `just release-dry-run v0.7.0`
