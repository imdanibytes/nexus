# Permissions

Nexus uses a permission system to control what Host API endpoints a plugin can
access. Permissions are declared in the plugin manifest and must be explicitly
approved by the user during installation. They can be revoked at any time from
**Settings > Permissions**.

## How Permissions Work

1. A plugin declares the permissions it needs in `plugin.json` under the
   `permissions` array.
2. At install time, the user sees a two-step dialog: plugin info first, then
   each requested permission with its risk level.
3. Only permissions the user approves are granted. The plugin receives no
   access beyond what was approved.
4. At runtime, every Host API request is checked against the plugin's granted
   permissions. Unauthorized calls return `403 Forbidden`.
5. Permissions can be revoked per-plugin in the Settings UI. Revocation takes
   effect immediately.

## Permission Reference

### `system:info`

| | |
|-|-|
| **Risk** | Low |
| **Grants access to** | `GET /api/v1/system/info` |
| **What it does** | Read basic host system information: OS name, version, architecture, hostname, uptime, CPU count, and total memory. |
| **What it does NOT do** | Does not expose running processes, usernames, IP addresses, or any identifying information beyond the OS profile. |
| **Typical use case** | Plugins that adapt behavior based on the host OS or display system stats. |

---

### `filesystem:read`

| | |
|-|-|
| **Risk** | Medium |
| **Grants access to** | `GET /api/v1/fs/read`, `GET /api/v1/fs/list` |
| **What it does** | Read file contents and list directory entries on the host filesystem. |
| **Constraints** | Paths are canonicalized to prevent `../` traversal. Access to the Nexus data directory (which contains auth tokens and permissions) is always blocked. When `approved_paths` are configured, access is restricted to those directories only. File reads are capped at 5 MB. |
| **What it does NOT do** | Does not allow writing, deleting, or modifying files. Does not grant access to the Nexus internal data directory regardless of path. |
| **Typical use case** | Plugins that read configuration files, log files, or project files on the host. |

---

### `filesystem:write`

| | |
|-|-|
| **Risk** | High |
| **Grants access to** | `POST /api/v1/fs/write` |
| **What it does** | Write content to files on the host filesystem. Creates parent directories if they don't exist. |
| **Constraints** | Paths are normalized to prevent traversal. The Nexus data directory is always blocked. When `approved_paths` are configured, writes are restricted to those directories only. Paths must be absolute. Request body is capped at 5 MB. |
| **What it does NOT do** | Does not grant read access (requires `filesystem:read` separately). Does not allow deleting files. |
| **Typical use case** | Plugins that generate reports, export data, or write configuration files. |

---

### `process:list`

| | |
|-|-|
| **Risk** | Medium |
| **Grants access to** | `GET /api/v1/process/list` |
| **What it does** | List running processes on the host: PID, name, CPU usage, and memory usage. |
| **What it does NOT do** | Does not allow killing, starting, or signaling processes. Does not expose process command lines, environment variables, or open file descriptors. |
| **Typical use case** | System monitoring plugins, resource dashboards. |

---

### `docker:read`

| | |
|-|-|
| **Risk** | Medium |
| **Grants access to** | `GET /api/v1/docker/containers`, `GET /api/v1/docker/stats/{id}` |
| **What it does** | List Nexus-managed Docker containers (filtered by `nexus.plugin.id` label) and read per-container CPU/memory stats. |
| **What it does NOT do** | Does not expose non-Nexus containers. Does not allow starting, stopping, creating, or removing containers. Does not expose container logs, environment variables, or volume mounts. |
| **Typical use case** | Dashboard plugins that show container health or resource usage. |

---

### `docker:manage`

| | |
|-|-|
| **Risk** | High |
| **Grants access to** | All `docker:read` endpoints plus container lifecycle operations (start, stop, create, remove). |
| **What it does** | Full management of Docker containers within the Nexus bridge network. |
| **What it does NOT do** | Containers created by plugins are still subject to Nexus security hardening (cap_drop ALL, no-new-privileges, no volume mounts, resource limits). Does not grant access to the Docker daemon directly. |
| **Typical use case** | Orchestration plugins that manage helper containers or sidecar services. |

---

### `network:local`

| | |
|-|-|
| **Risk** | Medium |
| **Grants access to** | `POST /api/v1/network/proxy` (for local/private network destinations) |
| **What it does** | Make HTTP/HTTPS requests to private network addresses: `127.x.x.x`, `10.x.x.x`, `172.16-31.x.x`, `192.168.x.x`, `localhost`, `.local`, and `.internal` domains. |
| **Constraints** | Requests to the Nexus Host API itself (`localhost:9600`) are always blocked to prevent relay attacks. Cloud metadata endpoints (`169.254.169.254`, `metadata.google.internal`) are always blocked. Only `http` and `https` schemes are allowed. Responses are capped at 10 MB with a 30-second timeout. |
| **What it does NOT do** | Does not allow requests to public internet addresses (requires `network:internet`). Does not allow raw TCP/UDP sockets, only HTTP proxying. |
| **Typical use case** | Plugins that integrate with local services (Home Assistant, NAS APIs, local databases). |

---

### `network:internet`

| | |
|-|-|
| **Risk** | Medium |
| **Grants access to** | `POST /api/v1/network/proxy` (for public internet destinations) |
| **What it does** | Make HTTP/HTTPS requests to public internet addresses. |
| **Constraints** | Same safeguards as `network:local`: metadata IPs blocked, Host API relay blocked, http/https only, 10 MB response cap, 30-second timeout, 5 redirect limit. |
| **What it does NOT do** | Does not allow requests to private/local network addresses (requires `network:local`). Does not allow raw sockets or non-HTTP protocols. |
| **Typical use case** | Plugins that fetch data from public APIs, check for updates, or sync with cloud services. |

---

## Settings (No Permission Required)

Every plugin can read and write its **own** settings without any permission:

| Endpoint | Description |
|----------|-------------|
| `GET /api/v1/settings` | Read the plugin's own settings (defaults from manifest overlaid with saved values) |
| `PUT /api/v1/settings` | Update the plugin's own settings (validated against manifest schema) |

Settings are siloed by auth token. A plugin can only access its own settings,
never another plugin's. No permission is required because this is considered
the plugin's own data.

## `approved_paths`

The `filesystem:read` and `filesystem:write` permissions support an optional
`approved_paths` constraint. When set, file access is restricted to only the
listed directories and their children.

Currently, `approved_paths` is set to `null` (unrestricted within the
permission) at install time. A future update will add a path picker to the
permission dialog, allowing users to scope filesystem access to specific
directories.

Regardless of `approved_paths`, the Nexus data directory is **always blocked**
for all plugins. This directory contains auth tokens, permissions, and plugin
state.

## Revoking Permissions

Permissions can be revoked from **Settings > Permissions** in the Nexus shell.
Each plugin shows an expandable list of its granted permissions with a revoke
button per permission. Revocation is immediate and persisted to disk. The
plugin will receive `403 Forbidden` on its next API call to the revoked
endpoint.

## For Plugin Authors

Declare only the permissions your plugin actually needs. Users see the risk
level of each permission during install, and requesting unnecessary high-risk
permissions will reduce trust and installation rates.

```json
{
  "permissions": [
    "system:info",
    "network:internet"
  ]
}
```

If your plugin works without a permission, don't declare it. Permissions can
always be re-requested in a future version (the user will see the new
permissions on update).
