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

### 1. Write Release Notes

Write release notes in a file at `docs/release-notes/v{VERSION}.md`.

The file has two sections separated by `---`:
- **Above the separator**: Brief bullet summary shown in the in-app "What's New" panel
- **Below the separator**: Full detailed changelog shown on the GitHub Release page

### 2. Local Validation

Run the dry-run script to catch problems before pushing:

```bash
bash scripts/release-dry-run.sh v{VERSION}
```

This checks: release notes exist, git state is clean, builds compile, and tests pass.

### 3. Commit Release Notes

```bash
git add docs/release-notes/
git commit -m "Release v{VERSION}"
git push origin main
```

**Note:** You do NOT need to manually bump version numbers. The release workflow
automatically syncs all version fields from the git tag:

| File | Field | Synced automatically |
|------|-------|:---:|
| `package.json` | `"version"` | yes |
| `src-tauri/tauri.conf.json` | `"version"` | yes |
| `src-tauri/Cargo.toml` | `version` under `[package]` | yes |
| `packages/plugin-sdk/package.json` | `"version"` | yes |

## Release

### 4. Tag and Push

```bash
git tag v{VERSION}
git push origin v{VERSION}
```

This triggers `.github/workflows/release.yml` which:
- Syncs all version fields from the tag (no manual bumps needed)
- Signs and notarizes macOS builds (aarch64 + x86_64) via Apple Developer certificate
- Builds Linux (.deb, .AppImage) and Windows (.exe) installers
- Builds and publishes the Plugin SDK to GitHub Packages
- Generates `latest.json` (used by the in-app updater, includes "What's New" summary)
- Creates a GitHub Release with the full release notes and all artifacts

### 5. Monitor CI

Watch the [Actions tab](https://github.com/imdanibytes/nexus/actions) for the Release workflow.
All build matrix jobs + publish must succeed.

If signing fails: check that all `APPLE_*` secrets are set correctly. The most common
issue is an expired certificate or wrong app-specific password.

## Post-Release

### 6. Verify

- [ ] GitHub Release page has correct notes, DMGs, and `latest.json`
- [ ] `latest.json` has correct version, platform URLs, and release notes
- [ ] Download a DMG and open the app — **no Gatekeeper warning** (signed + notarized)
- [ ] Run `codesign -dv --verbose=2 /Applications/Nexus.app` — shows "Developer ID Application"
- [ ] Run `spctl -a -vvv /Applications/Nexus.app` — shows "source=Notarized Developer ID"
- [ ] In a previous version, "Check for Updates" shows the new version with "What's New"
- [ ] SDK published to GitHub Packages with correct version

### 7. Announce

Post in relevant channels. The GitHub Release auto-generates a changelog from PR titles
via `--generate-notes`, but the curated notes from step 1 are the primary user-facing summary.
