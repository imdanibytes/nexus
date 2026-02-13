#!/usr/bin/env bash
# Build the nexus-mcp sidecar binary and place it where Tauri expects.
# Tauri sidecars must be named: {name}-{target-triple}
# e.g. nexus-mcp-aarch64-apple-darwin

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
SRC_MCP="$PROJECT_ROOT/src-mcp"
BINARIES_DIR="$PROJECT_ROOT/src-tauri/binaries"

# Determine target triple
TARGET="${CARGO_BUILD_TARGET:-$(rustc -vV | sed -n 's/host: //p')}"

echo "Building nexus-mcp for target: $TARGET"

cd "$SRC_MCP"
cargo build --release ${CARGO_BUILD_TARGET:+--target "$CARGO_BUILD_TARGET"}

# Find the built binary
if [ -n "${CARGO_BUILD_TARGET:-}" ]; then
    BINARY="$SRC_MCP/target/$TARGET/release/nexus-mcp"
else
    BINARY="$SRC_MCP/target/release/nexus-mcp"
fi

# Add .exe suffix on Windows
case "$TARGET" in
    *windows*) BINARY="$BINARY.exe"; EXT=".exe" ;;
    *) EXT="" ;;
esac

mkdir -p "$BINARIES_DIR"
DEST="$BINARIES_DIR/nexus-mcp-${TARGET}${EXT}"
cp "$BINARY" "$DEST"

# macOS: re-sign after copy so the binary isn't killed by code signature checks.
# Cargo produces ad-hoc linker-signed binaries; copying can invalidate the signature.
case "$TARGET" in
    *apple*) codesign --force --sign - "$DEST" ;;
esac

echo "Sidecar placed at: $DEST"
