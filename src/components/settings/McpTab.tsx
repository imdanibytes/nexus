import { useCallback, useEffect, useState } from "react";
import {
  mcpGetSettings,
  mcpSetEnabled,
  mcpListTools,
  mcpConfigSnippet,
} from "../../lib/tauri";
import type { McpSettings, McpToolStatus } from "../../types/mcp";
import {
  Cpu,
  Copy,
  Check,
  ChevronDown,
  Shield,
  ShieldAlert,
  CircleDot,
  Wrench,
  Terminal,
} from "lucide-react";

type ConfigTab = "desktop" | "code";

export function McpTab() {
  const [settings, setSettings] = useState<McpSettings | null>(null);
  const [tools, setTools] = useState<McpToolStatus[]>([]);
  const [desktopSnippet, setDesktopSnippet] = useState<string>("");
  const [codeSnippet, setCodeSnippet] = useState<string>("");
  const [configTab, setConfigTab] = useState<ConfigTab>("desktop");
  const [copied, setCopied] = useState(false);
  const [expanded, setExpanded] = useState<Set<string>>(new Set());
  const [loading, setLoading] = useState(true);

  const refresh = useCallback(async () => {
    try {
      const [s, t, c] = await Promise.all([
        mcpGetSettings(),
        mcpListTools(),
        mcpConfigSnippet(),
      ]);
      setSettings(s);
      setTools(t);
      const data = c as { desktop_config: unknown; claude_code_command: string };
      setDesktopSnippet(JSON.stringify(data.desktop_config, null, 2));
      setCodeSnippet(data.claude_code_command);
    } catch {
      // backend may not have MCP commands yet
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  async function toggleGlobal(enabled: boolean) {
    await mcpSetEnabled("global", enabled);
    await refresh();
  }

  async function togglePlugin(pluginId: string, enabled: boolean) {
    await mcpSetEnabled(`plugin:${pluginId}`, enabled);
    await refresh();
  }

  async function toggleTool(toolName: string, enabled: boolean) {
    await mcpSetEnabled(`tool:${toolName}`, enabled);
    await refresh();
  }

  function toggleExpanded(pluginId: string) {
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(pluginId)) {
        next.delete(pluginId);
      } else {
        next.add(pluginId);
      }
      return next;
    });
  }

  const activeSnippet = configTab === "desktop" ? desktopSnippet : codeSnippet;

  async function copySnippet() {
    await navigator.clipboard.writeText(activeSnippet);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  }

  // Group tools by plugin
  const pluginGroups = tools.reduce<
    Record<string, { pluginName: string; pluginId: string; tools: McpToolStatus[] }>
  >((acc, tool) => {
    if (!acc[tool.plugin_id]) {
      acc[tool.plugin_id] = {
        pluginName: tool.plugin_name,
        pluginId: tool.plugin_id,
        tools: [],
      };
    }
    acc[tool.plugin_id].tools.push(tool);
    return acc;
  }, {});

  if (loading) {
    return (
      <div className="space-y-6">
        <section className="bg-nx-surface rounded-[var(--radius-card)] border border-nx-border p-5">
          <p className="text-[12px] text-nx-text-ghost">Loading MCP settings...</p>
        </section>
      </div>
    );
  }

  const globalEnabled = settings?.enabled ?? false;

  return (
    <div className="space-y-6">
      {/* Section 1: MCP Gateway */}
      <section className="bg-nx-surface rounded-[var(--radius-card)] border border-nx-border p-5">
        <div className="flex items-center justify-between mb-3">
          <div className="flex items-center gap-2">
            <Cpu size={15} strokeWidth={1.5} className="text-nx-text-muted" />
            <h3 className="text-[14px] font-semibold text-nx-text">
              MCP Gateway
            </h3>
          </div>

          {/* Global toggle */}
          <button
            onClick={() => toggleGlobal(!globalEnabled)}
            className={`relative w-10 h-[22px] rounded-full transition-colors duration-200 ${
              globalEnabled ? "bg-nx-accent" : "bg-nx-overlay"
            }`}
          >
            <span
              className={`absolute top-[3px] left-[3px] w-4 h-4 rounded-full bg-white shadow transition-transform duration-200 ${
                globalEnabled ? "translate-x-[18px]" : ""
              }`}
            />
          </button>
        </div>

        <div className="flex items-center gap-2 mb-2">
          <CircleDot
            size={12}
            strokeWidth={2}
            className={globalEnabled ? "text-nx-success" : "text-nx-text-ghost"}
          />
          <span
            className={`text-[12px] font-medium ${
              globalEnabled ? "text-nx-success" : "text-nx-text-ghost"
            }`}
          >
            {globalEnabled ? "Gateway Active" : "Gateway Disabled"}
          </span>
        </div>

        <p className="text-[11px] text-nx-text-ghost">
          Expose plugin tools to AI assistants like Claude Desktop via the Model
          Context Protocol.
        </p>
      </section>

      {/* Section 2: Client Setup */}
      {globalEnabled && (desktopSnippet || codeSnippet) && (
        <section className="bg-nx-surface rounded-[var(--radius-card)] border border-nx-border p-5">
          <div className="flex items-center gap-2 mb-4">
            <Terminal size={15} strokeWidth={1.5} className="text-nx-text-muted" />
            <h3 className="text-[14px] font-semibold text-nx-text">
              Client Setup
            </h3>
          </div>

          {/* Client tabs */}
          <div className="flex gap-1 mb-3">
            {(
              [
                { id: "desktop" as ConfigTab, label: "Claude Desktop" },
                { id: "code" as ConfigTab, label: "Claude Code" },
              ] as const
            ).map((tab) => (
              <button
                key={tab.id}
                onClick={() => { setConfigTab(tab.id); setCopied(false); }}
                className={`px-3 py-1.5 text-[11px] font-medium rounded-[var(--radius-button)] transition-all duration-150 ${
                  configTab === tab.id
                    ? "bg-nx-accent text-nx-deep"
                    : "bg-nx-overlay text-nx-text-muted hover:text-nx-text-secondary"
                }`}
              >
                {tab.label}
              </button>
            ))}
          </div>

          <p className="text-[11px] text-nx-text-ghost mb-3">
            {configTab === "desktop"
              ? "Add this to your Claude Desktop config file."
              : "Run this in your terminal to register the MCP server."}
          </p>

          <div className="relative">
            <pre className="bg-nx-deep border border-nx-border-subtle rounded-[var(--radius-button)] p-3 text-[11px] text-nx-text-secondary font-mono overflow-x-auto leading-relaxed whitespace-pre-wrap break-all">
              {activeSnippet}
            </pre>
            <button
              onClick={copySnippet}
              className="absolute top-2 right-2 p-1.5 rounded-[var(--radius-button)] bg-nx-surface border border-nx-border-subtle hover:bg-nx-wash transition-colors duration-150"
              title="Copy to clipboard"
            >
              {copied ? (
                <Check size={12} strokeWidth={1.5} className="text-nx-success" />
              ) : (
                <Copy size={12} strokeWidth={1.5} className="text-nx-text-ghost" />
              )}
            </button>
          </div>
        </section>
      )}

      {/* Section 3: Tool Registry */}
      <section className="bg-nx-surface rounded-[var(--radius-card)] border border-nx-border p-5">
        <div className="flex items-center gap-2 mb-4">
          <Wrench size={15} strokeWidth={1.5} className="text-nx-text-muted" />
          <h3 className="text-[14px] font-semibold text-nx-text">
            Tool Registry
          </h3>
        </div>

        {Object.keys(pluginGroups).length === 0 ? (
          <p className="text-[11px] text-nx-text-ghost">
            No plugins with MCP tools installed.
          </p>
        ) : (
          <div className="space-y-2">
            {Object.values(pluginGroups).map((group) => {
              const isOpen = expanded.has(group.pluginId);
              const pluginSettings = settings?.plugins[group.pluginId];
              const pluginEnabled = pluginSettings?.enabled ?? true;
              const firstTool = group.tools[0];
              const pluginRunning = firstTool?.plugin_running ?? false;

              return (
                <div
                  key={group.pluginId}
                  className="rounded-[var(--radius-button)] border border-nx-border-subtle bg-nx-deep overflow-hidden"
                >
                  {/* Plugin header */}
                  <div className="flex items-center justify-between p-3">
                    <button
                      onClick={() => toggleExpanded(group.pluginId)}
                      className="flex items-center gap-3 min-w-0 flex-1 hover:opacity-80 transition-opacity"
                    >
                      <CircleDot
                        size={10}
                        strokeWidth={2.5}
                        className={
                          pluginRunning ? "text-nx-success" : "text-nx-text-ghost"
                        }
                      />
                      <span className="text-[13px] text-nx-text font-medium truncate">
                        {group.pluginName}
                      </span>
                      <span className="text-[11px] text-nx-text-ghost flex-shrink-0">
                        {group.tools.length} tool{group.tools.length !== 1 ? "s" : ""}
                      </span>
                      <ChevronDown
                        size={14}
                        strokeWidth={1.5}
                        className={`text-nx-text-ghost transition-transform duration-200 ${
                          isOpen ? "rotate-180" : ""
                        }`}
                      />
                    </button>

                    {/* Plugin-level toggle */}
                    <button
                      onClick={() => togglePlugin(group.pluginId, !pluginEnabled)}
                      className={`relative w-8 h-[18px] rounded-full transition-colors duration-200 flex-shrink-0 ml-3 ${
                        pluginEnabled ? "bg-nx-accent" : "bg-nx-overlay"
                      }`}
                    >
                      <span
                        className={`absolute top-[2px] left-[2px] w-[14px] h-[14px] rounded-full bg-white shadow transition-transform duration-200 ${
                          pluginEnabled ? "translate-x-[14px]" : ""
                        }`}
                      />
                    </button>
                  </div>

                  {/* Expanded tool list */}
                  {isOpen && (
                    <div className="border-t border-nx-border-subtle">
                      {group.tools.map((tool) => (
                        <div
                          key={tool.name}
                          className="flex items-center justify-between px-3 py-2.5 border-b border-nx-border-subtle last:border-b-0 hover:bg-nx-wash/20 transition-colors"
                        >
                          <div className="min-w-0 flex-1 mr-3">
                            <div className="flex items-center gap-2 mb-0.5">
                              <span className="text-[12px] text-nx-text font-mono truncate">
                                {tool.name.split(".").pop()}
                              </span>
                              {/* Permission badge with tooltip */}
                              <span className="relative group flex-shrink-0">
                                {tool.permissions_granted ? (
                                  <Shield
                                    size={11}
                                    strokeWidth={1.5}
                                    className="text-nx-success cursor-help"
                                  />
                                ) : (
                                  <ShieldAlert
                                    size={11}
                                    strokeWidth={1.5}
                                    className="text-nx-warning cursor-help"
                                  />
                                )}
                                <span className="absolute bottom-full left-1/2 -translate-x-1/2 mb-1.5 px-2 py-1 text-[10px] font-medium text-nx-text bg-nx-surface border border-nx-border rounded-[var(--radius-tag)] shadow-sm whitespace-nowrap opacity-0 pointer-events-none group-hover:opacity-100 transition-opacity duration-150 z-10">
                                  {tool.permissions_granted
                                    ? "All required permissions granted"
                                    : `Missing permissions: ${tool.required_permissions.join(", ")}`}
                                </span>
                              </span>
                            </div>
                            <p className="text-[11px] text-nx-text-ghost truncate">
                              {tool.description}
                            </p>
                            {tool.required_permissions.length > 0 && (
                              <div className="flex flex-wrap gap-1 mt-1">
                                {tool.required_permissions.map((perm) => (
                                  <span
                                    key={perm}
                                    className={`text-[9px] font-medium px-1.5 py-0.5 rounded-[var(--radius-tag)] ${
                                      tool.permissions_granted
                                        ? "bg-nx-success-muted text-nx-success"
                                        : "bg-nx-warning-muted text-nx-warning"
                                    }`}
                                  >
                                    {perm}
                                  </span>
                                ))}
                              </div>
                            )}
                          </div>

                          {/* Tool-level toggle */}
                          <button
                            onClick={() => toggleTool(tool.name, !tool.tool_enabled)}
                            className={`relative w-8 h-[18px] rounded-full transition-colors duration-200 flex-shrink-0 ${
                              tool.tool_enabled ? "bg-nx-accent" : "bg-nx-overlay"
                            }`}
                          >
                            <span
                              className={`absolute top-[2px] left-[2px] w-[14px] h-[14px] rounded-full bg-white shadow transition-transform duration-200 ${
                                tool.tool_enabled ? "translate-x-[14px]" : ""
                              }`}
                            />
                          </button>
                        </div>
                      ))}
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        )}
      </section>
    </div>
  );
}
