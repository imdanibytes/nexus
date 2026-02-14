# Nexus development commands

# Default: list available recipes
default:
    @just --list

# ── App ────────────────────────────────────────────────────────────────────

# Run the app in dev mode
dev:
    cargo tauri dev

# Build the signed app bundle (.app + .dmg)
build-app:
    @set -a && [ -f .env.local ] && . .env.local; set +a && cargo tauri build

# Check Rust code (fast compile check, no codegen)
check:
    cd src-tauri && cargo check

# Run clippy lints
clippy:
    cd src-tauri && cargo clippy

# Run TypeScript type checking
typecheck:
    npx tsc --noEmit

# Run all lints (clippy + tsc + eslint)
lint: clippy typecheck
    pnpm lint

# ── Sidecar ────────────────────────────────────────────────────────────────

# Build the MCP sidecar binary
build-sidecar:
    bash scripts/build-mcp-sidecar.sh

# ── Extensions ─────────────────────────────────────────────────────────────

# Build all example extensions
build-extensions:
    #!/usr/bin/env bash
    set -euo pipefail
    for dir in examples/extensions/*/; do
        name=$(basename "$dir")
        echo "Building extension: $name"
        cd "$dir" && cargo build --release && cd - > /dev/null
    done

# Build a specific example extension
build-extension name:
    cd examples/extensions/{{name}} && cargo build --release

# ── SDK ────────────────────────────────────────────────────────────────────

# Regenerate the plugin SDK from OpenAPI spec
build-sdk:
    pnpm sdk

# ── All ────────────────────────────────────────────────────────────────────

# Build everything: sidecar, extensions, frontend, app
build-all: build-sidecar build-extensions build-app
