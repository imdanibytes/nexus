//! Built-in Nexus MCP tools and Extension MCP tools.

use std::sync::Arc;
use axum::http::StatusCode;
use serde_json::json;

use crate::host_api::approval::{ApprovalBridge, ApprovalDecision, ApprovalRequest};
use super::types::{McpCallResponse, McpContent, McpToolEntry};
use crate::extensions::RiskLevel;
use crate::plugin_manager::storage::McpPluginSettings;
use crate::AppState;

/// Virtual plugin ID for built-in tools.
pub const NEXUS_PLUGIN_ID: &str = "nexus";
pub const NEXUS_PLUGIN_NAME: &str = "Nexus";

// ---------------------------------------------------------------------------
// Tool catalog
// ---------------------------------------------------------------------------

pub fn builtin_tools() -> Vec<McpToolEntry> {
    vec![
        // -- Read-only tools --
        McpToolEntry {
            name: "nexus.list_plugins".into(),
            description: "List all installed Nexus plugins with their status, version, port, and dev mode flag. Use to check what plugins are available, their health status, or to find a plugin ID for other operations. Do NOT use to check if a specific plugin exists by name — scan the results instead of calling repeatedly.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
            plugin_id: NEXUS_PLUGIN_ID.into(),
            plugin_name: NEXUS_PLUGIN_NAME.into(),
            required_permissions: vec![],
            permissions_granted: true,
            enabled: true,
            requires_approval: false,
        },
        McpToolEntry {
            name: "nexus.plugin_logs".into(),
            description: "Get recent log lines from a plugin's Docker container. Use to debug plugin issues, check startup errors, or monitor runtime behavior. Do NOT use for general status checks — use list_plugins for that. Defaults to 100 lines; adjust tail for more or fewer.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "plugin_id": {
                        "type": "string",
                        "description": "The plugin ID to fetch logs for."
                    },
                    "tail": {
                        "type": "integer",
                        "description": "Number of recent lines to return (default: 100)."
                    }
                },
                "required": ["plugin_id"],
                "additionalProperties": false
            }),
            plugin_id: NEXUS_PLUGIN_ID.into(),
            plugin_name: NEXUS_PLUGIN_NAME.into(),
            required_permissions: vec![],
            permissions_granted: true,
            enabled: true,
            requires_approval: false,
        },
        McpToolEntry {
            name: "nexus.list_extensions".into(),
            description: "List all host extensions (native binaries) with their enabled/running status and available operations. Use to discover what extensions are installed or to check if a specific extension is running. Extensions are different from plugins — they run as native processes, not Docker containers.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
            plugin_id: NEXUS_PLUGIN_ID.into(),
            plugin_name: NEXUS_PLUGIN_NAME.into(),
            required_permissions: vec![],
            permissions_granted: true,
            enabled: true,
            requires_approval: false,
        },
        McpToolEntry {
            name: "nexus.search_marketplace".into(),
            description: "Search the Nexus marketplace for plugins and extensions by keyword. Use when the user wants to find or install new capabilities. Returns matching plugins and extensions with their manifest URLs for installation. Do NOT use to search for already-installed plugins — use list_plugins for that.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search query string."
                    }
                },
                "required": ["query"],
                "additionalProperties": false
            }),
            plugin_id: NEXUS_PLUGIN_ID.into(),
            plugin_name: NEXUS_PLUGIN_NAME.into(),
            required_permissions: vec![],
            permissions_granted: true,
            enabled: true,
            requires_approval: false,
        },
        McpToolEntry {
            name: "nexus.get_settings".into(),
            description: "Get Nexus app settings including CPU quota, memory limit, and update check interval. Use when you need to understand resource constraints or check configuration. Do NOT call unless settings are relevant to the current task.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
            plugin_id: NEXUS_PLUGIN_ID.into(),
            plugin_name: NEXUS_PLUGIN_NAME.into(),
            required_permissions: vec![],
            permissions_granted: true,
            enabled: true,
            requires_approval: false,
        },
        McpToolEntry {
            name: "nexus.get_mcp_settings".into(),
            description: "Get MCP gateway settings including the global enabled flag and per-plugin tool enable/disable states. Use to check which tools are active or to debug tool availability issues. Do NOT use for general status — this is specifically about MCP tool routing configuration.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
            plugin_id: NEXUS_PLUGIN_ID.into(),
            plugin_name: NEXUS_PLUGIN_NAME.into(),
            required_permissions: vec![],
            permissions_granted: true,
            enabled: true,
            requires_approval: false,
        },
        McpToolEntry {
            name: "nexus.engine_status".into(),
            description: "Check if the Docker/container engine is installed, running, and responsive. Use to diagnose plugin startup failures or verify the container runtime before operations. Do NOT call routinely — only when container operations are failing or during setup troubleshooting.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
            plugin_id: NEXUS_PLUGIN_ID.into(),
            plugin_name: NEXUS_PLUGIN_NAME.into(),
            required_permissions: vec![],
            permissions_granted: true,
            enabled: true,
            requires_approval: false,
        },
        // -- Mutating tools --
        McpToolEntry {
            name: "nexus.plugin_start".into(),
            description: "Start a stopped Nexus plugin by its plugin ID. Use when the user explicitly asks to start a plugin or when a plugin needs to be running for a task. Do NOT start plugins speculatively — only when there's a clear need. Requires user approval.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "plugin_id": {
                        "type": "string",
                        "description": "The plugin ID to start."
                    }
                },
                "required": ["plugin_id"],
                "additionalProperties": false
            }),
            plugin_id: NEXUS_PLUGIN_ID.into(),
            plugin_name: NEXUS_PLUGIN_NAME.into(),
            required_permissions: vec![],
            permissions_granted: true,
            enabled: true,
            requires_approval: true,
        },
        McpToolEntry {
            name: "nexus.plugin_stop".into(),
            description: "Stop a running Nexus plugin. Use when the user asks to stop a plugin or when a plugin needs to be restarted. Do NOT stop plugins without the user's intent — stopping removes the plugin's tools from the MCP gateway. Requires user approval.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "plugin_id": {
                        "type": "string",
                        "description": "The plugin ID to stop."
                    }
                },
                "required": ["plugin_id"],
                "additionalProperties": false
            }),
            plugin_id: NEXUS_PLUGIN_ID.into(),
            plugin_name: NEXUS_PLUGIN_NAME.into(),
            required_permissions: vec![],
            permissions_granted: true,
            enabled: true,
            requires_approval: true,
        },
        McpToolEntry {
            name: "nexus.plugin_remove".into(),
            description: "Permanently remove an installed plugin, including its Docker container and configuration. Stops the plugin first if running. Use ONLY when the user explicitly asks to uninstall a plugin. This is destructive — plugin data in Docker volumes may be lost. Requires user approval.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "plugin_id": {
                        "type": "string",
                        "description": "The plugin ID to remove."
                    }
                },
                "required": ["plugin_id"],
                "additionalProperties": false
            }),
            plugin_id: NEXUS_PLUGIN_ID.into(),
            plugin_name: NEXUS_PLUGIN_NAME.into(),
            required_permissions: vec![],
            permissions_granted: true,
            enabled: true,
            requires_approval: true,
        },
        McpToolEntry {
            name: "nexus.plugin_install".into(),
            description: "Install a new Nexus plugin from a registry manifest URL. Use when the user wants to add new functionality via a plugin. The plugin starts with no permissions — the user must approve them through the UI after installation. Do NOT fabricate manifest URLs — use search_marketplace to find valid ones. Requires user approval.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "manifest_url": {
                        "type": "string",
                        "description": "URL of the plugin manifest JSON."
                    }
                },
                "required": ["manifest_url"],
                "additionalProperties": false
            }),
            plugin_id: NEXUS_PLUGIN_ID.into(),
            plugin_name: NEXUS_PLUGIN_NAME.into(),
            required_permissions: vec![],
            permissions_granted: true,
            enabled: true,
            requires_approval: true,
        },
        McpToolEntry {
            name: "nexus.extension_enable".into(),
            description: "Enable a host extension by spawning its native process and registering its operations. Use when the user wants to activate an installed extension. Do NOT enable extensions speculatively. Extensions run as native processes on the host, not in containers. Requires user approval.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "ext_id": {
                        "type": "string",
                        "description": "The extension ID to enable."
                    }
                },
                "required": ["ext_id"],
                "additionalProperties": false
            }),
            plugin_id: NEXUS_PLUGIN_ID.into(),
            plugin_name: NEXUS_PLUGIN_NAME.into(),
            required_permissions: vec![],
            permissions_granted: true,
            enabled: true,
            requires_approval: true,
        },
        McpToolEntry {
            name: "nexus.extension_disable".into(),
            description: "Disable a host extension by stopping its process and unregistering its operations. Use when the user wants to deactivate an extension. Do NOT disable extensions that are actively being used by other tools. Requires user approval.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "ext_id": {
                        "type": "string",
                        "description": "The extension ID to disable."
                    }
                },
                "required": ["ext_id"],
                "additionalProperties": false
            }),
            plugin_id: NEXUS_PLUGIN_ID.into(),
            plugin_name: NEXUS_PLUGIN_NAME.into(),
            required_permissions: vec![],
            permissions_granted: true,
            enabled: true,
            requires_approval: true,
        },
        McpToolEntry {
            name: "nexus.plugin_install_local".into(),
            description: "Install or reinstall a plugin from a local manifest path. Builds the Docker image automatically if a Dockerfile exists in the manifest directory. Preserves data volumes and permissions across reinstalls. Auto-restarts the plugin if it was previously running. Use for local plugin development — avoids the remove/reinstall cycle. Requires user approval.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "manifest_path": {
                        "type": "string",
                        "description": "Absolute path to the plugin manifest JSON file (plugin.json)."
                    }
                },
                "required": ["manifest_path"],
                "additionalProperties": false
            }),
            plugin_id: NEXUS_PLUGIN_ID.into(),
            plugin_name: NEXUS_PLUGIN_NAME.into(),
            required_permissions: vec![],
            permissions_granted: true,
            enabled: true,
            requires_approval: true,
        },
        McpToolEntry {
            name: "nexus.extension_install_local".into(),
            description: "Install or reinstall an extension from a local manifest path. Builds the binary with cargo if Cargo.toml exists and no precompiled binary matches the current platform. Auto-re-enables the extension if it was previously running. Use for local extension development — avoids the remove/reinstall cycle. Requires user approval.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "manifest_path": {
                        "type": "string",
                        "description": "Absolute path to the extension manifest JSON file (manifest.json)."
                    }
                },
                "required": ["manifest_path"],
                "additionalProperties": false
            }),
            plugin_id: NEXUS_PLUGIN_ID.into(),
            plugin_name: NEXUS_PLUGIN_NAME.into(),
            required_permissions: vec![],
            permissions_granted: true,
            enabled: true,
            requires_approval: true,
        },
        // -- Nexus Code tools --
        McpToolEntry {
            name: "nexus.read_file".into(),
            description: "Read a file from the host filesystem as text. Use to examine source code, config files, or any text file needed for the current task. Do NOT use for binary files (images, compiled code) — only text content is returned. Files over 5 MB are rejected. Requires an absolute path. The Nexus data directory is blocked for security.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Absolute path to the file to read."
                    }
                },
                "required": ["path"],
                "additionalProperties": false
            }),
            plugin_id: NEXUS_PLUGIN_ID.into(),
            plugin_name: NEXUS_PLUGIN_NAME.into(),
            required_permissions: vec![],
            permissions_granted: true,
            enabled: true,
            requires_approval: false,
        },
        McpToolEntry {
            name: "nexus.write_file".into(),
            description: "Write content to a file on the host filesystem. Creates parent directories automatically. Use for creating new files or replacing entire file contents. Do NOT use for partial edits — use edit_file for find-and-replace operations instead. Overwrites the entire file. Requires an absolute path. The Nexus data directory is blocked for security.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Absolute path to the file to write."
                    },
                    "content": {
                        "type": "string",
                        "description": "The content to write to the file."
                    }
                },
                "required": ["path", "content"],
                "additionalProperties": false
            }),
            plugin_id: NEXUS_PLUGIN_ID.into(),
            plugin_name: NEXUS_PLUGIN_NAME.into(),
            required_permissions: vec![],
            permissions_granted: true,
            enabled: true,
            requires_approval: false,
        },
        McpToolEntry {
            name: "nexus.edit_file".into(),
            description: "Perform an atomic find-and-replace in a file. Use for targeted edits to existing files — modifying functions, updating config values, fixing bugs. Preferred over write_file when changing part of a file. Do NOT use when you need to rewrite most of the file — use write_file instead. The old_string must exist and be unique unless replace_all is true. Read the file first to ensure your old_string matches exactly.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Absolute path to the file to edit."
                    },
                    "old_string": {
                        "type": "string",
                        "description": "The text to find in the file."
                    },
                    "new_string": {
                        "type": "string",
                        "description": "The text to replace it with."
                    },
                    "replace_all": {
                        "type": "boolean",
                        "description": "Replace all occurrences (default: false, requires unique match)."
                    }
                },
                "required": ["path", "old_string", "new_string"],
                "additionalProperties": false
            }),
            plugin_id: NEXUS_PLUGIN_ID.into(),
            plugin_name: NEXUS_PLUGIN_NAME.into(),
            required_permissions: vec![],
            permissions_granted: true,
            enabled: true,
            requires_approval: false,
        },
        McpToolEntry {
            name: "nexus.list_directory".into(),
            description: "List the contents of a directory on the host filesystem. Returns file names, paths, sizes, and whether each entry is a directory. Use to explore project structure or verify a path exists before reading/writing. Do NOT use recursively to scan large trees — use nexus.search_files with a glob pattern instead.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Absolute path to the directory to list."
                    }
                },
                "required": ["path"],
                "additionalProperties": false
            }),
            plugin_id: NEXUS_PLUGIN_ID.into(),
            plugin_name: NEXUS_PLUGIN_NAME.into(),
            required_permissions: vec![],
            permissions_granted: true,
            enabled: true,
            requires_approval: false,
        },
        McpToolEntry {
            name: "nexus.search_files".into(),
            description: "Search for files matching a glob pattern. Searches recursively from the given base directory. Returns up to 1000 matching file paths. Use when you need to find files by name or extension across a project (e.g. '**/*.ts', 'src/**/*.test.*'). Do NOT use for searching file contents — use nexus.search_content for that. Do NOT use for listing a single known directory — use nexus.list_directory instead.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "Glob pattern (e.g. \"**/*.ts\", \"src/**/*.rs\", \"*.json\")."
                    },
                    "path": {
                        "type": "string",
                        "description": "Absolute path to the base directory to search from."
                    }
                },
                "required": ["pattern", "path"],
                "additionalProperties": false
            }),
            plugin_id: NEXUS_PLUGIN_ID.into(),
            plugin_name: NEXUS_PLUGIN_NAME.into(),
            required_permissions: vec![],
            permissions_granted: true,
            enabled: true,
            requires_approval: false,
        },
        McpToolEntry {
            name: "nexus.search_content".into(),
            description: "Search file contents for a regex pattern (like grep/ripgrep). Walks the directory tree, skipping hidden dirs, node_modules, target, __pycache__, and dist. Returns matching lines with file paths and line numbers. Use to find function definitions, imports, usage patterns, or any text inside files. Use the 'include' parameter to narrow by file type. Do NOT use for finding files by name — use nexus.search_files instead. Do NOT use on very broad directories without an include filter — results will be noisy.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "Regex pattern to search for."
                    },
                    "path": {
                        "type": "string",
                        "description": "Absolute path to the file or directory to search in."
                    },
                    "include": {
                        "type": "string",
                        "description": "Optional glob filter for file names (e.g. \"*.ts\", \"*.rs\")."
                    },
                    "context_lines": {
                        "type": "integer",
                        "description": "Number of context lines around matches (default: 0)."
                    },
                    "max_results": {
                        "type": "integer",
                        "description": "Maximum number of matching files to return (default: 50)."
                    }
                },
                "required": ["pattern", "path"],
                "additionalProperties": false
            }),
            plugin_id: NEXUS_PLUGIN_ID.into(),
            plugin_name: NEXUS_PLUGIN_NAME.into(),
            required_permissions: vec![],
            permissions_granted: true,
            enabled: true,
            requires_approval: false,
        },
        McpToolEntry {
            name: "nexus.fetch_url".into(),
            description: "Fetch content from a URL via HTTP. Returns the response status, headers, and body. Use for retrieving web pages, calling REST APIs, or checking endpoint availability. Supports GET, POST, PUT, PATCH, DELETE, and HEAD methods. Response bodies larger than 512 KB are truncated. Timeout is 30 seconds (max 60). Only http:// and https:// URLs are supported.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "The URL to fetch (must be http:// or https://)."
                    },
                    "method": {
                        "type": "string",
                        "enum": ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD"],
                        "description": "HTTP method (default: GET)."
                    },
                    "headers": {
                        "type": "object",
                        "description": "Request headers as key-value pairs."
                    },
                    "body": {
                        "type": "string",
                        "description": "Request body (for POST/PUT/PATCH)."
                    },
                    "timeout_secs": {
                        "type": "integer",
                        "description": "Timeout in seconds (default: 30, max: 60)."
                    }
                },
                "required": ["url"],
                "additionalProperties": false
            }),
            plugin_id: NEXUS_PLUGIN_ID.into(),
            plugin_name: NEXUS_PLUGIN_NAME.into(),
            required_permissions: vec![],
            permissions_granted: true,
            enabled: true,
            requires_approval: false,
        },
        McpToolEntry {
            name: "nexus.directory_tree".into(),
            description: "Show a directory tree structure. Skips hidden directories, node_modules, target, __pycache__, and dist by default. Max depth is 6.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Absolute path to the root directory."
                    },
                    "depth": {
                        "type": "integer",
                        "description": "Maximum recursion depth (default: 3, max: 6)."
                    }
                },
                "required": ["path"],
                "additionalProperties": false
            }),
            plugin_id: NEXUS_PLUGIN_ID.into(),
            plugin_name: NEXUS_PLUGIN_NAME.into(),
            required_permissions: vec![],
            permissions_granted: true,
            enabled: true,
            requires_approval: false,
        },
        McpToolEntry {
            name: "nexus.execute_command".into(),
            description: "Execute a command on the host system. Returns stdout, stderr, and exit code. Every invocation requires explicit user approval. Timeout default is 30s (max 600s).".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The command to execute (e.g. \"git\", \"cargo\", \"ls\")."
                    },
                    "args": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Arguments to pass to the command."
                    },
                    "working_dir": {
                        "type": "string",
                        "description": "Absolute path to the working directory."
                    },
                    "timeout_secs": {
                        "type": "integer",
                        "description": "Timeout in seconds (default: 30, max: 60)."
                    }
                },
                "required": ["command"],
                "additionalProperties": false
            }),
            plugin_id: NEXUS_PLUGIN_ID.into(),
            plugin_name: NEXUS_PLUGIN_NAME.into(),
            required_permissions: vec![],
            permissions_granted: true,
            enabled: true,
            requires_approval: true,
        },
    ]
}

// ---------------------------------------------------------------------------
// Call dispatch
// ---------------------------------------------------------------------------

pub async fn handle_call(
    tool_name: &str,
    arguments: &serde_json::Value,
    state: &AppState,
    bridge: &Arc<ApprovalBridge>,
) -> Result<McpCallResponse, StatusCode> {
    match tool_name {
        "list_plugins" => handle_list_plugins(state).await,
        "plugin_logs" => handle_plugin_logs(arguments, state).await,
        "list_extensions" => handle_list_extensions(state).await,
        "search_marketplace" => handle_search_marketplace(arguments, state).await,
        "get_settings" => handle_get_settings(state).await,
        "get_mcp_settings" => handle_get_mcp_settings(state).await,
        "engine_status" => handle_engine_status(state).await,
        "read_file" => handle_read_file(arguments, state).await,
        "write_file" => handle_write_file(arguments, state).await,
        "edit_file" => handle_edit_file(arguments, state).await,
        "list_directory" => handle_list_directory(arguments, state).await,
        "search_files" => handle_search_files(arguments, state).await,
        "search_content" => handle_search_content(arguments, state).await,
        "fetch_url" => handle_fetch_url(arguments).await,
        "directory_tree" => handle_directory_tree(arguments, state).await,
        "execute_command" | "plugin_start" | "plugin_stop" | "plugin_remove"
        | "plugin_install" | "plugin_install_local" | "extension_enable"
        | "extension_disable" | "extension_install_local" => {
            handle_mutating(tool_name, arguments, state, bridge).await
        }
        _ => Err(StatusCode::NOT_FOUND),
    }
}

// ---------------------------------------------------------------------------
// Read-only handlers
// ---------------------------------------------------------------------------

async fn handle_list_plugins(state: &AppState) -> Result<McpCallResponse, StatusCode> {
    let mgr = state.read().await;
    let plugins: Vec<serde_json::Value> = mgr.storage.list().iter().map(|p| {
        json!({
            "id": p.manifest.id,
            "name": p.manifest.name,
            "version": p.manifest.version,
            "status": p.status,
            "port": p.assigned_port,
            "dev_mode": p.dev_mode,
            "description": p.manifest.description,
        })
    }).collect();
    ok_json(&plugins)
}

async fn handle_plugin_logs(args: &serde_json::Value, state: &AppState) -> Result<McpCallResponse, StatusCode> {
    let plugin_id = require_str(args, "plugin_id")?;
    let tail = args.get("tail").and_then(|v| v.as_u64()).unwrap_or(100) as u32;
    let mgr = state.read().await;
    match mgr.logs(&plugin_id, tail).await {
        Ok(lines) => ok_json(&lines),
        Err(e) => ok_error(format!("Failed to get logs for '{}': {}", plugin_id, e)),
    }
}

async fn handle_list_extensions(state: &AppState) -> Result<McpCallResponse, StatusCode> {
    let mgr = state.read().await;
    let mut extensions = Vec::new();
    for ext_info in mgr.extensions.list() {
        let installed_ext = mgr.extension_loader.storage.get(&ext_info.id);
        extensions.push(json!({
            "id": ext_info.id,
            "display_name": ext_info.display_name,
            "description": ext_info.description,
            "installed": installed_ext.is_some(),
            "enabled": installed_ext.is_some_and(|e| e.enabled),
            "operations": ext_info.operations.iter().map(|op| json!({
                "name": op.name,
                "description": op.description,
                "risk_level": op.risk_level,
            })).collect::<Vec<_>>(),
        }));
    }
    for installed in mgr.extension_loader.storage.list() {
        if !installed.enabled && !extensions.iter().any(|e| e["id"] == installed.manifest.id) {
            extensions.push(json!({
                "id": installed.manifest.id,
                "display_name": installed.manifest.display_name,
                "description": installed.manifest.description,
                "installed": true,
                "enabled": false,
                "operations": installed.manifest.operations.iter().map(|op| json!({
                    "name": op.name,
                    "description": op.description,
                    "risk_level": op.risk_level,
                })).collect::<Vec<_>>(),
            }));
        }
    }
    ok_json(&extensions)
}

async fn handle_search_marketplace(args: &serde_json::Value, state: &AppState) -> Result<McpCallResponse, StatusCode> {
    let query = require_str(args, "query")?;
    let mgr = state.read().await;
    let plugins = mgr.search_marketplace(&query);
    let extensions = mgr.search_extension_marketplace(&query);
    ok_json(&json!({
        "plugins": plugins.iter().map(|p| json!({ "id": p.id, "name": p.name, "version": p.version, "description": p.description, "manifest_url": p.manifest_url, "categories": p.categories })).collect::<Vec<_>>(),
        "extensions": extensions.iter().map(|e| json!({ "id": e.id, "name": e.name, "version": e.version, "description": e.description, "manifest_url": e.manifest_url, "categories": e.categories })).collect::<Vec<_>>(),
    }))
}

async fn handle_get_settings(state: &AppState) -> Result<McpCallResponse, StatusCode> {
    let mgr = state.read().await;
    ok_json(&json!({
        "cpu_quota_percent": mgr.settings.cpu_quota_percent,
        "memory_limit_mb": mgr.settings.memory_limit_mb,
        "update_check_interval_minutes": mgr.settings.update_check_interval_minutes,
    }))
}

async fn handle_get_mcp_settings(state: &AppState) -> Result<McpCallResponse, StatusCode> {
    let mgr = state.read().await;
    ok_json(&mgr.mcp_settings)
}

async fn handle_engine_status(state: &AppState) -> Result<McpCallResponse, StatusCode> {
    let runtime = { state.read().await.runtime.clone() };
    let engine_id = runtime.engine_id().to_string();
    let socket = runtime.socket_path();
    match tokio::time::timeout(std::time::Duration::from_secs(3), runtime.ping()).await {
        Ok(Ok(_)) => {
            let version = runtime.version().await.unwrap_or(None);
            ok_json(&json!({ "engine_id": engine_id, "installed": true, "running": true, "version": version, "socket": socket }))
        }
        Ok(Err(e)) => ok_json(&json!({ "engine_id": engine_id, "installed": true, "running": false, "socket": socket, "message": format!("Container engine not responding: {}", e) })),
        Err(_) => ok_json(&json!({ "engine_id": engine_id, "installed": true, "running": false, "socket": socket, "message": "Container engine connection timed out" })),
    }
}

// ---------------------------------------------------------------------------
// Mutating handlers (with approval)
// ---------------------------------------------------------------------------

async fn handle_mutating(
    tool_name: &str,
    arguments: &serde_json::Value,
    state: &AppState,
    bridge: &Arc<ApprovalBridge>,
) -> Result<McpCallResponse, StatusCode> {
    let already_approved = {
        let mgr = state.read().await;
        mgr.mcp_settings.plugins.get(NEXUS_PLUGIN_ID).is_some_and(|s| s.approved_tools.contains(&tool_name.to_string()))
    };

    if !already_approved {
        let mut context = std::collections::HashMap::new();
        context.insert("tool_name".to_string(), tool_name.to_string());
        context.insert("plugin_name".to_string(), NEXUS_PLUGIN_NAME.to_string());
        context.insert("description".to_string(), describe_mutating_tool(tool_name));
        if let serde_json::Value::Object(map) = arguments {
            for (k, v) in map {
                let display = match v { serde_json::Value::String(s) => s.clone(), other => other.to_string() };
                context.insert(format!("arg.{}", k), display);
            }
        }
        let approval_req = ApprovalRequest {
            id: uuid::Uuid::new_v4().to_string(),
            plugin_id: NEXUS_PLUGIN_ID.to_string(),
            plugin_name: NEXUS_PLUGIN_NAME.to_string(),
            category: "mcp_tool".to_string(),
            permission: format!("mcp:nexus:{}", tool_name),
            context,
        };
        match bridge.request_approval(approval_req).await {
            ApprovalDecision::Approve => {
                let mut mgr = state.write().await;
                let plugin_settings = mgr.mcp_settings.plugins.entry(NEXUS_PLUGIN_ID.to_string()).or_insert_with(McpPluginSettings::default);
                if !plugin_settings.approved_tools.contains(&tool_name.to_string()) {
                    plugin_settings.approved_tools.push(tool_name.to_string());
                }
                let _ = mgr.mcp_settings.save();
                log::info!("AUDIT Nexus MCP tool permanently approved: tool={}", tool_name);
            }
            ApprovalDecision::ApproveOnce => { log::info!("AUDIT Nexus MCP tool approved once: tool={}", tool_name); }
            ApprovalDecision::Deny => {
                log::warn!("AUDIT Nexus MCP tool denied: tool={}", tool_name);
                return ok_error(format!("[Nexus] Tool 'nexus.{}' was denied by the user.", tool_name));
            }
        }
    }

    match tool_name {
        "execute_command" => exec_execute_command(arguments, state).await,
        "plugin_start" => exec_plugin_start(arguments, state).await,
        "plugin_stop" => exec_plugin_stop(arguments, state).await,
        "plugin_remove" => exec_plugin_remove(arguments, state).await,
        "plugin_install" => exec_plugin_install(arguments, state).await,
        "extension_enable" => exec_extension_enable(arguments, state).await,
        "extension_disable" => exec_extension_disable(arguments, state).await,
        "plugin_install_local" => exec_plugin_install_local(arguments, state).await,
        "extension_install_local" => exec_extension_install_local(arguments, state).await,
        _ => Err(StatusCode::NOT_FOUND),
    }
}

fn describe_mutating_tool(tool_name: &str) -> String {
    match tool_name {
        "execute_command" => "Execute a command on the host system".into(),
        "plugin_start" => "Start a stopped plugin".into(),
        "plugin_stop" => "Stop a running plugin".into(),
        "plugin_remove" => "Remove an installed plugin".into(),
        "plugin_install" => "Install a plugin from a registry manifest URL".into(),
        "extension_enable" => "Enable a host extension".into(),
        "extension_disable" => "Disable a host extension".into(),
        "plugin_install_local" => "Install a plugin from a local manifest path".into(),
        "extension_install_local" => "Install an extension from a local manifest path".into(),
        _ => tool_name.to_string(),
    }
}

async fn exec_plugin_start(args: &serde_json::Value, state: &AppState) -> Result<McpCallResponse, StatusCode> {
    let plugin_id = require_str(args, "plugin_id")?;
    let mut mgr = state.write().await;
    match mgr.start(&plugin_id).await {
        Ok(()) => {
            mgr.notify_tools_changed();
            ok_json(&json!({ "status": "started", "plugin_id": plugin_id }))
        }
        Err(e) => ok_error(format!("Failed to start '{}': {}", plugin_id, e)),
    }
}

async fn exec_plugin_stop(args: &serde_json::Value, state: &AppState) -> Result<McpCallResponse, StatusCode> {
    let plugin_id = require_str(args, "plugin_id")?;
    let mut mgr = state.write().await;
    match mgr.stop(&plugin_id).await {
        Ok(()) => {
            mgr.notify_tools_changed();
            ok_json(&json!({ "status": "stopped", "plugin_id": plugin_id }))
        }
        Err(e) => ok_error(format!("Failed to stop '{}': {}", plugin_id, e)),
    }
}

async fn exec_plugin_remove(args: &serde_json::Value, state: &AppState) -> Result<McpCallResponse, StatusCode> {
    let plugin_id = require_str(args, "plugin_id")?;
    let mut mgr = state.write().await;
    match mgr.remove(&plugin_id).await {
        Ok(()) => {
            mgr.notify_tools_changed();
            ok_json(&json!({ "status": "removed", "plugin_id": plugin_id }))
        }
        Err(e) => ok_error(format!("Failed to remove '{}': {}", plugin_id, e)),
    }
}

async fn exec_plugin_install(args: &serde_json::Value, state: &AppState) -> Result<McpCallResponse, StatusCode> {
    let manifest_url = require_str(args, "manifest_url")?;
    let manifest = match crate::plugin_manager::registry::fetch_manifest(&manifest_url).await {
        Ok(m) => m,
        Err(e) => return ok_error(format!("Failed to fetch manifest: {}", e)),
    };
    if let Err(e) = manifest.validate() { return ok_error(format!("Invalid manifest: {}", e)); }
    let plugin_id = manifest.id.clone();
    let mut mgr = state.write().await;
    match mgr.install(manifest, vec![], vec![], Some(&manifest_url), None).await {
        Ok(_) => {
            mgr.notify_tools_changed();
            ok_json(&json!({ "status": "installed", "plugin_id": plugin_id }))
        }
        Err(e) => ok_error(format!("Failed to install '{}': {}", plugin_id, e)),
    }
}

async fn exec_extension_enable(args: &serde_json::Value, state: &AppState) -> Result<McpCallResponse, StatusCode> {
    let ext_id = require_str(args, "ext_id")?;
    let mut mgr = state.write().await;
    match mgr.enable_extension(&ext_id) {
        Ok(()) => {
            if let Some(ext) = mgr.extensions.get(&ext_id) {
                let mcp_ops: Vec<String> = ext.operations().iter().filter(|op| op.mcp_expose).map(|op| op.name.clone()).collect();
                if !mcp_ops.is_empty() {
                    let settings = mgr.mcp_settings.plugins.entry(ext_id.clone()).or_insert_with(McpPluginSettings::default);
                    for op in mcp_ops { if !settings.enabled_tools.contains(&op) { settings.enabled_tools.push(op); } }
                    let _ = mgr.mcp_settings.save();
                }
            }
            mgr.notify_tools_changed();
            ok_json(&json!({ "status": "enabled", "ext_id": ext_id }))
        }
        Err(e) => ok_error(format!("Failed to enable '{}': {}", ext_id, e)),
    }
}

async fn exec_extension_disable(args: &serde_json::Value, state: &AppState) -> Result<McpCallResponse, StatusCode> {
    let ext_id = require_str(args, "ext_id")?;
    let mut mgr = state.write().await;
    match mgr.disable_extension(&ext_id) {
        Ok(()) => {
            mgr.notify_tools_changed();
            ok_json(&json!({ "status": "disabled", "ext_id": ext_id }))
        }
        Err(e) => ok_error(format!("Failed to disable '{}': {}", ext_id, e)),
    }
}

async fn exec_plugin_install_local(args: &serde_json::Value, state: &AppState) -> Result<McpCallResponse, StatusCode> {
    let manifest_path = require_str(args, "manifest_path")?;
    let data = std::fs::read_to_string(&manifest_path).map_err(|_| StatusCode::BAD_REQUEST)?;
    let manifest: crate::plugin_manager::manifest::PluginManifest = serde_json::from_str(&data).map_err(|_| StatusCode::BAD_REQUEST)?;
    if let Err(e) = manifest.validate() { return ok_error(format!("Invalid manifest: {}", e)); }
    let plugin_id = manifest.id.clone();
    let manifest_dir = std::path::Path::new(&manifest_path).parent().ok_or(StatusCode::BAD_REQUEST)?;
    if manifest_dir.join("Dockerfile").exists() {
        let runtime = { state.read().await.runtime.clone() };
        if let Err(e) = runtime.build_image(manifest_dir, &manifest.image).await { return ok_error(format!("Docker build failed: {}", e)); }
    }
    let was_running = { let mgr = state.read().await; mgr.storage.get(&plugin_id).is_some_and(|p| matches!(p.status, crate::plugin_manager::storage::PluginStatus::Running)) };
    let mut mgr = state.write().await;
    if let Err(e) = mgr.install(manifest, vec![], vec![], None, Some(manifest_path.clone())).await {
        return ok_error(format!("Failed to install '{}': {}", plugin_id, e));
    }
    if was_running { let _ = mgr.start(&plugin_id).await; }
    mgr.notify_tools_changed();
    ok_json(&json!({ "status": "installed", "plugin_id": plugin_id }))
}

async fn exec_extension_install_local(args: &serde_json::Value, state: &AppState) -> Result<McpCallResponse, StatusCode> {
    let manifest_path_str = require_str(args, "manifest_path")?;
    let manifest_path = std::path::Path::new(&manifest_path_str);
    let manifest_data = std::fs::read_to_string(manifest_path).map_err(|_| StatusCode::BAD_REQUEST)?;
    let manifest: crate::extensions::manifest::ExtensionManifest = serde_json::from_str(&manifest_data).map_err(|_| StatusCode::BAD_REQUEST)?;
    let ext_id = manifest.id.clone();
    let manifest_dir = manifest_path.parent().unwrap_or(std::path::Path::new("."));
    let binary_override = if manifest.binary_for_current_platform().is_none() && manifest_dir.join("Cargo.toml").exists() {
        match crate::extensions::loader::cargo_build_extension(manifest_dir).await { Ok(path) => Some(path), Err(e) => return ok_error(format!("Build failed: {}", e)) }
    } else { None };
    let was_enabled = { let mgr = state.read().await; mgr.extension_loader.storage.get(&ext_id).is_some_and(|e| e.enabled) };
    { let mut mgr = state.write().await; if let Err(e) = mgr.install_extension_local(manifest_path, binary_override.as_deref()) {
        return ok_error(format!("Failed to install '{}': {}", ext_id, e));
    } }
    if was_enabled { let mut mgr = state.write().await; let _ = mgr.enable_extension(&ext_id); }
    state.read().await.notify_tools_changed();
    ok_json(&json!({ "status": "installed", "ext_id": ext_id }))
}

// ---------------------------------------------------------------------------
// Nexus Code handlers
// ---------------------------------------------------------------------------

async fn handle_read_file(args: &serde_json::Value, state: &AppState) -> Result<McpCallResponse, StatusCode> {
    let path = require_str(args, "path")?;
    let mgr = state.read().await;
    let canonical = std::path::PathBuf::from(&path).canonicalize().map_err(|_| StatusCode::NOT_FOUND)?;
    if canonical.starts_with(&mgr.data_dir) { return ok_error("Access to Nexus data directory is blocked".into()); }
    if !canonical.is_file() { return ok_error(format!("'{}' is not a file", path)); }
    let metadata = std::fs::metadata(&canonical).map_err(|_| StatusCode::NOT_FOUND)?;
    if metadata.len() > 5 * 1024 * 1024 { return ok_error(format!("File too large ({} bytes, max 5 MB)", metadata.len())); }
    match std::fs::read_to_string(&canonical) {
        Ok(content) => ok_json(&json!({ "path": canonical.to_string_lossy(), "content": content, "size": metadata.len() })),
        Err(e) => ok_error(format!("Failed to read '{}': {}", path, e)),
    }
}

async fn handle_write_file(args: &serde_json::Value, state: &AppState) -> Result<McpCallResponse, StatusCode> {
    let path = require_str(args, "path")?;
    let content = require_str(args, "content")?;
    let target = std::path::PathBuf::from(&path);
    if !target.is_absolute() { return ok_error("Path must be absolute".into()); }
    let normalized = crate::host_api::filesystem::normalize_path(&target);
    let mgr = state.read().await;
    if normalized.starts_with(&mgr.data_dir) { return ok_error("Access to Nexus data directory is blocked".into()); }
    if let Some(parent) = normalized.parent() { if let Err(e) = std::fs::create_dir_all(parent) { return ok_error(format!("Failed to create parent directories: {}", e)); } }
    match std::fs::write(&normalized, &content) {
        Ok(()) => ok_json(&json!({ "path": normalized.to_string_lossy(), "bytes_written": content.len() })),
        Err(e) => ok_error(format!("Failed to write '{}': {}", path, e)),
    }
}

async fn handle_edit_file(args: &serde_json::Value, state: &AppState) -> Result<McpCallResponse, StatusCode> {
    let path = require_str(args, "path")?;
    let old_string = require_str(args, "old_string")?;
    let new_string = require_str(args, "new_string")?;
    let replace_all = args.get("replace_all").and_then(|v| v.as_bool()).unwrap_or(false);
    let canonical = std::path::PathBuf::from(&path).canonicalize().map_err(|_| StatusCode::NOT_FOUND)?;
    let mgr = state.read().await;
    if canonical.starts_with(&mgr.data_dir) { return ok_error("Access to Nexus data directory is blocked".into()); }
    if !canonical.is_file() { return ok_error(format!("'{}' is not a file", path)); }
    if old_string == new_string { return ok_error("old_string and new_string must be different".into()); }
    let content = std::fs::read_to_string(&canonical).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let count = content.matches(&old_string).count();
    if count == 0 { return ok_error("old_string not found in file".into()); }
    let new_content = if replace_all { content.replace(&old_string, &new_string) } else {
        if count > 1 { return ok_error(format!("old_string found {} times — must be unique", count)); }
        content.replacen(&old_string, &new_string, 1)
    };
    match std::fs::write(&canonical, &new_content) {
        Ok(()) => ok_json(&json!({ "path": canonical.to_string_lossy(), "replacements": if replace_all { count } else { 1 } })),
        Err(e) => ok_error(format!("Failed to write '{}': {}", path, e)),
    }
}

async fn handle_list_directory(args: &serde_json::Value, state: &AppState) -> Result<McpCallResponse, StatusCode> {
    let path = require_str(args, "path")?;
    let canonical = std::path::PathBuf::from(&path).canonicalize().map_err(|_| StatusCode::NOT_FOUND)?;
    let mgr = state.read().await;
    if canonical.starts_with(&mgr.data_dir) { return ok_error("Access to Nexus data directory is blocked".into()); }
    if !canonical.is_dir() { return ok_error(format!("'{}' is not a directory", path)); }
    let entries: Vec<serde_json::Value> = match std::fs::read_dir(&canonical) {
        Ok(rd) => rd.flatten().map(|entry| {
            let metadata = entry.metadata().ok();
            json!({ "name": entry.file_name().to_string_lossy(), "path": entry.path().to_string_lossy(), "is_dir": metadata.as_ref().map(|m| m.is_dir()).unwrap_or(false), "size": metadata.as_ref().map(|m| m.len()).unwrap_or(0) })
        }).collect(),
        Err(e) => return ok_error(format!("Failed to read directory: {}", e)),
    };
    ok_json(&json!({ "path": canonical.to_string_lossy(), "entries": entries }))
}

async fn handle_search_files(args: &serde_json::Value, state: &AppState) -> Result<McpCallResponse, StatusCode> {
    let pattern = require_str(args, "pattern")?;
    let path = require_str(args, "path")?;
    let canonical = std::path::PathBuf::from(&path).canonicalize().map_err(|_| StatusCode::NOT_FOUND)?;
    let mgr = state.read().await;
    if canonical.starts_with(&mgr.data_dir) { return ok_error("Access to Nexus data directory is blocked".into()); }
    let full_pattern = canonical.join(&pattern).to_string_lossy().to_string();
    let mut matches = Vec::new();
    match glob::glob(&full_pattern) {
        Ok(paths) => {
            for entry in paths {
                if let Ok(p) = entry {
                    matches.push(p.to_string_lossy().to_string());
                }
                if matches.len() >= 1000 {
                    break;
                }
            }
        }
        Err(e) => return ok_error(format!("Invalid glob pattern: {}", e)),
    }
    ok_json(&json!({ "pattern": pattern, "base_path": canonical.to_string_lossy(), "matches": matches, "truncated": matches.len() >= 1000 }))
}

async fn handle_search_content(args: &serde_json::Value, state: &AppState) -> Result<McpCallResponse, StatusCode> {
    let pattern = require_str(args, "pattern")?;
    let path = require_str(args, "path")?;
    let include = args.get("include").and_then(|v| v.as_str()).map(String::from);
    let context_lines = args.get("context_lines").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
    let max_results = args.get("max_results").and_then(|v| v.as_u64()).unwrap_or(50) as usize;
    let search_path = std::path::PathBuf::from(&path).canonicalize().map_err(|_| StatusCode::NOT_FOUND)?;
    let mgr = state.read().await;
    if search_path.starts_with(&mgr.data_dir) { return ok_error("Access to Nexus data directory is blocked".into()); }
    let re = match regex::Regex::new(&pattern) { Ok(r) => r, Err(e) => return ok_error(format!("Invalid regex: {}", e)) };
    let include_glob = include.as_ref().and_then(|g| glob::Pattern::new(g).ok());
    let mut file_matches = Vec::new();
    if search_path.is_file() {
        if let Some(m) = crate::host_api::filesystem::grep_single_file(&search_path, &re, context_lines) {
            file_matches.push(json!({ "path": m.path, "lines": m.lines.iter().map(|l| json!({ "line_number": l.line_number, "content": l.content, "is_context": l.is_context })).collect::<Vec<_>>() }));
        }
    } else {
        for entry in walkdir::WalkDir::new(&search_path).follow_links(false).into_iter().filter_entry(|e| { let name = e.file_name().to_string_lossy(); !name.starts_with('.') && name != "node_modules" && name != "target" && name != "__pycache__" && name != "dist" }) {
            let entry = match entry { Ok(e) => e, Err(_) => continue };
            if !entry.file_type().is_file() { continue; }
            if let Some(ref pat) = include_glob { if !pat.matches(&entry.file_name().to_string_lossy()) { continue; } }
            if let Some(m) = crate::host_api::filesystem::grep_single_file(entry.path(), &re, context_lines) {
                file_matches.push(json!({ "path": m.path, "lines": m.lines.iter().map(|l| json!({ "line_number": l.line_number, "content": l.content, "is_context": l.is_context })).collect::<Vec<_>>() }));
                if file_matches.len() >= max_results { break; }
            }
        }
    }
    ok_json(&json!({ "pattern": pattern, "search_path": search_path.to_string_lossy(), "matches": file_matches }))
}

async fn handle_fetch_url(args: &serde_json::Value) -> Result<McpCallResponse, StatusCode> {
    let url_str = require_str(args, "url")?;
    let method = args.get("method").and_then(|v| v.as_str()).unwrap_or("GET").to_uppercase();
    let url = match reqwest::Url::parse(&url_str) { Ok(u) => u, Err(e) => return ok_error(format!("Invalid URL: {}", e)) };
    if url.scheme() != "http" && url.scheme() != "https" { return ok_error("Only http/https supported".into()); }
    let client = reqwest::Client::new();
    let mut rb = match method.as_str() { "POST" => client.post(url), "PUT" => client.put(url), "PATCH" => client.patch(url), "DELETE" => client.delete(url), "HEAD" => client.head(url), _ => client.get(url) };
    if let Some(headers) = args.get("headers").and_then(|v| v.as_object()) { for (k, v) in headers { if let Some(s) = v.as_str() { rb = rb.header(k, s); } } }
    if let Some(body) = args.get("body").and_then(|v| v.as_str()) { rb = rb.body(body.to_string()); }
    match rb.send().await {
        Ok(resp) => {
            let status = resp.status().as_u16();
            let ct = resp.headers().get("content-type").and_then(|v| v.to_str().ok()).unwrap_or("").to_string();
            let body = resp.text().await.unwrap_or_default();
            ok_json(&json!({ "status": status, "content_type": ct, "body": if ct.contains("html") { htmd::convert(&body).unwrap_or(body) } else { body } }))
        }
        Err(e) => ok_error(format!("Fetch failed: {}", e))
    }
}

async fn handle_directory_tree(args: &serde_json::Value, state: &AppState) -> Result<McpCallResponse, StatusCode> {
    let path = require_str(args, "path")?;
    let depth = args.get("depth").and_then(|v| v.as_u64()).unwrap_or(3).min(6) as usize;
    let canonical = std::path::PathBuf::from(&path).canonicalize().map_err(|_| StatusCode::NOT_FOUND)?;
    let mgr = state.read().await;
    if canonical.starts_with(&mgr.data_dir) { return ok_error("Access to Nexus data directory is blocked".into()); }
    let mut lines = vec![canonical.to_string_lossy().to_string()];
    build_tree(&canonical, "", depth, &mut lines);
    ok_json(&json!({ "tree": lines.join("\n") }))
}

fn build_tree(dir: &std::path::Path, prefix: &str, depth: usize, lines: &mut Vec<String>) {
    let mut entries: Vec<_> = match std::fs::read_dir(dir) { Ok(rd) => rd.flatten().collect(), Err(_) => return };
    entries.sort_by_key(|e| (e.metadata().map(|m| !m.is_dir()).unwrap_or(true), e.file_name()));
    let entries: Vec<_> = entries.into_iter().filter(|e| !e.file_name().to_string_lossy().starts_with('.')).collect();
    let total = entries.len();
    for (i, entry) in entries.iter().enumerate() {
        let is_last = i == total - 1;
        let name = entry.file_name().to_string_lossy().to_string();
        let is_dir = entry.metadata().map(|m| m.is_dir()).unwrap_or(false);
        lines.push(format!("{}{}{} ", prefix, if is_last { "└── " } else { "├── " }, if is_dir { format!("{}/", name) } else { name.clone() }));
        if is_dir && depth > 1 && !["node_modules", "target", "dist", ".git"].contains(&name.as_str()) {
            build_tree(&entry.path(), &format!("{}{}", prefix, if is_last { "    " } else { "│   " }), depth - 1, lines);
        }
    }
}

async fn exec_execute_command(args: &serde_json::Value, _state: &AppState) -> Result<McpCallResponse, StatusCode> {
    let command = require_str(args, "command")?;
    let cmd_args: Vec<String> = args.get("args").and_then(|v| v.as_array()).map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect()).unwrap_or_default();
    let mut cmd = tokio::process::Command::new(&command);
    cmd.args(&cmd_args);
    cmd.stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped());
    if let Some(wd) = args.get("working_dir").and_then(|v| v.as_str()) { cmd.current_dir(wd); }
    match cmd.spawn() {
        Ok(child) => {
            let out = child.wait_with_output().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            ok_json(&json!({ "exit_code": out.status.code(), "stdout": String::from_utf8_lossy(&out.stdout), "stderr": String::from_utf8_lossy(&out.stderr) }))
        }
        Err(e) => ok_error(format!("Failed to spawn: {}", e)),
    }
}

// ---------------------------------------------------------------------------
// Extension tools
// ---------------------------------------------------------------------------

pub async fn extension_mcp_tools(state: &AppState) -> Vec<McpToolEntry> {
    let mgr = state.read().await;
    let mut tools = Vec::new();
    for ext_info in mgr.extensions.list() {
        if !mgr.extension_loader.storage.get(&ext_info.id).is_some_and(|e| e.enabled) { continue; }
        for op in &ext_info.operations {
            if !op.mcp_expose { continue; }
            tools.push(McpToolEntry {
                name: format!("{}.{}", ext_info.id, op.name),
                description: op.mcp_description.clone().unwrap_or(op.description.clone()),
                input_schema: op.input_schema.clone(),
                plugin_id: ext_info.id.clone(),
                plugin_name: ext_info.display_name.clone(),
                required_permissions: vec![],
                permissions_granted: true,
                enabled: true,
                requires_approval: matches!(op.risk_level, RiskLevel::Medium | RiskLevel::High),
            });
        }
    }
    tools
}

pub async fn handle_extension_call(ext_id: &str, operation: &str, arguments: &serde_json::Value, state: &AppState, bridge: &Arc<ApprovalBridge>) -> Result<McpCallResponse, StatusCode> {
    let (ext_arc, op_def) = {
        let mgr = state.read().await;
        let ext = mgr.extensions.get_arc(ext_id).ok_or(StatusCode::NOT_FOUND)?;
        let op = ext.operations().into_iter().find(|o| o.name == operation && o.mcp_expose).ok_or(StatusCode::NOT_FOUND)?;
        (ext, op)
    };
    if matches!(op_def.risk_level, RiskLevel::Medium | RiskLevel::High) {
        let already_approved = { let mgr = state.read().await; mgr.mcp_settings.plugins.get(ext_id).is_some_and(|s| s.approved_tools.contains(&operation.to_string())) };
        if !already_approved {
            let mut context = std::collections::HashMap::new();
            context.insert("tool_name".to_string(), format!("{}.{}", ext_id, operation));
            context.insert("plugin_name".to_string(), ext_id.to_string());
            context.insert("description".to_string(), op_def.description.clone());
            let approval_req = ApprovalRequest { id: uuid::Uuid::new_v4().to_string(), plugin_id: ext_id.to_string(), plugin_name: ext_id.to_string(), category: "mcp_tool".to_string(), permission: format!("mcp:{}:{}", ext_id, operation), context };
            match bridge.request_approval(approval_req).await {
                ApprovalDecision::Approve => {
                    let mut mgr = state.write().await;
                    let s = mgr.mcp_settings.plugins.entry(ext_id.to_string()).or_insert_with(McpPluginSettings::default);
                    if !s.approved_tools.contains(&operation.to_string()) { s.approved_tools.push(operation.to_string()); }
                    let _ = mgr.mcp_settings.save();
                }
                ApprovalDecision::ApproveOnce => {}
                ApprovalDecision::Deny => return ok_error("Denied by user".to_string())
            }
        }
    }
    match ext_arc.execute(operation, arguments.clone()).await {
        Ok(result) => {
            if result.success { ok_json(&result.data) }
            else { ok_error(result.message.unwrap_or_else(|| "Failed".into())) }
        }
        Err(e) => ok_error(format!("Extension error: {}", e)),
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn require_str(args: &serde_json::Value, key: &str) -> Result<String, StatusCode> {
    args.get(key).and_then(|v| v.as_str()).map(|s| s.to_string()).ok_or(StatusCode::BAD_REQUEST)
}

fn ok_json<T: serde::Serialize>(value: &T) -> Result<McpCallResponse, StatusCode> {
    let text = serde_json::to_string_pretty(value).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(McpCallResponse { content: vec![McpContent { content_type: "text".into(), text }], is_error: false })
}

fn ok_error(message: String) -> Result<McpCallResponse, StatusCode> {
    Ok(McpCallResponse { content: vec![McpContent { content_type: "text".into(), text: message }], is_error: true })
}
