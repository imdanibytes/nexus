# Plugin Authoring Guide

Build plugins for the Nexus desktop dashboard. Plugins run as Docker containers
and interact with the host through a REST API.

## Quick Start

```
my-plugin/
├── plugin.json          # Manifest (required)
├── Dockerfile           # Container definition (required)
└── src/
    ├── server.js        # HTTP server
    └── public/
        └── index.html   # Plugin UI
```

### 1. Create the Manifest

`plugin.json` declares your plugin's identity, Docker image, and permissions.

```json
{
  "id": "com.yourname.my-plugin",
  "name": "My Plugin",
  "version": "1.0.0",
  "description": "What this plugin does.",
  "author": "Your Name",
  "image": "my-plugin:latest",
  "ui": {
    "port": 80,
    "path": "/"
  },
  "permissions": ["system:info"],
  "health": {
    "endpoint": "/health",
    "interval_secs": 30
  },
  "env": {},
  "settings": []
}
```

**Required fields**: `id`, `name`, `version`, `description`, `author`, `image`,
`ui.port`.

**ID format**: Reverse-domain notation (`com.example.plugin-name`). Max 100
characters.

**Image**: Docker image reference. For local development, use `name:latest`. For
published plugins, use a registry path (`ghcr.io/user/plugin:1.0.0`).

See [Manifest Specification](spec/manifest-spec.md) for the full schema.

### 2. Write the Dockerfile

```dockerfile
FROM node:20-alpine

WORKDIR /app
COPY src/ ./
EXPOSE 80

CMD ["node", "server.js"]
```

Use any base image and language you want. The only requirement is an HTTP server
listening on the port declared in `ui.port`.

### 3. Implement the Server

Your server needs three things:

1. **Health endpoint** — returns 2xx when ready
2. **Config endpoint** — serves `NEXUS_TOKEN` and `NEXUS_API_URL` to your
   frontend
3. **Static file serving** — serves your UI

```javascript
const http = require("http");
const fs = require("fs");
const path = require("path");

const PORT = 80;
const NEXUS_TOKEN = process.env.NEXUS_TOKEN || "";
const NEXUS_API_URL =
  process.env.NEXUS_API_URL || "http://host.docker.internal:9600";

const server = http.createServer((req, res) => {
  // Health check
  if (req.url === "/health") {
    res.writeHead(200, { "Content-Type": "application/json" });
    return res.end(JSON.stringify({ status: "ok" }));
  }

  // Config endpoint — frontend retrieves auth credentials here
  if (req.url === "/api/config") {
    res.writeHead(200, { "Content-Type": "application/json" });
    return res.end(JSON.stringify({ token: NEXUS_TOKEN, apiUrl: NEXUS_API_URL }));
  }

  // Serve index.html with API URL templated in (for theme CSS link)
  if (req.url === "/" || req.url === "/index.html") {
    const html = fs.readFileSync(path.join(__dirname, "public/index.html"), "utf8")
      .replace(/\{\{NEXUS_API_URL\}\}/g, NEXUS_API_URL);
    res.writeHead(200, { "Content-Type": "text/html" });
    return res.end(html);
  }

  // Static files
  const file = path.join(__dirname, "public", req.url);
  fs.readFile(file, (err, data) => {
    if (err) { res.writeHead(404); return res.end("Not Found"); }
    res.writeHead(200);
    res.end(data);
  });
});

server.listen(PORT);
```

### 4. Build the UI

Your `index.html` is loaded inside an iframe in the Nexus shell. Use the shared
design system for visual consistency:

```html
<!DOCTYPE html>
<html>
<head>
  <!-- Load Nexus theme variables -->
  <link rel="stylesheet" href="{{NEXUS_API_URL}}/api/v1/theme.css" />
  <style>
    body {
      font-family: var(--font-sans);
      background: var(--color-nx-deep);
      color: var(--color-nx-text);
    }
  </style>
</head>
<body>
  <div id="app"></div>
  <script>
    async function init() {
      // Get auth credentials from your server
      const { token, apiUrl } = await fetch('/api/config').then(r => r.json());

      // Call the Host API
      const info = await fetch(`${apiUrl}/api/v1/system/info`, {
        headers: { 'Authorization': `Bearer ${token}` }
      }).then(r => r.json());

      document.getElementById('app').textContent = `Hostname: ${info.hostname}`;
    }
    init();
  </script>
</body>
</html>
```

**`{{NEXUS_API_URL}}`** is replaced by the server at serve time. Use it in the
`<link>` tag; for JS, fetch the URL from `/api/config` instead.

### 5. Install Locally

In Nexus, use the **Install from File** option and select your `plugin.json`.
Nexus will:

1. Read the manifest
2. Build the Docker image automatically (if it doesn't exist and a `Dockerfile`
   is next to the manifest)
3. Show the permission approval dialog
4. Create and start the container

No need to manually `docker build` during development.

---

## Environment Variables

Nexus injects these into every plugin container:

| Variable | Description |
|----------|-------------|
| `NEXUS_TOKEN` | Bearer token for Host API authentication |
| `NEXUS_API_URL` | Host API base URL (usually `http://localhost:9600`) |

Plus any custom variables declared in `manifest.env`.

**Never expose `NEXUS_TOKEN` in client-facing responses** other than through
your own authenticated config endpoint. The token grants access to everything
the user approved for your plugin.

---

## Permissions

Plugins run with zero access by default. Each Host API endpoint requires a
specific permission. Users approve permissions at install time.

| Permission | Risk | Grants Access To |
|-----------|------|-----------------|
| `system:info` | Low | `GET /api/v1/system/info` — OS, hostname, uptime |
| `filesystem:read` | Medium | `GET /api/v1/fs/read`, `GET /api/v1/fs/list` — read files and directories |
| `filesystem:write` | High | `POST /api/v1/fs/write` — create/overwrite files |
| `process:list` | Medium | `GET /api/v1/process/list` — running processes |
| `docker:read` | Medium | `GET /api/v1/docker/containers`, `GET /api/v1/docker/stats/{id}` |
| `docker:manage` | High | Start/stop/create containers |
| `network:local` | Medium | Proxy HTTP requests to LAN addresses |
| `network:internet` | Medium | Proxy HTTP requests to the internet |

**Request only what you need.** Users see risk levels during installation and
may deny high-risk permissions.

See [Permissions Reference](spec/permissions.md) for detailed behavior.

### Runtime Path Approval

Filesystem permissions use **runtime approval prompts**. Even after a user
grants `filesystem:read`, the plugin must be approved for each directory it
accesses. When the plugin requests a path not yet approved, Nexus pauses the
request and shows the user a dialog:

- **Allow** — grants access to the parent directory (persisted across restarts)
- **Allow Once** — grants access for this single request only
- **Deny** — returns 403 to the plugin

This means you don't need to know upfront which paths your plugin will access.
Just request `filesystem:read` in the manifest and let the user approve
directories at runtime.

---

## Host API Reference

Base URL: `http://localhost:9600` (from inside the container, use the
`NEXUS_API_URL` environment variable).

All authenticated endpoints require:
```
Authorization: Bearer <NEXUS_TOKEN>
```

### System

```
GET /api/v1/system/info
```
Returns: `{ os, os_version, hostname, uptime, cpu_count, total_memory, nexus_version }`

### Filesystem

```
GET  /api/v1/fs/read?path=/etc/hostname
GET  /api/v1/fs/list?path=/home/user
POST /api/v1/fs/write  { "path": "/tmp/out.txt", "content": "data" }
```

- Read limit: 5 MB per file
- Paths must be absolute
- The Nexus data directory is always blocked (contains auth tokens)
- Paths are canonicalized to prevent traversal attacks

### Process

```
GET /api/v1/process/list
```
Returns: array of `{ pid, name, cpu, memory }`

### Docker

```
GET /api/v1/docker/containers
GET /api/v1/docker/stats/{container_id}
```

### Network

```
POST /api/v1/network/proxy
{
  "url": "https://api.example.com/data",
  "method": "GET",
  "headers": { "Accept": "application/json" },
  "body": null
}
```

- 10 MB response limit
- SSRF protection: cloud metadata endpoints and Host API relay are blocked
- Redirect validation: public-to-private redirects are blocked

### Settings

```
GET /api/v1/settings
PUT /api/v1/settings  { "key": "value" }
```

Settings are scoped to your plugin. Define available settings in the manifest:

```json
{
  "settings": [
    {
      "key": "refresh_interval",
      "type": "number",
      "label": "Refresh Interval (seconds)",
      "description": "How often to poll for updates",
      "default": 30
    },
    {
      "key": "theme",
      "type": "select",
      "label": "Color Theme",
      "options": ["auto", "light", "dark"],
      "default": "auto"
    }
  ]
}
```

Setting types: `string`, `number`, `boolean`, `select`.

### OpenAPI Spec

```
GET /api/openapi.json
```

Full OpenAPI 3.0 specification for all endpoints. No authentication required.

---

## Design System

Link the theme CSS to inherit Nexus visual styling:

```html
<link rel="stylesheet" href="{{NEXUS_API_URL}}/api/v1/theme.css" />
```

### CSS Variables

**Colors**:
- `--color-nx-deep` — deepest background
- `--color-nx-base` — main content background
- `--color-nx-surface` — card/panel background
- `--color-nx-raised` — elevated surface (toasts, dialogs)
- `--color-nx-overlay` — subtle overlay/button background
- `--color-nx-wash` — hover state for overlays
- `--color-nx-text` — primary text
- `--color-nx-text-secondary` — secondary text
- `--color-nx-text-muted` — labels, hints
- `--color-nx-text-ghost` — placeholder text
- `--color-nx-accent` — primary accent color
- `--color-nx-accent-hover` — accent hover state
- `--color-nx-accent-muted` — accent background tint
- `--color-nx-border` — primary borders
- `--color-nx-border-subtle` — subtle dividers
- `--color-nx-border-accent` — accent-colored borders
- `--color-nx-success`, `--color-nx-warning`, `--color-nx-error` — status
  colors (each has a `-muted` variant)
- `--color-nx-info` — informational

**Typography**:
- `--font-sans` — UI text (Geist Sans)
- `--font-mono` — code and data (Geist Mono)

**Spacing**:
- `--radius-card` — card corner radius
- `--radius-button` — button/input corner radius
- `--radius-modal` — modal corner radius
- `--radius-tag` — badge/tag corner radius

**Shadows**:
- `--shadow-toast` — toast notification shadow
- `--shadow-modal` — modal dialog shadow

### Fonts

Theme fonts are served from the Host API:

```
GET /api/v1/theme/fonts/{filename}
```

The `theme.css` file already includes `@font-face` declarations, so you don't
need to load fonts manually.

---

## Container Security

Plugins run in sandboxed Docker containers with:

- **Dropped capabilities** — all Linux capabilities dropped except
  `NET_BIND_SERVICE`
- **No new privileges** — `no-new-privileges:true` secopt
- **No volume mounts** — no access to host filesystem (all file access goes
  through the Host API)
- **Bridge network** — containers communicate with the host via
  `host.docker.internal`, not direct network access
- **Resource limits** — CPU and memory quotas configurable by the user

Your plugin cannot:
- Access the host filesystem directly
- Escalate privileges
- Communicate with other plugin containers
- Access cloud metadata endpoints

---

## Development Tips

**Fast iteration**: Use local install (`Install from File`). Nexus auto-builds
the Docker image from your Dockerfile. After code changes, remove and reinstall
the plugin to rebuild.

**Debugging**: Use the browser DevTools to inspect your plugin's iframe.
Network requests to the Host API are visible in the Network tab.

**Logs**: Plugin container stdout/stderr is accessible from the Nexus UI
(plugin detail view > Logs tab) or via the Tauri command `plugin_logs`.

**Test permissions**: Install the `permission-tester` plugin
(`plugins/permission-tester/`) to exercise all Host API endpoints and verify
security boundaries.

**OpenAPI**: Hit `http://localhost:9600/api/openapi.json` for the full API spec.
Import it into Postman or similar tools for interactive testing.

---

## Publishing

To publish a plugin to a registry:

1. Push your Docker image to a container registry (Docker Hub, GHCR, etc.)
2. Host your `plugin.json` manifest at a public URL
3. Add the manifest URL to a Nexus registry's `registry.json`

Registry format:

```json
[
  {
    "manifest_url": "https://raw.githubusercontent.com/you/plugin/main/plugin.json",
    "name": "My Plugin",
    "description": "What it does",
    "author": "Your Name",
    "version": "1.0.0"
  }
]
```

Users can add custom registries in **Settings > Registries**.
