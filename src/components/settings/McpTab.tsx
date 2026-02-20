import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  mcpGetSettings,
  mcpSetEnabled,
  mcpListTools,
  mcpConfigSnippet,
  apiKeyGetDefault,
  apiKeyRegenerateDefault,
} from "../../lib/tauri";
import type { McpSettings, McpToolStatus } from "../../types/mcp";
import {
  Cpu,
  ChevronDown,
  Shield,
  ShieldAlert,
  CircleDot,
  Wrench,
  Terminal,
  Key,
  Copy,
  Check,
  RefreshCw,
  Eye,
  EyeOff,
  TriangleAlert,
} from "lucide-react";
import {
  Switch,
  Chip,
  Card,
  CardBody,
  Tabs,
  Tab,
  Tooltip,
  Button,
  Modal,
  ModalContent,
  ModalHeader,
  ModalBody,
  ModalFooter,
} from "@heroui/react";
import { CodeBlock } from "@imdanibytes/nexus-ui";

type ConfigTab = "desktop" | "code" | "cursor" | "cline" | "kiro";

export function McpTab() {
  const { t } = useTranslation("settings");
  const [settings, setSettings] = useState<McpSettings | null>(null);
  const [tools, setTools] = useState<McpToolStatus[]>([]);
  const [configData, setConfigData] = useState<{
    desktop_config: unknown;
    claude_code_command: string;
    cursor_config: unknown;
    cline_config: unknown;
    kiro_config: unknown;
  } | null>(null);
  const [configTab, setConfigTab] = useState<ConfigTab>("desktop");
  const [expanded, setExpanded] = useState<Set<string>>(new Set());
  const [loading, setLoading] = useState(true);
  const [apiKey, setApiKey] = useState<string | null>(null);
  const [keyVisible, setKeyVisible] = useState(false);
  const [keyCopied, setKeyCopied] = useState(false);
  const [regenerating, setRegenerating] = useState(false);
  const [regenDialogOpen, setRegenDialogOpen] = useState(false);

  const refresh = useCallback(async () => {
    try {
      const [s, t, c, key] = await Promise.all([
        mcpGetSettings(),
        mcpListTools(),
        mcpConfigSnippet(),
        apiKeyGetDefault(),
      ]);
      setSettings(s);
      setTools(t);
      setConfigData(c as typeof configData);
      setApiKey(key);
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
        <Card><CardBody className="p-5">
          <p className="text-[12px] text-default-400">{t("mcp.loadingSettings")}</p>
        </CardBody></Card>
      </div>
    );
  }

  const globalEnabled = settings?.enabled ?? false;

  async function copyApiKey() {
    if (!apiKey) return;
    await navigator.clipboard.writeText(apiKey);
    setKeyCopied(true);
    setTimeout(() => setKeyCopied(false), 2000);
  }

  async function handleRegenerateConfirmed() {
    setRegenDialogOpen(false);
    setRegenerating(true);
    try {
      await apiKeyRegenerateDefault();
      await refresh();
    } finally {
      setRegenerating(false);
    }
  }

  function maskedKey(key: string): string {
    if (key.length <= 12) return key;
    return key.slice(0, 8) + "•".repeat(key.length - 12) + key.slice(-4);
  }

  const stringify = (v: unknown) => (v ? JSON.stringify(v, null, 2) : "");
  const desktopSnippet = stringify(configData?.desktop_config);
  const codeSnippet = configData?.claude_code_command ?? "";
  const cursorSnippet = stringify(configData?.cursor_config);
  const clineSnippet = stringify(configData?.cline_config);
  const kiroSnippet = stringify(configData?.kiro_config);

  return (
    <div className="space-y-6">
      {/* Section 1: MCP Gateway */}
      <Card><CardBody className="p-5">
        <div className="flex items-center justify-between mb-3">
          <div className="flex items-center gap-2">
            <Cpu size={15} strokeWidth={1.5} className="text-default-500" />
            <h3 className="text-[14px] font-semibold">
              {t("mcp.gateway")}
            </h3>
          </div>

          {/* Global toggle */}
          <Switch isSelected={globalEnabled} onValueChange={(checked) => toggleGlobal(checked)} />
        </div>

        <div className="flex items-center gap-2 mb-2">
          <CircleDot
            size={12}
            strokeWidth={2}
            className={globalEnabled ? "text-success" : "text-default-400"}
          />
          <span
            className={`text-[12px] font-medium ${
              globalEnabled ? "text-success" : "text-default-400"
            }`}
          >
            {globalEnabled ? t("mcp.gatewayActive") : t("mcp.gatewayDisabled")}
          </span>
        </div>

        <p className="text-[11px] text-default-400">
          {t("mcp.gatewayDesc")}
        </p>
      </CardBody></Card>

      {/* Section 2: Client Setup */}
      {globalEnabled && configData && (
        <Card><CardBody className="p-5">
          <div className="flex items-center gap-2 mb-4">
            <Terminal size={15} strokeWidth={1.5} className="text-default-500" />
            <h3 className="text-[14px] font-semibold">
              {t("mcp.clientSetup")}
            </h3>
          </div>

          {/* API Key */}
          {apiKey && (
            <div className="mb-4 p-3 rounded-lg bg-default-100">
              <div className="flex items-center gap-2 mb-2">
                <Key size={13} strokeWidth={1.5} className="text-default-500" />
                <span className="text-[12px] font-medium">{t("mcp.apiKey.label")}</span>
              </div>
              <p className="text-[11px] text-default-400 mb-2">
                {t("mcp.apiKey.description")}
              </p>
              <div className="flex items-center gap-2">
                <code className="flex-1 text-[11px] font-mono bg-default-200 px-2.5 py-1.5 rounded select-all truncate">
                  {keyVisible ? apiKey : maskedKey(apiKey)}
                </code>
                <Tooltip content={keyVisible ? t("common:action.hide") : t("common:action.show")} size="sm">
                  <Button
                    isIconOnly
                    size="sm"
                    variant="flat"
                    onPress={() => setKeyVisible(!keyVisible)}
                  >
                    {keyVisible ? <EyeOff size={14} /> : <Eye size={14} />}
                  </Button>
                </Tooltip>
                <Tooltip content={keyCopied ? t("mcp.apiKey.copied") : t("common:action.copy")} size="sm">
                  <Button
                    isIconOnly
                    size="sm"
                    variant="flat"
                    onPress={copyApiKey}
                  >
                    {keyCopied ? <Check size={14} className="text-success" /> : <Copy size={14} />}
                  </Button>
                </Tooltip>
                <Tooltip content={t("mcp.apiKey.regenerate")} size="sm">
                  <Button
                    isIconOnly
                    size="sm"
                    variant="flat"
                    isLoading={regenerating}
                    onPress={() => setRegenDialogOpen(true)}
                  >
                    <RefreshCw size={14} />
                  </Button>
                </Tooltip>
              </div>
            </div>
          )}

          {/* Client tabs */}
          <Tabs
            selectedKey={configTab}
            onSelectionChange={(key) => setConfigTab(key as ConfigTab)}
            className="mb-3"
            size="sm"
          >
            <Tab key="desktop" title={t("mcp.claudeDesktop")} />
            <Tab key="code" title={t("mcp.claudeCode")} />
            <Tab key="cursor" title="Cursor" />
            <Tab key="cline" title="Cline" />
            <Tab key="kiro" title="Kiro" />
          </Tabs>

          {configTab === "code" ? (
            <div className="space-y-3">
              <p className="text-[11px] text-default-400">
                {t("mcp.codeHint")}
              </p>
              <CodeBlock text={codeSnippet} />
            </div>
          ) : (
            <div className="space-y-3">
              <p className="text-[11px] text-default-400">
                {t(`mcp.${configTab}Hint`)}
              </p>
              <CodeBlock text={
                configTab === "desktop" ? desktopSnippet :
                configTab === "cursor" ? cursorSnippet :
                configTab === "cline" ? clineSnippet :
                kiroSnippet
              } />
            </div>
          )}
        </CardBody></Card>
      )}

      {/* Section 3: Tool Registry */}
      <Card><CardBody className="p-5">
        <div className="flex items-center gap-2 mb-4">
          <Wrench size={15} strokeWidth={1.5} className="text-default-500" />
          <h3 className="text-[14px] font-semibold">
            {t("mcp.toolRegistry")}
          </h3>
        </div>

        {Object.keys(pluginGroups).length === 0 ? (
          <p className="text-[11px] text-default-400">
            {t("mcp.noTools")}
          </p>
        ) : (
          <div className="space-y-2">
            {Object.values(pluginGroups).map((group) => {
              const isOpen = expanded.has(group.pluginId);
              const pluginSettings = settings?.plugins[group.pluginId];
              const pluginEnabled = pluginSettings?.enabled ?? false;
              const firstTool = group.tools[0];
              const pluginRunning = firstTool?.plugin_running ?? false;

              return (
                <div key={group.pluginId} className="overflow-hidden">
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
                          pluginRunning ? "text-success" : "text-default-400"
                        }
                      />
                      <span className="text-[13px] font-medium truncate">
                        {group.pluginName}
                      </span>
                      {group.pluginId === "nexus" && (
                        <Chip size="sm">
                          {t("common:status.builtIn")}
                        </Chip>
                      )}
                      <span className="text-[11px] text-default-400 flex-shrink-0">
                        {t("mcp.toolCount", { count: group.tools.length })}
                      </span>
                      <ChevronDown
                        size={14}
                        strokeWidth={1.5}
                        className={`text-default-400 transition-transform duration-200 ${
                          isOpen ? "rotate-180" : ""
                        }`}
                      />
                    </button>

                    {/* Plugin-level toggle */}
                    <Switch className="flex-shrink-0 ml-3" isSelected={pluginEnabled} onValueChange={(checked) => togglePlugin(group.pluginId, checked)} />
                  </div>

                  {/* Expanded tool list */}
                  {isOpen && (
                    <div className="border-t border-default-100">
                      {group.tools.map((tool) => (
                        <div
                          key={tool.name}
                          className="flex items-center justify-between px-3 py-2.5 border-b border-default-100 last:border-b-0 hover:bg-default-200/20 transition-colors"
                        >
                          <div className="min-w-0 flex-1 mr-3">
                            <div className="flex items-center gap-2 mb-0.5">
                              <span className="text-[12px] font-mono truncate">
                                {tool.name.split(".").pop()}
                              </span>
                              {/* Permission badge with tooltip */}
                              <Tooltip
                                content={
                                  tool.permissions_granted
                                    ? t("mcp.allPermissionsGranted")
                                    : t("mcp.missingPermissions", { permissions: tool.required_permissions.join(", ") })
                                }
                                size="sm"
                              >
                                <span className="flex-shrink-0">
                                  {tool.permissions_granted ? (
                                    <Shield
                                      size={11}
                                      strokeWidth={1.5}
                                      className="text-success cursor-help"
                                    />
                                  ) : (
                                    <ShieldAlert
                                      size={11}
                                      strokeWidth={1.5}
                                      className="text-warning cursor-help"
                                    />
                                  )}
                                </span>
                              </Tooltip>
                            </div>
                            <p className="text-[11px] text-default-400 truncate">
                              {tool.description}
                            </p>
                            {tool.required_permissions.length > 0 && (
                              <div className="flex flex-wrap gap-1 mt-1">
                                {tool.required_permissions.map((perm) => (
                                  <Chip
                                    key={perm}
                                    size="sm"
                                    color={tool.permissions_granted ? "success" : "warning"}
                                  >
                                    {perm}
                                  </Chip>
                                ))}
                              </div>
                            )}
                          </div>

                          {/* Tool-level toggle */}
                          <Switch isSelected={tool.tool_enabled} onValueChange={(checked) => toggleTool(tool.name, checked)} />
                        </div>
                      ))}
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        )}
      </CardBody></Card>

      {/* Regenerate API key confirmation */}
      <Modal isOpen={regenDialogOpen} onOpenChange={setRegenDialogOpen}>
        <ModalContent>
          {(onClose) => (
            <>
              <ModalHeader className="flex items-center gap-2 text-base">
                <TriangleAlert size={18} className="text-warning" />
                {t("mcp.apiKey.regenerate")}
              </ModalHeader>
              <ModalBody>
                <p className="text-[13px] leading-relaxed text-default-500">
                  {t("mcp.apiKey.regenerateConfirm")}
                </p>
              </ModalBody>
              <ModalFooter>
                <Button onPress={onClose}>
                  {t("common:action.cancel")}
                </Button>
                <Button
                  color="danger"
                  onPress={handleRegenerateConfirmed}
                >
                  {t("mcp.apiKey.regenerate")}
                </Button>
              </ModalFooter>
            </>
          )}
        </ModalContent>
      </Modal>
    </div>
  );
}
