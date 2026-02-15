#!/usr/bin/env bash
# Release dry-run: validates everything needed for a successful release.
# Usage: bash scripts/release-dry-run.sh v0.3.0
set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BOLD='\033[1m'
RESET='\033[0m'

pass=0
fail=0
warn=0

ok()   { ((pass++)); echo -e "  ${GREEN}✓${RESET} $1"; }
fail() { ((fail++)); echo -e "  ${RED}✗${RESET} $1"; }
warn() { ((warn++)); echo -e "  ${YELLOW}!${RESET} $1"; }

# --- Args ---
TAG="${1:-}"
if [[ -z "$TAG" ]]; then
  echo -e "${RED}Usage: bash scripts/release-dry-run.sh v0.3.0${RESET}"
  exit 1
fi

VERSION="${TAG#v}"
echo -e "${BOLD}Release dry-run for ${TAG} (version ${VERSION})${RESET}"
echo ""

# --- 1. Version consistency ---
echo -e "${BOLD}Version Consistency${RESET}"

PKG_VERSION=$(jq -r '.version' package.json)
TAURI_VERSION=$(jq -r '.version' src-tauri/tauri.conf.json)
# Cargo.toml needs grep since it's TOML, not JSON
CARGO_VERSION=$(grep '^version' src-tauri/Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')

# Versions are auto-synced from the git tag at build time, so mismatches are fine locally
if [[ "$PKG_VERSION" == "$VERSION" ]]; then
  ok "package.json: $PKG_VERSION"
else
  ok "package.json: $PKG_VERSION (will be synced to $VERSION at build time)"
fi
if [[ "$TAURI_VERSION" == "$VERSION" ]]; then
  ok "tauri.conf.json: $TAURI_VERSION"
else
  ok "tauri.conf.json: $TAURI_VERSION (will be synced to $VERSION at build time)"
fi
if [[ "$CARGO_VERSION" == "$VERSION" ]]; then
  ok "Cargo.toml: $CARGO_VERSION"
else
  ok "Cargo.toml: $CARGO_VERSION (will be synced to $VERSION at build time)"
fi

echo ""

# --- 2. Release notes ---
echo -e "${BOLD}Release Notes${RESET}"

NOTES_FILE="docs/release-notes/${TAG}.md"
if [[ -f "$NOTES_FILE" ]]; then
  LINES=$(wc -l < "$NOTES_FILE" | tr -d ' ')
  if [[ "$LINES" -gt 2 ]]; then
    ok "Found $NOTES_FILE ($LINES lines)"
  else
    warn "$NOTES_FILE exists but looks empty ($LINES lines)"
  fi
else
  warn "No release notes at $NOTES_FILE (GitHub will auto-generate from PR titles)"
fi

echo ""

# --- 3. Git state ---
echo -e "${BOLD}Git State${RESET}"

if git diff --quiet && git diff --cached --quiet; then
  ok "Working tree clean"
else
  fail "Uncommitted changes (commit or stash before tagging)"
fi

BRANCH=$(git rev-parse --abbrev-ref HEAD)
[[ "$BRANCH" == "main" ]] && ok "On main branch" || warn "On branch '$BRANCH' (releases usually tag main)"

if git rev-parse "$TAG" >/dev/null 2>&1; then
  fail "Tag $TAG already exists"
else
  ok "Tag $TAG is available"
fi

# Check if local main is up to date with remote
if git fetch origin main --dry-run 2>&1 | grep -q "up to date" || [[ -z "$(git log HEAD..origin/main --oneline 2>/dev/null)" ]]; then
  ok "Up to date with origin/main"
else
  warn "Local branch may be behind origin/main — consider pulling"
fi

echo ""

# --- 4. Apple signing secrets ---
echo -e "${BOLD}Apple Code Signing (GitHub Secrets)${RESET}"

# We can't check the actual secret values, but we can remind what's needed
SECRETS_URL="https://github.com/imdanibytes/nexus/settings/secrets/actions"
echo "  Verify these secrets are set at: $SECRETS_URL"
for secret in APPLE_CERTIFICATE APPLE_CERTIFICATE_PASSWORD APPLE_ID APPLE_PASSWORD APPLE_TEAM_ID KEYCHAIN_PASSWORD; do
  echo -e "  ${YELLOW}?${RESET} $secret — cannot verify locally, check GitHub"
done

# Check if a "Developer ID Application" cert exists in the local keychain
if security find-identity -v -p codesigning 2>/dev/null | grep -q "Developer ID Application"; then
  IDENTITY=$(security find-identity -v -p codesigning 2>/dev/null | grep "Developer ID Application" | awk -F'"' '{print $2}')
  ok "Local signing identity found: $IDENTITY"
else
  warn "No 'Developer ID Application' certificate in local keychain (CI uses its own keychain — this is only needed for local signed builds)"
fi

echo ""

# --- 5. Build checks ---
echo -e "${BOLD}Build Checks${RESET}"

echo "  Checking TypeScript..."
if pnpm tsc -b --noEmit 2>/dev/null; then
  ok "TypeScript compiles"
else
  fail "TypeScript errors"
fi

echo "  Checking Rust..."
if (cd src-tauri && cargo check 2>/dev/null); then
  ok "Cargo check passes"
else
  fail "Cargo check failed"
fi

echo "  Running Rust tests..."
if (cd src-tauri && cargo test --lib 2>/dev/null); then
  ok "Rust tests pass"
else
  fail "Rust tests failed"
fi

echo "  Checking lint..."
if pnpm lint 2>/dev/null; then
  ok "ESLint passes"
else
  warn "ESLint issues (non-blocking)"
fi

echo ""

# --- 6. SDK check ---
echo -e "${BOLD}Nexus SDK${RESET}"

SDK_VERSION=$(jq -r '.version' packages/nexus-sdk/package.json)
echo "  SDK version: $SDK_VERSION"
if (cd packages/nexus-sdk && npm run build 2>/dev/null); then
  ok "SDK builds"
else
  fail "SDK build failed"
fi

echo ""

# --- Summary ---
echo -e "${BOLD}────────────────────────${RESET}"
echo -e "  ${GREEN}Passed: $pass${RESET}  ${RED}Failed: $fail${RESET}  ${YELLOW}Warnings: $warn${RESET}"

if [[ $fail -gt 0 ]]; then
  echo -e "${RED}${BOLD}Release blocked — fix failures above.${RESET}"
  exit 1
elif [[ $warn -gt 0 ]]; then
  echo -e "${YELLOW}${BOLD}Warnings present — review before releasing.${RESET}"
  exit 0
else
  echo -e "${GREEN}${BOLD}All clear. Ready to tag and push.${RESET}"
  exit 0
fi
