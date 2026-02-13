# Release Checklist

## One-Time Setup: Apple Code Signing

The release workflow signs and notarizes macOS builds. This requires an Apple Developer
Program membership ($99/year) and the following GitHub repository secrets:

| Secret | How to get it |
|--------|---------------|
| `APPLE_CERTIFICATE` | Export "Developer ID Application" cert as `.p12` from Keychain Access, then `base64 -i cert.p12 \| pbcopy` |
| `APPLE_CERTIFICATE_PASSWORD` | Password you set when exporting the `.p12` |
| `APPLE_ID` | Your Apple ID email |
| `APPLE_PASSWORD` | App-specific password — generate at [appleid.apple.com](https://appleid.apple.com) → Sign-In and Security → App-Specific Passwords |
| `APPLE_TEAM_ID` | 10-character Team ID from [Apple Developer portal](https://developer.apple.com/account) → Membership details |
| `KEYCHAIN_PASSWORD` | Any random string (used for the temporary CI keychain, e.g. `openssl rand -hex 16`) |
| `TAURI_SIGNING_PRIVATE_KEY` | Tauri updater key (already set up — this signs update bundles, separate from Apple signing) |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Password for the Tauri updater key (already set up) |

### Getting the certificate

1. Enroll in the [Apple Developer Program](https://developer.apple.com/programs/)
2. In the Developer portal → Certificates → create a **"Developer ID Application"** certificate
3. Download and install it in Keychain Access
4. In Keychain Access → My Certificates → right-click the "Developer ID Application: ..." cert → Export as `.p12`
5. Base64-encode: `base64 -i DeveloperIDApplication.p12 | pbcopy`
6. Paste as the `APPLE_CERTIFICATE` secret in GitHub

## Pre-Release

### 1. Version Bumps

Three files must have matching versions (the app version):

| File | Field |
|------|-------|
| `package.json` | `"version"` |
| `src-tauri/tauri.conf.json` | `"version"` |
| `src-tauri/Cargo.toml` | `version` under `[package]` |

The Plugin SDK version is independent (only bump if the SDK changed):

| File | Field |
|------|-------|
| `packages/plugin-sdk/package.json` | `"version"` |

### 2. Write Release Notes

Write release notes in a file at `docs/release-notes/v{VERSION}.md`. These get passed
to `gh release create --notes-file` so they appear on the GitHub release page **and** in
the in-app "What's New" panel via `latest.json`.

Format: Markdown. Keep it user-facing — what changed, not how.

### 3. Local Validation

Run the dry-run script to catch problems before pushing:

```bash
bash scripts/release-dry-run.sh v0.3.0
```

This checks: version consistency, release notes exist, git state is clean, builds compile,
tests pass, and the tag doesn't already exist.

### 4. Commit Version Bumps

```bash
git add package.json src-tauri/tauri.conf.json src-tauri/Cargo.toml docs/release-notes/
git commit -m "Release v{VERSION}"
git push origin main
```

## Release

### 5. Tag and Push

```bash
git tag v{VERSION}
git push origin v{VERSION}
```

This triggers `.github/workflows/release.yml` which:
- Signs and notarizes macOS builds (aarch64 + x86_64) via Apple Developer certificate
- Builds the Plugin SDK
- Generates `latest.json` (used by the in-app updater, includes release notes for "What's New")
- Creates a GitHub Release with the release notes and all artifacts
- Publishes the SDK to GitHub Packages (if SDK version changed)

### 6. Monitor CI

Watch the [Actions tab](https://github.com/imdanibytes/nexus/actions) for the Release workflow.
Both build matrix jobs + publish must succeed.

If signing fails: check that all `APPLE_*` secrets are set correctly. The most common
issue is an expired certificate or wrong app-specific password.

## Post-Release

### 7. Verify

- [ ] GitHub Release page has correct notes, DMGs, and `latest.json`
- [ ] `latest.json` has correct version, platform URLs, and release notes
- [ ] Download a DMG and open the app — **no Gatekeeper warning** (signed + notarized)
- [ ] Run `codesign -dv --verbose=2 /Applications/Nexus.app` — shows "Developer ID Application"
- [ ] Run `spctl -a -vvv /Applications/Nexus.app` — shows "source=Notarized Developer ID"
- [ ] In a previous version, "Check for Updates" shows the new version with "What's New"
- [ ] SDK published to GitHub Packages (if SDK version changed)

### 8. Announce

Post in relevant channels. The GitHub Release auto-generates a changelog from PR titles
via `--generate-notes`, but the curated notes from step 2 are the primary user-facing summary.
