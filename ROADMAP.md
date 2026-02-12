# Nexus Roadmap

## Platform & Core (Future Phases)

- [ ] Linux + Windows support
- [ ] Plugin hot-reload (restart container on image update)
- [ ] Plugin sandboxing levels (strict/standard/privileged)
- [ ] Plugin-to-plugin communication bus
- [ ] Plugin data volumes (persistent storage per plugin)
- [ ] Plugin resource limits (CPU/memory caps via Docker)
- [ ] Plugin auto-update (watch registry for new versions)
- [ ] Theming / dark mode / custom CSS for shell
- [ ] Keyboard shortcuts / command palette
- [ ] Notification center (aggregated plugin notifications)
- [ ] Multi-window support (pop plugins out into separate windows)
- [ ] Drag-and-drop plugin layout / dashboard grid
- [ ] Plugin settings UI framework (standardized settings schema)
- [ ] Audit log (track all Host API calls)
- [ ] Encrypted credential store for plugins
- [ ] Plugin signing / verification (trust chain)
- [ ] WebSocket channel for real-time plugin <-> host events
- [ ] CLI companion tool (`nexus install <plugin>`, `nexus start`, etc.)
- [ ] Plugin SDK / template generator

## Plugin Ideas (Community)

- **Docker Manager** — Full container management: create, start, stop, remove, logs, stats, compose support
- **Process Manager** — System process viewer, CPU/memory usage, kill processes
- **System Monitor** — Real-time CPU, memory, disk, network graphs
- **Terminal** — Embedded terminal emulator (xterm.js)
- **File Manager** — Browse, upload, download, quick-look files
- **Network Scanner** — Discover devices on LAN, port scanning, service detection
- **DNS Manager** — Manage local DNS entries, flush cache
- **SSH Manager** — Saved connections, key management, multi-tab SSH
- **Database Browser** — Connect to PostgreSQL, MySQL, SQLite, Redis — browse and query
- **API Client** — HTTP request builder/tester (Postman-like)
- **Log Viewer** — Aggregate and search logs from multiple sources
- **Git Dashboard** — Multi-repo status, branches, recent commits, PR status
- **CI/CD Monitor** — GitHub Actions / GitLab CI pipeline status
- **Cron Manager** — View and edit cron jobs, launchd agents
- **Backup Manager** — Scheduled backup orchestration with notifications
- **Media Encoder** — Video/audio transcoding queue
- **AI Chat** — Local LLM interface (ollama, llama.cpp)
- **AI Agent Builder** — Visual workflow for chaining LLM calls
- **Clipboard Manager** — Clipboard history, snippets, templates
- **Note Taking** — Quick markdown notes and code snippets
- **Service Health** — Uptime monitoring for URLs and services
- **VPN Manager** — WireGuard/OpenVPN connection management
- **Certificate Monitor** — Track TLS cert expiration dates
- **Screenshot Tool** — Capture, annotate, organize screenshots
- **Kanban Board** — Simple task/project management
- **Webhook Relay** — Receive and forward webhooks, inspect payloads
- **Port Manager** — See what's listening on which ports, kill listeners
- **Environment Manager** — Manage .env files, environment variables across projects
- **Package Updater** — Check for outdated brew/npm/cargo/pip packages
- **Disk Analyzer** — Visualize disk usage, find large files

## Contributing

Want to build a plugin? Check out the `plugins/hello-world` example for a starting point. Plugins are Docker containers that serve a web UI and communicate with the host through the Nexus Host API.
