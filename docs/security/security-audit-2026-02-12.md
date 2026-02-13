# Security Audit Report: Nexus Plugin System

**Date**: 2026-02-12
**Scope**: `/src-tauri/` (Rust backend + Host API)
**Auditor**: Automated (Claude)

---

## Summary

| Severity | Count | Status |
|----------|-------|--------|
| Critical | 4 | All fixed |
| High | 4 | All fixed |
| Medium | 4 | 3 fixed, 1 accepted (CSP) |
| Low | 2 | 1 fixed, 1 deferred (rate limiting) |

---

## CRITICAL SEVERITY

### 1. Path Traversal in Filesystem API

**File**: `src/host_api/filesystem.rs`
**Confidence**: 100%

The filesystem read/write/list endpoints accept arbitrary paths from plugins without validation against `approved_paths`. While the permission system includes an `approved_paths` field, it was never checked in the filesystem handlers.

A malicious plugin could read any file on the host system (`/etc/passwd`, `~/.ssh/id_rsa`, `~/.aws/credentials`), write to arbitrary locations, or list sensitive directories.

**Fix**: Added auth context to handlers, path canonicalization, Nexus data dir blocking, and `approved_paths` enforcement.

---

### 2. Server-Side Request Forgery (SSRF) in Network Proxy

**File**: `src/host_api/network.rs`
**Confidence**: 100%

The `/api/v1/network/proxy` endpoint accepts arbitrary URLs without validation. A malicious plugin can scan internal networks, access cloud metadata services (169.254.169.254), exfiltrate data, or port-scan hosts.

**Fix**: URL parsing + classification (local vs internet), metadata IP blocking, scheme restriction (http/https only), Host API self-access blocking, timeout + size limits.

---

### 3. Auth Tokens Stored in Plain Text

**File**: `src/plugin_manager/storage.rs`
**Confidence**: 95%

Plugin authentication tokens stored as raw UUIDs in `plugins.json`. Any process with read access can extract tokens and impersonate plugins. No token rotation mechanism.

**Fix**: SHA-256 hashing at rest. Raw tokens only exist in-memory during install (passed to container via env var). Auto-migration on load converts existing raw tokens to hashes.

---

### 4. Missing Permission Check in Network Proxy

**File**: `src/permissions/checker.rs`
**Confidence**: 100%

The `required_permission_for_endpoint()` returns `None` for network endpoints with a TODO comment about per-request checking, but the handler never checks permissions. Any authenticated plugin can make arbitrary HTTP requests.

**Fix**: Combined with Finding #2 — permission enforcement added directly in the handler.

---

## HIGH SEVERITY

### 5. Docker Container Escape Vectors

**File**: `src/plugin_manager/docker.rs`
**Confidence**: 85%

Containers run as root by default, no `cap_drop`, no seccomp profile, `host.docker.internal` exposed. Could be extended to request volume mounts.

**Fix**: Added `cap_drop: ALL`, `cap_add: NET_BIND_SERVICE`, `security_opt: no-new-privileges:true`, explicit `binds: None` / `mounts: None`.

---

### 6. Resource Quotas Not Enforced

**File**: `src/plugin_manager/storage.rs`, `docker.rs`
**Confidence**: 90%

`cpu_quota_percent` and `memory_limit_mb` exist in settings and UI but are never applied to Docker containers. A malicious plugin can DoS via CPU/memory exhaustion.

**Fix**: Added `ResourceLimits` struct, compute from settings, pass to `create_container` as `nano_cpus` and `memory` in `HostConfig`.

---

### 7. Privilege Escalation via Auto-Granted Permissions

**File**: `src/plugin_manager/mod.rs`
**Confidence**: 85%

During installation, all manifest-declared permissions were auto-granted without user consent.

**Status**: RESOLVED. Two-step install dialog now requires explicit user approval. Backend `install()` only grants `approved_permissions`.

---

### 8. Unauthenticated Registry Fetching

**File**: `src/plugin_manager/registry.rs`
**Confidence**: 85%

Bare `reqwest::get()` with no timeout, no size limit, no redirect cap, and `file://` allowed for remote sources. MITM and slow-loris attacks possible.

**Fix**: Shared HTTP client with 30s timeout, 5-redirect limit, 10MB response size cap. `file://` blocked in remote manifest fetch.

---

## MEDIUM SEVERITY

### 9. Arbitrary Docker Images Allowed

**File**: `src/plugin_manager/manifest.rs`
**Confidence**: 80%

No registry whitelist or image signature verification. Could pull from attacker-controlled registries.

**Fix**: Docker image digest pinning. Manifest declares `image_digest: "sha256:..."`. After pulling, Nexus compares the pulled image's registry digest to the declared value. Mismatch rejects the install. Prevents tag mutation, registry compromise, and MITM attacks. Optional for local dev installs, logged warning when absent.

---

### 10. Frontend XSS via Plugin Metadata

**File**: `src/components/plugins/PluginViewport.tsx`
**Confidence**: 75%

No validation on manifest field lengths. React auto-escapes, but Unicode direction overrides could spoof names and very long fields could DoS UI.

**Fix**: Added field length limits in manifest validation.

---

### 11. CSP Allows Unsafe Inline Styles

**File**: `tauri.conf.json`
**Confidence**: 80%

`style-src 'self' 'unsafe-inline'` in CSP. Minor attack surface for CSS injection.

**Status**: Deferred. Requires extracting all Tailwind inline styles.

---

### 12. Plugin Tokens Never Expire

**File**: `src/plugin_manager/storage.rs`
**Confidence**: 85%

Tokens are UUIDs generated once at install, never rotated. Survive permission revocations.

**Fix (Phase 1)**: Tokens are ephemeral — every `start()` recreates the container with a fresh UUID secret.

**Fix (Phase 2)**: AWS-style temporary credentials. Plugin secret (`NEXUS_PLUGIN_SECRET`) is exchanged for short-lived access tokens (15 min TTL) via `POST /api/v1/auth/token`. The secret never reaches the browser — only access tokens are exposed to frontend code. Session store validates access tokens; expired tokens auto-reject. Plugin server caches tokens and refreshes 30s before expiry.

---

## LOW SEVERITY

### 13. No Rate Limiting on Host API

No rate limiting middleware. A malicious plugin could DoS with millions of requests.

**Fix**: Per-plugin fixed-window rate limiter (100 req/s per plugin). Returns 429 + Retry-After header. Runs as axum middleware after auth.

### 14. Verbose Error Messages

Some handlers return `INTERNAL_SERVER_ERROR` with unvalidated error text.

**Status**: Partially addressed (filesystem/docker now return generic 403).

---

## Supply Chain

- **Rust deps**: No known CVEs. `reqwest` needs TLS hardening (addressed).
- **JS deps**: React 19.2.0, Vite 7.3.1, Tauri 2.x — all current.
- Recommend periodic `cargo audit` and `npm audit`.

---

## Remediation Tracking

### Original Audit Findings

| Finding | Status | Notes |
|---------|--------|-------|
| #1 Path Traversal | Fixed | Auth context + canonicalization + data dir blocking + approved_paths enforcement |
| #2 SSRF | Fixed | URL classification + metadata blocking + Host API relay blocking + timeouts |
| #3 Plain Text Tokens | Fixed | SHA-256 hashing at rest, auto-migration of existing tokens |
| #4 Network Perms | Fixed | NetworkLocal/NetworkInternet enforced in handler |
| #5 Docker Hardening | Fixed | cap_drop ALL, no-new-privileges, explicit no mounts |
| #6 Resource Quotas | Fixed | CPU + memory limits passed to Docker HostConfig |
| #7 Auto-Grant Perms | Fixed | Two-step install dialog (previous session) |
| #8 Registry Fetching | Fixed | Hardened client: 30s timeout, 5 redirects, 10MB cap |
| #9 Arbitrary Images | Fixed | Digest pinning: optional `image_digest` in manifest, verified after pull. Rejects on mismatch. |
| #10 Manifest XSS | Fixed | Length limits + bidi override detection |
| #11 CSP Inline | Accepted | React dynamic styles require unsafe-inline for style-src |
| #12 Token Rotation | Fixed | Ephemeral secrets + AWS-style temporary credentials (15 min access tokens via POST /v1/auth/token) |
| #13 Rate Limiting | Fixed | Per-plugin 100 req/s fixed-window limiter, 429 + Retry-After |
| #14 Verbose Errors | Fixed | All fs/docker/network handlers return generic 403 |

### Additional Security Hardening

| Finding | Status | Notes |
|---------|--------|-------|
| Host API network exposure | Fixed | Bound to 127.0.0.1:9600 (was 0.0.0.0) |
| Request body size limits | Fixed | 5MB DefaultBodyLimit on authenticated routes |
| File read size limit | Fixed | 5MB cap prevents memory exhaustion from huge reads |
| Settings schema validation | Fixed | PUT /settings validates keys + types against manifest |
| Audit logging | Fixed | All authenticated requests logged with plugin_id, method, path, status |
| Manifest spec documented | Done | docs/manifest-spec.md — source of truth for registry validation |

### Re-Audit Findings (Pass 2)

| Finding | Status | Notes |
|---------|--------|-------|
| Redirect bypass in network proxy | Fixed | Custom redirect policy re-validates each hop: blocks metadata, Host API, and public→private redirects |
| DNS rebinding in network proxy | Fixed | Full IP-level validation: `resolve_and_classify()` resolves hostname via DNS, validates resolved IP (blocks metadata IPs), classifies as private/public based on actual IP. Connection pinned to resolved address via `reqwest::resolve()` to prevent TOCTOU race. IP literals parsed directly without DNS. |
| Docker `mounts` not explicitly empty | Fixed | Added `mounts: Some(vec![])` for defense-in-depth alongside `binds: Some(vec![])` |
