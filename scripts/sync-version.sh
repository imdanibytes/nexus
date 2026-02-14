#!/usr/bin/env bash
# Sync all version fields to a single value.
# Usage: bash scripts/sync-version.sh 0.4.0
set -euo pipefail

VERSION="${1:-}"
if [[ -z "$VERSION" ]]; then
  echo "Usage: bash scripts/sync-version.sh 0.4.0"
  exit 1
fi

echo "Syncing version to $VERSION"

# package.json
jq --arg v "$VERSION" '.version = $v' package.json > tmp.json && mv tmp.json package.json
echo "  package.json"

# tauri.conf.json
jq --arg v "$VERSION" '.version = $v' src-tauri/tauri.conf.json > tmp.json && mv tmp.json src-tauri/tauri.conf.json
echo "  src-tauri/tauri.conf.json"

# Cargo.toml
awk -v ver="$VERSION" '/^version = "/{print "version = \""ver"\""; next} {print}' src-tauri/Cargo.toml > tmp.toml && mv tmp.toml src-tauri/Cargo.toml
echo "  src-tauri/Cargo.toml"

# Plugin SDK
jq --arg v "$VERSION" '.version = $v' packages/plugin-sdk/package.json > tmp.json && mv tmp.json packages/plugin-sdk/package.json
echo "  packages/plugin-sdk/package.json"

echo "Done."
