# Plugin Manifest Specification

Reference for plugin authors and the Nexus registry. Manifests are validated
both client-side (when installing) and should be validated server-side by the
registry before publishing.

## Schema

```jsonc
{
  // REQUIRED — unique reverse-domain identifier
  "id": "com.example.my-plugin",

  // REQUIRED — human-readable display name
  "name": "My Plugin",

  // REQUIRED — semver version string
  "version": "1.0.0",

  // REQUIRED — short description shown in marketplace and install dialog
  "description": "Does something useful.",

  // REQUIRED — plugin author name
  "author": "Example Corp",

  // REQUIRED — Docker image reference (pulled at install time)
  "image": "ghcr.io/example/my-plugin:1.0.0",

  // REQUIRED — UI configuration
  "ui": {
    "port": 3000,       // Container port the plugin's web UI listens on
    "path": "/"         // Optional, defaults to "/"
  },

  // Optional — license identifier (SPDX)
  "license": "MIT",

  // Optional — project homepage URL (displayed in install dialog)
  "homepage": "https://example.com/my-plugin",

  // Optional — icon URL (must be http or https)
  "icon": "https://example.com/icon.png",

  // Optional — permissions the plugin requests
  "permissions": [
    "system:info",
    "filesystem:read",
    "network:internet"
  ],

  // Optional — health check configuration
  "health": {
    "endpoint": "/healthz",
    "interval_secs": 30    // Default: 30
  },

  // Optional — extra environment variables injected into the container
  "env": {
    "MY_CONFIG": "value"
  },

  // Optional — minimum Nexus version required
  "min_nexus_version": "0.2.0",

  // Optional — MCP tools exposed to AI assistants
  "mcp": {
    "tools": [
      {
        "name": "tool_name",
        "description": "What this tool does.",
        "permissions": [],
        "input_schema": {
          "type": "object",
          "properties": {},
          "required": []
        }
      }
    ]
  },

  // Optional — configurable settings (rendered as forms in Nexus shell)
  "settings": [
    {
      "key": "refresh_interval",
      "type": "number",
      "label": "Refresh Interval (s)",
      "description": "How often to poll for updates",
      "default": 30
    },
    {
      "key": "theme",
      "type": "select",
      "label": "Theme",
      "options": ["light", "dark", "auto"],
      "default": "auto"
    },
    {
      "key": "show_notifications",
      "type": "boolean",
      "label": "Show Notifications",
      "default": true
    },
    {
      "key": "api_key",
      "type": "string",
      "label": "API Key"
    }
  ]
}
```

## Validation Rules

These are enforced by Nexus at install time. The registry should enforce the
same rules at publish time to give authors early feedback.

### Required Fields

| Field | Constraint |
|-------|-----------|
| `id` | Non-empty, max 100 characters |
| `name` | Non-empty, max 100 characters |
| `version` | Non-empty, max 50 characters |
| `description` | Non-empty, max 2000 characters |
| `author` | Non-empty, max 100 characters |
| `image` | Non-empty, max 200 characters |
| `ui.port` | Must be non-zero (1-65535) |

### Content Restrictions

| Rule | Fields Affected | Reason |
|------|----------------|--------|
| No Unicode bidirectional override characters | `name`, `description`, `author` | Prevents display spoofing (e.g., making "malware" appear as "safe-app") |
| Icon must be `http://` or `https://` URL | `icon` | Prevents `javascript:` or `data:` URI injection |

### Bidirectional Characters Blocked

The following Unicode code points are rejected in display fields:

- U+200E, U+200F (LRM, RLM)
- U+202A–U+202E (LRE, RLE, PDF, LRO, RLO)
- U+2066–U+2069 (LRI, RLI, FSI, PDI)

### MCP Tools Schema

Each entry in the `mcp.tools` array must have:

| Field | Required | Type | Description |
|-------|----------|------|-------------|
| `name` | Yes | string | Tool name (used in `tools/call` requests) |
| `description` | Yes | string | What the tool does (shown to AI assistants) |
| `permissions` | No | string[] | Permissions required to call this tool |
| `input_schema` | Yes | object | JSON Schema for the tool's input arguments |

The `input_schema` must be a valid JSON Schema object with `type: "object"`.
Nexus routes `tools/call` MCP requests to `POST /mcp/call` on the plugin's
container, passing `{ "tool_name": "...", "arguments": { ... } }`.

### Settings Schema

Each entry in the `settings` array must have:

| Field | Required | Type | Description |
|-------|----------|------|-------------|
| `key` | Yes | string | Unique key within the plugin |
| `type` | Yes | `"string"` \| `"number"` \| `"boolean"` \| `"select"` | Value type |
| `label` | Yes | string | Human-readable label for the form field |
| `description` | No | string | Help text shown below the field |
| `default` | No | matches `type` | Default value |
| `options` | Required for `select` | string[] | Allowed values for select fields |

Settings values are validated on write:
- Only keys declared in the manifest are accepted
- Value types must match the declared `type`
- Select values must be one of the declared `options`

## Permissions

Available permissions that can be requested:

| Permission | Risk | Description |
|-----------|------|-------------|
| `system:info` | Low | Read OS info, hostname, uptime |
| `filesystem:read` | Medium | Read files on approved paths |
| `filesystem:write` | High | Write files to approved paths |
| `process:list` | Medium | List running processes |
| `docker:read` | Medium | List containers, read stats |
| `docker:manage` | High | Start/stop/create containers |
| `network:local` | Medium | HTTP requests to LAN |
| `network:internet` | Medium | HTTP requests to internet |

Permissions are **not auto-granted**. The user sees a two-step dialog
(plugin info, then permissions) and must explicitly approve each permission
before installation proceeds.

## Docker Container Constraints

Nexus applies the following security hardening to all plugin containers:

- `cap_drop: ALL` — all Linux capabilities dropped
- `cap_add: NET_BIND_SERVICE` — only low-port binding allowed
- `security_opt: no-new-privileges:true` — no privilege escalation
- No host volume mounts (binds explicitly empty)
- Resource limits applied from user settings (CPU + memory)
- Bound to `nexus-bridge` Docker network
- Port mapped to `127.0.0.1` only (not exposed to LAN)

## Environment Variables Injected by Nexus

These are automatically set on the container and should not be overridden
in the manifest's `env` field:

| Variable | Description |
|----------|-------------|
| `NEXUS_PLUGIN_SECRET` | Plugin secret for exchanging for a short-lived access token via `POST /api/v1/auth/token` |
| `NEXUS_API_URL` | Base URL for the Host API from the browser (e.g., `http://localhost:9600`) |
| `NEXUS_HOST_URL` | Base URL for the Host API from inside the container (e.g., `http://host.docker.internal:9600`) |
