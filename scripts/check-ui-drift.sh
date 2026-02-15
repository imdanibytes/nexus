#!/usr/bin/env bash
# Compares host app UI components against plugin-ui copies.
# Normalizes import paths before diffing so only real changes flag.
set -euo pipefail

HOST_DIR="src/components/ui"
PLUGIN_DIR="packages/plugin-ui/src/components"
DRIFT=0

normalize() {
  sed -E \
    -e 's|from "@/lib/utils"|from "../lib/utils"|g' \
    -e 's|from "@/components/ui/([^"]+)"|from "./\1"|g' \
    -e '/eslint-disable/d' \
    "$1"
}

for plugin_file in "$PLUGIN_DIR"/*.tsx; do
  name=$(basename "$plugin_file")
  host_file="$HOST_DIR/$name"

  if [[ ! -f "$host_file" ]]; then
    echo "SKIP: $name (no host counterpart)"
    continue
  fi

  if ! diff <(normalize "$host_file") <(cat "$plugin_file") > /dev/null 2>&1; then
    echo "DRIFT: $name"
    diff <(normalize "$host_file") <(cat "$plugin_file") || true
    DRIFT=1
  else
    echo "  OK: $name"
  fi
done

if [[ $DRIFT -eq 1 ]]; then
  echo ""
  echo "Component drift detected. Reconcile manually."
  exit 1
fi

echo ""
echo "All components in sync."
