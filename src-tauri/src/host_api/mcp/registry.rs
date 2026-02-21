//! Unified MCP tool registry and routing.
//!
//! The Registry is the central dispatch for all Model Context Protocol (MCP)
//! operations in Nexus. It aggregates capabilities from four distinct sources:
//!
//! 1. **Built-in tools**: Host-level management tools (prefixed with `nexus.`).
//! 2. **Extension tools**: Operations exposed by native binary extensions.
//! 3. **Native MCP plugins**: Plugins running their own MCP servers in containers.
//! 4. **Legacy HTTP plugins**: Deprecated plugins using the custom Nexus HTTP MCP protocol.
//!
//! ### Namespacing
//! To avoid collisions, tools and prompts are namespaced as `{plugin_id}.{name}`.
//! For example, `com.nexus.hello-world.greet`. Built-in tools use the `nexus`
//! namespace (e.g., `nexus.list_plugins`).

use std::sync::Arc;
use std::borrow::Cow;
use std::collections::HashMap;
use rmcp::model::*;
use rmcp::ErrorData as McpError;
use crate::AppState;
use super::builtin;
use crate::audit::writer::AuditWriter;
use crate::plugin_manager::storage::{PluginStatus, McpPluginSettings};
use crate::host_api::approval::{ApprovalBridge, ApprovalDecision, ApprovalRequest};
use crate::permissions::Permission;

pub struct McpRegistry {
    state: AppState,
    approval_bridge: Arc<ApprovalBridge>,
    audit: AuditWriter,
}

impl McpRegistry {
    pub fn new(state: AppState, approval_bridge: Arc<ApprovalBridge>, audit: AuditWriter) -> Self {
        Self { state, approval_bridge, audit }
    }

    /// Aggregates all available tools from all providers.
    ///
    /// This filtering follows a whitelist model:
    /// - The plugin must be healthy and running.
    /// - All host-level permissions for the plugin must be granted.
    /// - The specific tool must be enabled in the Nexus MCP settings.
    pub async fn list_tools(&self) -> Vec<Tool> {
        let mgr = self.state.read().await;
        let mut tools = Vec::new();
        if !mgr.mcp_settings.enabled { return tools; }

        // 1. Built-in tools (e.g. nexus.list_plugins)
        for tool_def in builtin::builtin_tools() {
            let local_name = tool_def.name.strip_prefix("nexus.").unwrap_or(&tool_def.name);
            if mgr.mcp_settings.plugins.get("nexus").is_some_and(|s| s.enabled && s.enabled_tools.contains(&local_name.to_string())) {
                tools.push(Tool {
                    name: Cow::Owned(tool_def.name.clone()),
                    title: None, description: Some(Cow::Owned(tool_def.description.clone())),
                    input_schema: Arc::new(match &tool_def.input_schema { serde_json::Value::Object(map) => map.clone(), _ => serde_json::Map::new() }),
                    output_schema: None, annotations: None, execution: None, icons: None, meta: None,
                });
            }
        }

        // 2. Extension tools (from native binaries)
        for ext_info in mgr.extensions.list() {
            if !mgr.extension_loader.storage.get(&ext_info.id).is_some_and(|e| e.enabled) { continue; }
            let ext_mcp = mgr.mcp_settings.plugins.get(&ext_info.id);
            if !ext_mcp.is_some_and(|s| s.enabled) { continue; }
            for op in &ext_info.operations {
                if !op.mcp_expose || !ext_mcp.is_some_and(|s| s.enabled_tools.contains(&op.name)) { continue; }
                tools.push(Tool {
                    name: Cow::Owned(format!("{}.{}", ext_info.id, op.name)),
                    title: None, description: Some(Cow::Owned(op.mcp_description.clone().unwrap_or(op.description.clone()))),
                    input_schema: Arc::new(match &op.input_schema { serde_json::Value::Object(map) => map.clone(), _ => serde_json::Map::new() }),
                    output_schema: None, annotations: None, icons: None, execution: None, meta: None,
                });
            }
        }

        // 3. Plugin tools (Native and Legacy)
        for plugin in mgr.storage.list() {
            if plugin.status != PluginStatus::Running { continue; }
            let plugin_id = &plugin.manifest.id;
            let plugin_mcp = mgr.mcp_settings.plugins.get(plugin_id);
            if !plugin_mcp.is_some_and(|s| s.enabled) { continue; }
            if !plugin.manifest.permissions.iter().all(|perm| mgr.permissions.has_permission(plugin_id, perm)) { continue; }

            if let Some(cache) = mgr.mcp_clients.get(plugin_id) {
                // Native MCP (preferred)
                for tool in &cache.tools {
                    if plugin_mcp.is_some_and(|s| s.enabled_tools.contains(&tool.name.to_string())) {
                        let mut t = tool.clone();
                        t.name = Cow::Owned(format!("{}.{}", plugin_id, tool.name));
                        tools.push(t);
                    }
                }
            } else if let Some(mcp_config) = &plugin.manifest.mcp {
                // Legacy HTTP protocol
                for tool_def in &mcp_config.tools {
                    if plugin_mcp.is_some_and(|s| s.enabled_tools.contains(&tool_def.name)) {
                        tools.push(Tool {
                            name: Cow::Owned(format!("{}.{}", plugin_id, tool_def.name)),
                            title: None, description: Some(Cow::Owned(tool_def.description.clone())),
                            input_schema: Arc::new(match &tool_def.input_schema { serde_json::Value::Object(map) => map.clone(), _ => serde_json::Map::new() }),
                            output_schema: None, annotations: None, icons: None, execution: None, meta: None,
                        });
                    }
                }
            }
        }
        tools
    }

    /// List resources across all plugins.
    pub async fn list_resources(&self) -> Vec<Resource> {
        let mgr = self.state.read().await;
        let mut resources = Vec::new();
        if !mgr.mcp_settings.enabled { return resources; }
        for (plugin_id, cache) in mgr.mcp_clients.iter() {
            let plugin_mcp = mgr.mcp_settings.plugins.get(plugin_id);
            if !plugin_mcp.is_some_and(|s| s.enabled) { continue; }
            if let Some(plugin) = mgr.storage.get(plugin_id) {
                if !plugin.manifest.permissions.iter().all(|perm| mgr.permissions.has_permission(plugin_id, perm)) { continue; }
            }
            for res in &cache.resources {
                if !plugin_mcp.is_some_and(|s| s.disabled_resources.contains(&res.uri.to_string())) {
                    resources.push(res.clone());
                }
            }
        }
        resources
    }

    pub async fn list_resource_templates(&self) -> Vec<ResourceTemplate> {
        let mgr = self.state.read().await;
        let mut templates = Vec::new();
        if !mgr.mcp_settings.enabled { return templates; }
        for (plugin_id, cache) in mgr.mcp_clients.iter() {
            if !mgr.mcp_settings.plugins.get(plugin_id).is_some_and(|s| s.enabled) { continue; }
            for t in &cache.resource_templates { templates.push(t.clone()); }
        }
        templates
    }

    pub async fn list_prompts(&self) -> Vec<Prompt> {
        let mgr = self.state.read().await;
        let mut prompts = Vec::new();
        if !mgr.mcp_settings.enabled { return prompts; }
        for (plugin_id, cache) in mgr.mcp_clients.iter() {
            let plugin_mcp = mgr.mcp_settings.plugins.get(plugin_id);
            if !plugin_mcp.is_some_and(|s| s.enabled) { continue; }
            for p in &cache.prompts {
                if !plugin_mcp.is_some_and(|s| s.disabled_prompts.contains(&p.name)) {
                    let mut p_clone = p.clone();
                    p_clone.name = format!("{}.{}", plugin_id, p.name);
                    prompts.push(p_clone);
                }
            }
        }
        prompts
    }

    /// Routes a tool call to the correct provider and handles runtime approval.
    ///
    /// **Security audit**: Every MCP tool invocation is recorded in the audit log,
    /// regardless of whether the tool is read-only or mutating. MCP is an external
    /// interface — AI clients can read plugin inventories, filesystem contents, and
    /// system configuration. In a security context, all access must be logged for
    /// compliance and incident investigation.
    pub async fn call_tool(&self, name: &str, arguments: Option<serde_json::Map<String, serde_json::Value>>) -> Result<CallToolResult, McpError> {
        // Extract the primary subject from arguments before dispatch (for the audit trail).
        let subject = arguments.as_ref().and_then(|args| {
            args.get("plugin_id")
                .or_else(|| args.get("ext_id"))
                .or_else(|| args.get("path"))
                .or_else(|| args.get("command"))
                .or_else(|| args.get("manifest_url"))
                .or_else(|| args.get("manifest_path"))
                .or_else(|| args.get("query"))
                .or_else(|| args.get("url"))
                .or_else(|| args.get("pattern"))
                .and_then(|v| v.as_str())
                .map(String::from)
        });

        let result = self.dispatch_tool(name, arguments).await;

        // Security audit: record every MCP tool invocation.
        // Severity is derived from the tool's nature — execute_command and
        // destructive operations are Critical, mutating tools are Warn,
        // read-only tools are Info.
        use crate::audit::{AuditEntry, AuditActor, AuditSeverity, AuditResult as AuditRes};
        let severity = match name {
            n if n.contains("execute_command") => AuditSeverity::Critical,
            n if n.contains("plugin_remove") || n.contains("extension_remove") => AuditSeverity::Critical,
            n if n.contains("plugin_install") || n.contains("plugin_start")
                || n.contains("plugin_stop") || n.contains("extension_enable")
                || n.contains("extension_disable") || n.contains("extension_install") => AuditSeverity::Warn,
            // File writes and edits from an external client warrant Warn
            n if n.contains("write_file") || n.contains("edit_file") => AuditSeverity::Warn,
            _ => AuditSeverity::Info,
        };
        let audit_result = match &result {
            Ok(_) => AuditRes::Success,
            Err(_) => AuditRes::Failure,
        };
        self.audit.record(AuditEntry {
            actor: AuditActor::McpClient,
            source_id: None, // TODO: thread MCP session/client identity here
            severity,
            action: format!("mcp.{}", name),
            subject,
            result: audit_result,
            details: None,
        });

        result
    }

    /// Internal dispatch — routes to builtin, extension, or plugin handler.
    async fn dispatch_tool(&self, name: &str, arguments: Option<serde_json::Map<String, serde_json::Value>>) -> Result<CallToolResult, McpError> {
        // 1. Check for built-in namespace
        if let Some(local_name) = name.strip_prefix("nexus.") {
            return self.call_builtin(local_name, arguments).await;
        }

        // 2. Resolve to a specific plugin or extension using longest-prefix matching.
        let (plugin_id, local_name) = self.resolve_namespace(name).await?;

        // 3. Extension dispatch
        let is_ext = {
            let mgr = self.state.read().await;
            mgr.extensions.get(&plugin_id).is_some()
        };
        if is_ext {
            return self.call_extension(&plugin_id, &local_name, arguments).await;
        }

        // 4. Plugin dispatch
        self.call_plugin(&plugin_id, &local_name, arguments).await
    }

    async fn call_builtin(&self, local_name: &str, arguments: Option<serde_json::Map<String, serde_json::Value>>) -> Result<CallToolResult, McpError> {
        let args_val = serde_json::Value::Object(arguments.unwrap_or_default());
        match builtin::handle_call(local_name, &args_val, &self.state, &self.approval_bridge).await {
            Ok(resp) => {
                let content = resp.content.into_iter().map(|c| Content::text(c.text)).collect();
                if resp.is_error { Ok(CallToolResult::error(content)) } else { Ok(CallToolResult::success(content)) }
            }
            Err(_) => Err(McpError::internal_error(format!("Built-in tool '{}' failed", local_name), None)),
        }
    }

    async fn call_extension(&self, ext_id: &str, operation: &str, arguments: Option<serde_json::Map<String, serde_json::Value>>) -> Result<CallToolResult, McpError> {
        let args_val = serde_json::Value::Object(arguments.unwrap_or_default());
        match builtin::handle_extension_call(ext_id, operation, &args_val, &self.state, &self.approval_bridge).await {
            Ok(resp) => {
                let content = resp.content.into_iter().map(|c| Content::text(c.text)).collect();
                if resp.is_error { Ok(CallToolResult::error(content)) } else { Ok(CallToolResult::success(content)) }
            }
            Err(_) => Err(McpError::internal_error(format!("Extension tool '{}.{}' failed", ext_id, operation), None)),
        }
    }

    /// Handles calls to actual plugins (Native or Legacy).
    ///
    /// This function implements the Nexus **Runtime Approval Flow**:
    /// 1. Checks if the tool requires approval (per manifest).
    /// 2. Checks if the tool has been permanently approved in the whitelist.
    /// 3. If not, pauses the execution and prompts the user via the Host UI.
    async fn call_plugin(&self, plugin_id: &str, local_name: &str, arguments: Option<serde_json::Map<String, serde_json::Value>>) -> Result<CallToolResult, McpError> {
        let mgr = self.state.read().await;
        let plugin = mgr.storage.get(plugin_id).ok_or_else(|| McpError::invalid_request(format!("Plugin '{}' not found", plugin_id), None))?;
        if plugin.status != PluginStatus::Running { return Err(McpError::invalid_request(format!("Plugin '{}' is not running", plugin_id), None)); }

        let plugin_mcp = mgr.mcp_settings.plugins.get(plugin_id);
        if !plugin_mcp.is_some_and(|s| s.enabled && s.enabled_tools.contains(&local_name.to_string())) {
            return Err(McpError::invalid_request(format!("Tool '{}.{}' is not enabled", plugin_id, local_name), None));
        }

        let is_native = mgr.mcp_clients.has(plugin_id);
        let mcp_config = plugin.manifest.mcp.as_ref();
        let requires_approval = if is_native {
            mcp_config.and_then(|c| c.server.as_ref()).is_some_and(|s| s.requires_approval)
        } else {
            mcp_config.and_then(|c| c.tools.iter().find(|t| t.name == local_name)).is_some_and(|t| t.requires_approval)
        };

        // Legacy tools also require host-level permission checks
        if !is_native {
            if let Some(tool_def) = mcp_config.and_then(|c| c.tools.iter().find(|t| t.name == local_name)) {
                for perm_str in &tool_def.permissions {
                    let perm = serde_json::from_value::<Permission>(serde_json::Value::String(perm_str.clone())).map_err(|_| McpError::internal_error("Invalid permission", None))?;
                    if !mgr.permissions.has_permission(plugin_id, &perm) { return Err(McpError::invalid_request(format!("Plugin '{}' lacks permission '{}'", plugin_id, perm_str), None)); }
                }
            }
        }

        let plugin_name = plugin.manifest.name.clone();
        let port = plugin.assigned_port;
        let already_approved = requires_approval && plugin_mcp.is_some_and(|s| s.approved_tools.contains(&local_name.to_string()));
        drop(mgr);

        // Runtime approval trigger
        if requires_approval && !already_approved {
            let mut context = HashMap::new();
            context.insert("tool_name".to_string(), local_name.to_string());
            context.insert("plugin_name".to_string(), plugin_name.clone());
            if let Some(ref args) = arguments {
                for (k, v) in args { context.insert(format!("arg.{}", k), match v { serde_json::Value::String(s) => s.clone(), other => other.to_string() }); }
            }
            let approval_req = ApprovalRequest { id: uuid::Uuid::new_v4().to_string(), plugin_id: plugin_id.to_string(), plugin_name: plugin_name.clone(), category: "mcp_tool".to_string(), permission: format!("mcp:{}:{}", plugin_id, local_name), context };
            match self.approval_bridge.request_approval(approval_req).await {
                ApprovalDecision::Approve => {
                    let mut mgr = self.state.write().await;
                    let s = mgr.mcp_settings.plugins.entry(plugin_id.to_string()).or_insert_with(McpPluginSettings::default);
                    if !s.approved_tools.contains(&local_name.to_string()) { s.approved_tools.push(local_name.to_string()); }
                    let _ = mgr.mcp_settings.save();
                }
                ApprovalDecision::ApproveOnce => {}
                ApprovalDecision::Deny => return Ok(CallToolResult::error(vec![Content::text(format!("[Nexus] Tool '{}.{}' was denied by the user.", plugin_id, local_name))])),
            }
        }

        if is_native {
            let mgr = self.state.read().await;
            mgr.mcp_clients.call_tool(plugin_id, local_name, arguments).await.map_err(|e| McpError::internal_error(e, None))
        } else {
            // Legacy HTTP fallback
            let client = reqwest::Client::new();
            let url = format!("http://localhost:{}/mcp/call", port);
            let body = serde_json::json!({ "tool_name": local_name, "arguments": arguments.unwrap_or_default() });
            match client.post(&url).json(&body).timeout(std::time::Duration::from_secs(30)).send().await {
                Ok(resp) if resp.status().is_success() => {
                    let call_resp = resp.json::<super::types::McpCallResponse>().await.map_err(|e| McpError::internal_error(e.to_string(), None))?;
                    let content = call_resp.content.into_iter().map(|c| Content::text(c.text)).collect();
                    if call_resp.is_error { Ok(CallToolResult::error(content)) } else { Ok(CallToolResult::success(content)) }
                }
                Ok(resp) => Ok(CallToolResult::error(vec![Content::text(format!("[Nexus] Plugin returned HTTP {}", resp.status()))])),
                Err(e) => Ok(CallToolResult::error(vec![Content::text(format!("[Nexus] Plugin not responding: {}", e))])),
            }
        }
    }

    /// Resolve a namespaced name (e.g., "com.nexus.hello-world.greet") to (plugin_id, local_name)
    /// using longest-prefix matching against registered plugins and extensions.
    pub async fn resolve_namespace(&self, namespaced: &str) -> Result<(String, String), McpError> {
        let mgr = self.state.read().await;
        let mut best_plugin_id: Option<String> = None;
        let mut best_local_name: Option<String> = None;
        for plugin in mgr.storage.list() {
            let prefix = format!("{}.", plugin.manifest.id);
            if let Some(local) = namespaced.strip_prefix(&prefix) {
                if !local.is_empty() && best_plugin_id.as_ref().map_or(true, |prev| plugin.manifest.id.len() > prev.len()) {
                    best_plugin_id = Some(plugin.manifest.id.clone()); best_local_name = Some(local.to_string());
                }
            }
        }
        for ext in mgr.extension_loader.storage.list() {
            let prefix = format!("{}.", ext.manifest.id);
            if let Some(local) = namespaced.strip_prefix(&prefix) {
                if !local.is_empty() && best_plugin_id.as_ref().map_or(true, |prev| ext.manifest.id.len() > prev.len()) {
                    best_plugin_id = Some(ext.manifest.id.clone()); best_local_name = Some(local.to_string());
                }
            }
        }
        match (best_plugin_id, best_local_name) {
            (Some(pid), Some(name)) => Ok((pid, name)),
            _ => Err(McpError::invalid_request(format!("Cannot resolve '{}'", namespaced), None)),
        }
    }
}
