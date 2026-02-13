# Nexus Roadmap

## In Progress

### Host Extensions (Dynamic, Process-Based)
Native process-based extensions spawned as child processes. JSON-RPC over stdin/stdout.
Full host access, crash isolation, any language. Ed25519 binary signing.
**Design doc**: `docs/arch/p2p-agent-communication.md` (extensions plan in `.claude/plans/`)

## Planned

### OS Notifications for Runtime Approval
**Plugin**: `tauri-plugin-notification`

When Nexus is not in focus and a plugin requests a permission that needs runtime approval
(filesystem path, extension scope, high-risk operation), push a native OS notification
instead of silently blocking or timing out.

**Scope**:
- Add `tauri-plugin-notification` dependency
- Hook into `ApprovalBridge` — if the window is not focused when approval is requested,
  fire a notification with the plugin name + requested resource
- Clicking the notification brings Nexus to front with the approval dialog visible
- Fallback: if notifications are denied by OS, current behavior (60s timeout → deny) unchanged

### Encrypted Secret Vault
**Plugin**: `tauri-plugin-stronghold`

Replace JSON-file-based secret storage with an OS-level encrypted vault (libsodium/XChaCha20-Poly1305).

**What moves into the vault**:
- Plugin auth token hashes (currently `plugins.json`)
- MCP gateway token (currently `mcp_gateway_token` plaintext file)
- Extension signing keys (currently planned as `trusted_keys.json`)
- Any future API keys or credentials

**What stays as JSON**: Plugin metadata, settings, permissions (not secrets).

**Migration**: On first launch after upgrade, read existing secrets from JSON files,
write them into the vault, then delete the plaintext copies.

### Persistent Plugin Data Store
**Plugin**: `tauri-plugin-store`

A Host API endpoint that gives plugins a persistent key-value store that:
- Survives Docker container recreation (data lives on the host, not in the volume)
- Survives plugin uninstall/reinstall (opt-in, plugin declares `persistent_data: true` in manifest)
- Is scoped per-plugin (plugin A cannot read plugin B's store)
- Has size limits (e.g. 10 MB per plugin)

**New Host API endpoints**:
- `GET /api/v1/store/{key}` — read a value
- `PUT /api/v1/store/{key}` — write a value
- `DELETE /api/v1/store/{key}` — delete a value
- `GET /api/v1/store` — list keys

**New permission**: `store:read`, `store:write`

**Storage location**: `~/.nexus/plugin-data/{plugin-id}/` (outside the Docker volume)

**On uninstall**: If `persistent_data: true`, data is kept. If false or absent, data is deleted
with the plugin. User can manually clear persistent data from the UI.

## Future

### Deep Link Protocol (`nexus://`)
Register `nexus://` URL scheme for one-click plugin installs from the browser.
`nexus://install/com.example.weather` opens Nexus with the install dialog pre-filled.

### P2P Agent Communication
AI agent plugins communicating across Nexus instances with bilateral consent,
Ed25519 identity, and message signing.
**Design doc**: `docs/arch/p2p-agent-communication.md`

### Global Shortcuts
System-wide hotkeys (e.g. `Cmd+Shift+N` to summon Nexus, custom shortcuts per MCP tool).

### Autostart on Login
Launch Nexus as a background service on OS login. Plugins stay alive, MCP tools
always available to Claude Code.

### Plugin Clipboard Access
Host API permission for clipboard read/write. Useful for password managers,
snippet tools, etc.
