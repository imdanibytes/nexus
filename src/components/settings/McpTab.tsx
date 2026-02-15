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
import { Switch } from "@/components/ui/switch";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Tooltip, TooltipTrigger, TooltipContent } from "@/components/ui/tooltip";
import { Collapsible, CollapsibleTrigger, CollapsibleContent } from "@/components/ui/collapsible";

type ConfigTab = "desktop" | "code";

function CopyButton({ text }: { text: string }) {
  const [copied, setCopied] = useState(false);

  async function copy() {
    await navigator.clipboard.writeText(text);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  }

  return (
    <Button
      variant="ghost"
      size="icon-xs"
      onClick={copy}
      className="absolute top-2 right-2 bg-nx-surface border border-nx-border-subtle hover:bg-nx-wash"
      title="Copy to clipboard"
    >
      {copied ? (
        <Check size={12} strokeWidth={1.5} className="text-nx-success" />
      ) : (
        <Copy size={12} strokeWidth={1.5} className="text-nx-text-ghost" />
      )}
    </Button>
  );
}

function CodeBlock({ text }: { text: string }) {
  return (
    <div className="relative">
      <pre className="bg-nx-deep border border-nx-border-subtle rounded-[var(--radius-button)] p-3 text-[11px] text-nx-text-secondary font-mono overflow-x-auto leading-relaxed whitespace-pre-wrap break-all">
        {text}
      </pre>
      <CopyButton text={text} />
    </div>
  );
}

export function McpTab() {
  const [settings, setSettings] = useState<McpSettings | null>(null);
  const [tools, setTools] = useState<McpToolStatus[]>([]);
  const [configData, setConfigData] = useState<{
    direct_config: unknown;
    desktop_config: unknown;
    claude_code_command: string;
    claude_code_command_legacy: string;
  } | null>(null);
  const [configTab, setConfigTab] = useState<ConfigTab>("desktop");
  const [expanded, setExpanded] = useState<Set<string>>(new Set());
  const [legacyOpen, setLegacyOpen] = useState(false);
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
      setConfigData(c as typeof configData);
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

  // Group tools by plugin, with built-in "nexus" group sorted first
  const pluginGroupMap = tools.reduce<
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

  // Sort: "nexus" first, then alphabetical by plugin name
  const pluginGroups = Object.fromEntries(
    Object.entries(pluginGroupMap).sort(([a], [b]) => {
      if (a === "nexus") return -1;
      if (b === "nexus") return 1;
      return (pluginGroupMap[a].pluginName).localeCompare(pluginGroupMap[b].pluginName);
    })
  );

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

  const directDesktopSnippet = configData?.direct_config
    ? JSON.stringify(configData.direct_config, null, 2)
    : "";
  const legacyDesktopSnippet = configData?.desktop_config
    ? JSON.stringify(configData.desktop_config, null, 2)
    : "";

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
          <Switch checked={globalEnabled} onCheckedChange={(checked) => toggleGlobal(checked)} />
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
      {globalEnabled && configData && (
        <section className="bg-nx-surface rounded-[var(--radius-card)] border border-nx-border p-5">
          <div className="flex items-center gap-2 mb-4">
            <Terminal size={15} strokeWidth={1.5} className="text-nx-text-muted" />
            <h3 className="text-[14px] font-semibold text-nx-text">
              Client Setup
            </h3>
          </div>

          {/* Client tabs */}
          <Tabs value={configTab} onValueChange={(v) => { setConfigTab(v as ConfigTab); setLegacyOpen(false); }} className="mb-3">
            <TabsList>
              <TabsTrigger value="desktop">Claude Desktop</TabsTrigger>
              <TabsTrigger value="code">Claude Code</TabsTrigger>
            </TabsList>
          </Tabs>

          {configTab === "desktop" ? (
            <div className="space-y-3">
              <p className="text-[11px] text-nx-text-ghost">
                Add this to your Claude Desktop config file. Uses a direct HTTP connection — no sidecar binary needed.
              </p>
              <CodeBlock text={directDesktopSnippet} />

              {/* Legacy sidecar fallback */}
              <Collapsible open={legacyOpen} onOpenChange={setLegacyOpen}>
                <CollapsibleTrigger asChild>
                  <button className="flex items-center gap-1.5 text-[11px] text-nx-text-ghost hover:text-nx-text-muted transition-colors">
                    <ChevronDown
                      size={12}
                      strokeWidth={1.5}
                      className={`transition-transform duration-200 ${legacyOpen ? "rotate-180" : ""}`}
                    />
                    Legacy (sidecar binary)
                  </button>
                </CollapsibleTrigger>
                <CollapsibleContent>
                  <div className="mt-2 space-y-2">
                    <p className="text-[11px] text-nx-text-ghost">
                      For clients that don't support streamable HTTP transport, use the stdio sidecar instead.
                    </p>
                    <CodeBlock text={legacyDesktopSnippet} />
                  </div>
                </CollapsibleContent>
              </Collapsible>
            </div>
          ) : (
            <div className="space-y-3">
              <p className="text-[11px] text-nx-text-ghost">
                Run this in your terminal to register the MCP server. Uses a direct HTTP connection.
              </p>
              <CodeBlock text={configData.claude_code_command} />

              {/* Legacy sidecar fallback */}
              <Collapsible open={legacyOpen} onOpenChange={setLegacyOpen}>
                <CollapsibleTrigger asChild>
                  <button className="flex items-center gap-1.5 text-[11px] text-nx-text-ghost hover:text-nx-text-muted transition-colors">
                    <ChevronDown
                      size={12}
                      strokeWidth={1.5}
                      className={`transition-transform duration-200 ${legacyOpen ? "rotate-180" : ""}`}
                    />
                    Legacy (sidecar binary)
                  </button>
                </CollapsibleTrigger>
                <CollapsibleContent>
                  <div className="mt-2 space-y-2">
                    <p className="text-[11px] text-nx-text-ghost">
                      For clients that don't support streamable HTTP transport, use the stdio sidecar instead.
                    </p>
                    <CodeBlock text={configData.claude_code_command_legacy} />
                  </div>
                </CollapsibleContent>
              </Collapsible>
            </div>
          )}
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
            No MCP tools available.
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
                <Collapsible
                  key={group.pluginId}
                  open={isOpen}
                  onOpenChange={() => toggleExpanded(group.pluginId)}
                >
                  <div className="rounded-[var(--radius-button)] border border-nx-border-subtle bg-nx-deep overflow-hidden">
                    {/* Plugin header */}
                    <div className="flex items-center justify-between p-3">
                      <CollapsibleTrigger asChild>
                        <button
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
                          {group.pluginId === "nexus" && (
                            <Badge variant="outline" className="text-[9px] px-1.5 py-0 flex-shrink-0">
                              Built-in
                            </Badge>
                          )}
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
                      </CollapsibleTrigger>

                      {/* Plugin-level toggle */}
                      <Switch size="sm" className="flex-shrink-0 ml-3" checked={pluginEnabled} onCheckedChange={(checked) => togglePlugin(group.pluginId, checked)} />
                    </div>

                    {/* Expanded tool list */}
                    <CollapsibleContent>
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
                                <Tooltip>
                                  <TooltipTrigger asChild>
                                    <span className="flex-shrink-0">
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
                                    </span>
                                  </TooltipTrigger>
                                  <TooltipContent>
                                    {tool.permissions_granted
                                      ? "All required permissions granted"
                                      : `Missing permissions: ${tool.required_permissions.join(", ")}`}
                                  </TooltipContent>
                                </Tooltip>
                              </div>
                              <p className="text-[11px] text-nx-text-ghost truncate">
                                {tool.description}
                              </p>
                              {tool.required_permissions.length > 0 && (
                                <div className="flex flex-wrap gap-1 mt-1">
                                  {tool.required_permissions.map((perm) => (
                                    <Badge
                                      key={perm}
                                      variant={tool.permissions_granted ? "success" : "warning"}
                                      className="text-[9px]"
                                    >
                                      {perm}
                                    </Badge>
                                  ))}
                                </div>
                              )}
                            </div>

                            {/* Tool-level toggle */}
                            <Switch size="sm" checked={tool.tool_enabled} onCheckedChange={(checked) => toggleTool(tool.name, checked)} />
                          </div>
                        ))}
                      </div>
                    </CollapsibleContent>
                  </div>
                </Collapsible>
              );
            })}
          </div>
        )}
      </section>
    </div>
  );
}
