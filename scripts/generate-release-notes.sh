#!/usr/bin/env bash
# Generate release notes from conventional commits between two git tags.
# Usage: ./scripts/generate-release-notes.sh v0.11.0 [v0.10.1]
#   $1 = current tag (required)
#   $2 = previous tag (auto-detected if omitted)
#
# If docs/release-notes/$TAG.md exists, prints that instead (manual override).
# Output: markdown to stdout.

set -euo pipefail

TAG="${1:?Usage: generate-release-notes.sh <tag> [prev-tag]}"
REPO_URL="https://github.com/imdanibytes/nexus"

# Manual override — if a hand-written file exists, use it
NOTES_FILE="docs/release-notes/${TAG}.md"
if [ -f "$NOTES_FILE" ]; then
  cat "$NOTES_FILE"
  exit 0
fi

# Find previous tag if not provided
if [ -n "${2:-}" ]; then
  PREV_TAG="$2"
else
  PREV_TAG=$(git tag --sort=-v:refname | grep -E '^v[0-9]' | grep -v "^${TAG}$" | head -1)
fi

if [ -z "$PREV_TAG" ]; then
  echo "No previous tag found"
  exit 1
fi

# Collect commits: hash|author|subject
declare -a FEATURES=()
declare -a FIXES=()
declare -a OTHER=()

while IFS='|' read -r hash author subject; do
  [ -z "$hash" ] && continue

  # Extract conventional commit prefix
  prefix=""
  message="$subject"
  if [[ "$subject" =~ ^([a-z]+)(\(.+\))?:\ (.+)$ ]]; then
    prefix="${BASH_REMATCH[1]}"
    message="${BASH_REMATCH[3]}"
  fi

  # Extract issue references (#N) and build links
  issue_links=""
  if [[ "$message" =~ \#([0-9]+) ]]; then
    # Find all issue refs
    issue_links=$(echo "$message" | grep -oE '#[0-9]+' | while read -r ref; do
      num="${ref#\#}"
      echo -n " [${ref}](${REPO_URL}/issues/${num})"
    done)
    # Remove issue refs from message text (they're now links at the end)
    message=$(echo "$message" | sed -E 's/ *\(?[Cc]loses? #[0-9]+\)? *//g; s/ *#[0-9]+ *//g' | sed 's/ *$//')
  fi

  short_hash="${hash:0:7}"
  entry="- ${message}${issue_links} — [\`${short_hash}\`](${REPO_URL}/commit/${hash})"

  case "$prefix" in
    feat)     FEATURES+=("$entry") ;;
    fix)      FIXES+=("$entry") ;;
    chore|build|ci|docs|refactor|style|perf|test)
              OTHER+=("$entry") ;;
    *)        OTHER+=("$entry") ;;
  esac
done < <(git log "${PREV_TAG}..${TAG}" --pretty=format:"%H|%aN|%s" --no-merges --reverse)

# Build markdown
{
  if [ ${#FEATURES[@]} -gt 0 ]; then
    echo "## Features"
    echo ""
    printf '%s\n' "${FEATURES[@]}"
    echo ""
  fi

  if [ ${#FIXES[@]} -gt 0 ]; then
    echo "## Fixes"
    echo ""
    printf '%s\n' "${FIXES[@]}"
    echo ""
  fi

  if [ ${#OTHER[@]} -gt 0 ]; then
    echo "## Other"
    echo ""
    printf '%s\n' "${OTHER[@]}"
    echo ""
  fi

  echo "**Full changelog**: [\`${PREV_TAG}...${TAG}\`](${REPO_URL}/compare/${PREV_TAG}...${TAG})"
}
