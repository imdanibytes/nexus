import { memo, useCallback, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import type { InstalledPlugin } from "../../types/plugin";
import type { McpToolDef } from "../../types/mcp";
import type { PluginAction } from "../../stores/appStore";
import { useAppStore } from "../../stores/appStore";

/** Stable selector — returns the same reference if the plugin hasn't meaningfully changed. */
function usePlugin(pluginId: string): InstalledPlugin | undefined {
  const ref = useRef<InstalledPlugin | undefined>(undefined);
  return useAppStore((s) => {
    const next = s.installedPlugins.find((p) => p.manifest.id === pluginId);
    if (!next) return undefined;
    const prev = ref.current;
    // Fast structural check: status, version, port, and dev_mode cover all render-relevant fields
    if (
      prev &&
      prev.status === next.status &&
      prev.manifest.version === next.manifest.version &&
      prev.assigned_port === next.assigned_port &&
      prev.dev_mode === next.dev_mode &&
      prev.local_manifest_path === next.local_manifest_path
    ) {
      return prev; // same reference → no re-render
    }
    ref.current = next;
    return next;
  });
}
import { usePluginActions } from "../../hooks/usePlugins";
import { getColorMode } from "../../lib/theme";
import { Play, StopCircle, Loader2, Trash2, Square, Terminal, Hammer, Expand, Wrench, ScrollText, TriangleAlert, ArrowUp } from "lucide-react";
import {
  Button,
  Card,
  CardBody,
  Chip,
  Dropdown,
  DropdownTrigger,
  DropdownMenu,
  DropdownItem,
  DropdownSection,
  Drawer,
  DrawerContent,
  DrawerHeader,
  DrawerBody,
  Modal,
  ModalContent,
  ModalHeader,
  ModalBody,
  ModalFooter,
  useDisclosure,
} from "@heroui/react";

interface Props {
  pluginId: string;
}

export const PluginViewport = memo(function PluginViewport({ pluginId }: Props) {
  const { t } = useTranslation("plugins");
  const plugin = usePlugin(pluginId);
  const busyAction = useAppStore((s) => s.busyPlugins[pluginId] ?? null) as PluginAction | null;
  const { start } = usePluginActions();

  // Ref-based overlay toggle — prevents re-rendering the entire tree when a dropdown opens
  const iframeShieldRef = useRef<HTMLDivElement>(null);
  const handleMenuOpenChange = useCallback((open: boolean) => {
    if (iframeShieldRef.current) {
      iframeShieldRef.current.style.display = open ? "block" : "none";
    }
  }, []);

  const handleStart = useCallback(() => start(pluginId), [start, pluginId]);

  const handleIframeLoad = useCallback((e: React.SyntheticEvent<HTMLIFrameElement>) => {
    const theme = getColorMode();
    try {
      e.currentTarget.contentWindow?.postMessage(
        { type: "nexus:system", event: "theme_changed", data: { theme } },
        "*"
      );
    } catch {
      // cross-origin or unmounted
    }
  }, []);

  if (!plugin) return null;

  const isRunning = plugin.status === "running";
  const isBusy = busyAction !== null;
  const hasUi = plugin.manifest.ui !== null;
  const theme = getColorMode();
  const iframeSrc = hasUi
    ? `http://localhost:${plugin.assigned_port}${plugin.manifest.ui!.path}${plugin.manifest.ui!.path.includes("?") ? "&" : "?"}nexus_theme=${theme}`
    : null;

  return (
    <div className="flex flex-col h-full relative">
      <PluginMenuBar pluginId={pluginId} disabled={isBusy} onOpenChange={handleMenuOpenChange} />

      <div className="flex-1 relative">
        <div ref={iframeShieldRef} className="absolute inset-0 z-10" style={{ display: "none" }} />
        {isRunning && !isBusy && hasUi ? (
          <iframe
            key={`${plugin.manifest.id}-${plugin.manifest.version}`}
            src={iframeSrc!}
            className="w-full h-full border-0"
            title={plugin.manifest.name}
            data-nexus-plugin={plugin.manifest.id}
            sandbox="allow-scripts allow-same-origin"
            onLoad={handleIframeLoad}
          />
        ) : isRunning && !isBusy && !hasUi ? (
          <HeadlessPluginStatus plugin={plugin} />
        ) : !isBusy ? (
          <div className="flex flex-col items-center justify-center h-full text-center">
            <div className="w-16 h-16 rounded-[14px] bg-default-100 flex items-center justify-center mb-4">
              <StopCircle size={28} strokeWidth={1.5} className="text-default-400" />
            </div>
            <p className="text-[13px] text-default-500 mb-4">
              {plugin.status === "error"
                ? t("viewport.pluginError")
                : t("viewport.pluginStopped")}
            </p>
            <Button color="primary" onPress={handleStart} startContent={<Play size={14} strokeWidth={1.5} />}>
              {t("viewport.startPlugin")}
            </Button>
          </div>
        ) : null}
      </div>

      {busyAction && (
        <BusyOverlay action={busyAction} pluginName={plugin.manifest.name} />
      )}
    </div>
  );
});

const PluginMenuBar = memo(function PluginMenuBar({ pluginId, disabled, onOpenChange }: { pluginId: string; disabled: boolean; onOpenChange?: (open: boolean) => void }) {
  const { t } = useTranslation("plugins");
  const plugin = usePlugin(pluginId);
  const { start, stop, restart, remove, rebuild, toggleDevMode } = usePluginActions();

  if (!plugin) return null;

  const isRunning = plugin.status === "running";
  const isLocal = !!plugin.local_manifest_path;
  const id = pluginId;
  const removeModal = useDisclosure();
  const aboutModal = useDisclosure();

  const handleStart = () => start(id);
  const handleStop = () => stop(id);
  const handleRestart = () => restart(id);
  const handleRebuild = () => rebuild(id);
  const handleToggleDevMode = () => toggleDevMode(id, !plugin.dev_mode);
  const handleRemove = () => { removeModal.onClose(); remove(id); };

  const m = plugin.manifest;

  return (
    <>
      <div
        className="flex items-center gap-0 mx-2 mt-2 px-1 h-8 rounded-xl bg-default-50/40 backdrop-blur-xl border border-default-200/50"
      >
        {/* App menu */}
        <Dropdown onOpenChange={(open) => onOpenChange?.(open)}>
          <DropdownTrigger>
            <button className="px-2 py-1 text-[13px] font-semibold rounded hover:bg-default-200/40 transition-colors">
              {m.name}
            </button>
          </DropdownTrigger>
          <DropdownMenu aria-label="Plugin menu">
            <DropdownSection showDivider>
              <DropdownItem key="about" onPress={aboutModal.onOpen}>
                {t("menu.about", { name: m.name })}
              </DropdownItem>
            </DropdownSection>
            <DropdownSection showDivider>
              {isRunning ? (
                <>
                  <DropdownItem key="restart" onPress={handleRestart} isDisabled={disabled} startContent={<Play size={14} strokeWidth={1.5} className="text-success" />}>
                    {t("common:action.restart")}
                  </DropdownItem>
                  <DropdownItem key="stop" onPress={handleStop} isDisabled={disabled} startContent={<Square size={14} strokeWidth={1.5} className="text-warning" />}>
                    {t("common:action.stop")}
                  </DropdownItem>
                </>
              ) : (
                <DropdownItem key="start" onPress={handleStart} isDisabled={disabled} startContent={<Play size={14} strokeWidth={1.5} className="text-success" />}>
                  {t("common:action.start")}
                </DropdownItem>
              )}
            </DropdownSection>
            <DropdownSection>
              <DropdownItem key="remove" onPress={removeModal.onOpen} isDisabled={disabled} className="text-danger" color="danger" startContent={<Trash2 size={14} strokeWidth={1.5} />}>
                {t("menu.remove", { name: m.name })}
              </DropdownItem>
            </DropdownSection>
          </DropdownMenu>
        </Dropdown>

        {/* View menu */}
        <Dropdown onOpenChange={(open) => onOpenChange?.(open)}>
          <DropdownTrigger>
            <button className="px-2 py-1 text-[13px] text-default-500 rounded hover:bg-default-200/40 transition-colors">
              {t("menu.view")}
            </button>
          </DropdownTrigger>
          <DropdownMenu aria-label="View menu">
            <DropdownItem key="logs" onPress={() => useAppStore.getState().setShowLogs(id)} startContent={<ScrollText size={14} strokeWidth={1.5} />}>
              {t("menu.logs")}
            </DropdownItem>
          </DropdownMenu>
        </Dropdown>

        {/* Dev menu */}
        {isLocal && (
          <Dropdown onOpenChange={(open) => onOpenChange?.(open)}>
            <DropdownTrigger>
              <button className="px-2 py-1 text-[13px] text-default-500 rounded hover:bg-default-200/40 transition-colors">
                {t("menu.dev")}
              </button>
            </DropdownTrigger>
            <DropdownMenu aria-label="Dev menu">
              <DropdownItem key="rebuild" onPress={handleRebuild} isDisabled={disabled} startContent={<Hammer size={14} strokeWidth={1.5} className="text-primary" />}>
                {t("menu.rebuild")}
              </DropdownItem>
              <DropdownItem key="autorebuild" onPress={handleToggleDevMode} isDisabled={disabled} startContent={<Wrench size={14} strokeWidth={1.5} />} endContent={plugin.dev_mode ? <span className="text-[10px] text-primary">ON</span> : null}>
                {t("menu.autoRebuild")}
              </DropdownItem>
            </DropdownMenu>
          </Dropdown>
        )}
      </div>

      {/* About dialog */}
      <Modal isOpen={aboutModal.isOpen} onOpenChange={aboutModal.onOpenChange}>
        <ModalContent>
          <ModalHeader className="flex flex-col items-center text-center pb-0">
            <div className="w-16 h-16 rounded-[14px] bg-default-100 flex items-center justify-center mb-2">
              {m.icon ? (
                <img src={m.icon} alt={m.name} className="w-10 h-10 rounded-md" />
              ) : (
                <Terminal size={28} strokeWidth={1.5} className="text-primary" />
              )}
            </div>
            <span className="text-base">{m.name}</span>
          </ModalHeader>
          <ModalBody className="text-center">
            <p className="text-[12px] text-default-500">{m.description}</p>
            <div className="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1.5 text-left text-[11px] mt-3">
              <span className="text-default-400">{t("about.version")}</span>
              <span className="font-mono text-default-500">{m.version}</span>
              <span className="text-default-400">{t("about.author")}</span>
              <span className="text-default-500">{m.author}</span>
              <span className="text-default-400">{t("about.id")}</span>
              <span className="font-mono text-default-500">{m.id}</span>
              {m.license && (
                <>
                  <span className="text-default-400">{t("about.license")}</span>
                  <span className="text-default-500">{m.license}</span>
                </>
              )}
              <span className="text-default-400">{t("about.type")}</span>
              <span className="text-default-500">{m.ui ? t("about.uiPlugin") : t("about.headlessService")}</span>
              {m.mcp && (
                <>
                  <span className="text-default-400">{t("viewport.mcpTools")}</span>
                  <span className="text-default-500">{m.mcp.tools?.length ?? 0}</span>
                </>
              )}
            </div>
          </ModalBody>
        </ModalContent>
      </Modal>

      {/* Remove confirmation */}
      <Modal isOpen={removeModal.isOpen} onOpenChange={removeModal.onOpenChange}>
        <ModalContent>
          {(onClose) => (
            <>
              <ModalHeader className="flex items-center gap-2 text-base">
                <TriangleAlert size={18} className="text-warning" />
                {t("common:confirm.removePlugin", { name: m.name })}
              </ModalHeader>
              <ModalBody>
                <p className="text-[13px] text-default-500 leading-relaxed">
                  {t("common:confirm.removePluginDesc")}
                </p>
              </ModalBody>
              <ModalFooter>
                <Button onPress={onClose}>
                  {t("common:action.cancel")}
                </Button>
                <Button color="danger" onPress={handleRemove}>
                  {t("common:confirm.removeAndDeleteData")}
                </Button>
              </ModalFooter>
            </>
          )}
        </ModalContent>
      </Modal>
    </>
  );
});

function McpToolCard({ tool, onDetail }: {
  tool: McpToolDef;
  onDetail: (tool: McpToolDef) => void;
}) {
  const properties = (tool.input_schema?.properties ?? {}) as Record<string, { type?: string }>;
  const params = Object.keys(properties);

  return (
    <Card
      as="button"
      isPressable
      onPress={() => onDetail(tool)}
    >
      <CardBody className="gap-2">
        <div className="flex items-center gap-2">
          <Terminal size={13} strokeWidth={1.5} className="text-primary shrink-0" />
          <span className="text-[12px] font-mono font-medium text-primary truncate">{tool.name}</span>
          <Expand size={12} strokeWidth={1.5} className="ml-auto shrink-0 text-default-400" />
        </div>
        {tool.description && (
          <p className="text-[11px] text-default-500 leading-relaxed line-clamp-3">
            {tool.description}
          </p>
        )}
        {params.length > 0 && (
          <div className="flex flex-wrap gap-1 mt-auto pt-1">
            {params.map((p) => (
              <Chip key={p} size="sm" variant="flat">
                {p}
              </Chip>
            ))}
          </div>
        )}
      </CardBody>
    </Card>
  );
}

function SchemaBlock({ label, schema }: { label: string; schema: Record<string, unknown> }) {
  return (
    <div>
      <p className="text-[11px] font-semibold text-default-500 uppercase tracking-wider mb-1.5">
        {label}
      </p>
      <pre className="text-[11px] font-mono text-default-500 bg-background border border-default-200/50 rounded-[6px] p-3 overflow-x-auto whitespace-pre-wrap break-words">
        {JSON.stringify(schema, null, 2)}
      </pre>
    </div>
  );
}

function McpToolDetailDrawer({
  tool,
  open,
  onOpenChange,
}: {
  tool: McpToolDef | null;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const { t } = useTranslation("plugins");

  // Keep last tool visible while the drawer animates out
  const [displayTool, setDisplayTool] = useState<McpToolDef | null>(null);
  if (tool && tool !== displayTool) setDisplayTool(tool);

  const shown = displayTool ?? tool;
  const properties = (shown?.input_schema?.properties ?? {}) as Record<string, { type?: string; description?: string }>;
  const required = (shown?.input_schema?.required ?? []) as string[];
  const params = Object.entries(properties);

  return (
    <Drawer isOpen={open} onOpenChange={onOpenChange} onClose={() => setDisplayTool(null)} placement="right">
      <DrawerContent>
        <DrawerHeader className="flex flex-col gap-1">
          <div className="flex items-center gap-2">
            <Terminal size={15} strokeWidth={1.5} className="text-primary" />
            <span className="font-mono text-primary text-[14px]">
              {shown?.name}
            </span>
          </div>
          {shown?.description && (
            <p className="text-default-500 text-[12px] leading-relaxed font-normal">
              {shown.description}
            </p>
          )}
        </DrawerHeader>

        <DrawerBody>
          <div className="flex flex-col gap-5">
            {params.length > 0 && (
              <div>
                <p className="text-[11px] font-semibold text-default-500 uppercase tracking-wider mb-2">
                  {t("viewport.parameters")}
                </p>
                <div className="space-y-2">
                  {params.map(([name, meta]) => (
                    <div
                      key={name}
                      className="rounded-[8px] px-3 py-2"
                    >
                      <div className="flex items-center gap-2">
                        <span className="text-[11px] font-mono font-medium">
                          {name}
                        </span>
                        {meta.type && (
                          <Chip size="sm" variant="flat">
                            {meta.type}
                          </Chip>
                        )}
                        {required.includes(name) && (
                          <Chip size="sm" variant="flat" color="primary">
                            {t("viewport.required")}
                          </Chip>
                        )}
                      </div>
                      {meta.description && (
                        <p className="text-[10px] text-default-400 mt-1 leading-relaxed">
                          {meta.description}
                        </p>
                      )}
                    </div>
                  ))}
                </div>
              </div>
            )}

            {shown?.input_schema && Object.keys(shown.input_schema).length > 0 && (
              <SchemaBlock label={t("viewport.inputSchema")} schema={shown.input_schema} />
            )}

            {(shown?.permissions?.length ?? 0) > 0 && (
              <div>
                <p className="text-[11px] font-semibold text-default-500 uppercase tracking-wider mb-1.5">
                  {t("viewport.requiredPermissions")}
                </p>
                <div className="flex flex-wrap gap-1.5">
                  {shown!.permissions.map((p) => (
                    <Chip key={p} size="sm" variant="flat" color="warning">
                      {p}
                    </Chip>
                  ))}
                </div>
              </div>
            )}
          </div>
        </DrawerBody>
      </DrawerContent>
    </Drawer>
  );
}

function HeadlessPluginStatus({ plugin }: { plugin: InstalledPlugin }) {
  const { t } = useTranslation("plugins");
  const mcpTools = plugin.manifest.mcp?.tools ?? [];
  const [detailTool, setDetailTool] = useState<McpToolDef | null>(null);

  return (
    <div className="h-full overflow-y-auto p-6">
      <div className="flex flex-col items-center text-center mb-6">
        <div className="w-14 h-14 rounded-[14px] bg-default-100 flex items-center justify-center mb-3">
          <Terminal size={24} strokeWidth={1.5} className="text-primary" />
        </div>
        <p className="text-[14px] font-semibold mb-1">
          {t("viewport.headlessRunning")}
        </p>
        <p className="text-[12px] text-default-500 max-w-md">
          {t("viewport.headlessDesc", { count: mcpTools.length })}
        </p>
      </div>
      {mcpTools.length > 0 && (
        <div>
          <p className="text-[11px] font-semibold text-default-500 uppercase tracking-wider mb-3">
            {t("viewport.mcpTools")}
          </p>
          <div className="grid grid-cols-[repeat(auto-fill,minmax(220px,1fr))] gap-2.5">
            {mcpTools.map((tool) => (
              <McpToolCard key={tool.name} tool={tool} onDetail={setDetailTool} />
            ))}
          </div>
        </div>
      )}

      <McpToolDetailDrawer
        tool={detailTool}
        open={detailTool !== null}
        onOpenChange={(open) => { if (!open) setDetailTool(null); }}
      />
    </div>
  );
}

function BusyOverlay({ action, pluginName }: { action: PluginAction; pluginName: string }) {
  const { t } = useTranslation("plugins");

  const overlayConfig: Record<
    PluginAction,
    { icon: typeof Trash2; label: string; sub: string; color: string; bg: string }
  > = {
    removing: {
      icon: Trash2,
      label: t("overlay.removing"),
      sub: t("overlay.removingSub"),
      color: "text-danger",
      bg: "bg-danger-50",
    },
    stopping: {
      icon: Square,
      label: t("overlay.stopping"),
      sub: t("overlay.stoppingSub"),
      color: "text-warning",
      bg: "bg-warning-50",
    },
    starting: {
      icon: Play,
      label: t("overlay.starting"),
      sub: t("overlay.startingSub"),
      color: "text-success",
      bg: "bg-success-50",
    },
    rebuilding: {
      icon: Hammer,
      label: t("overlay.rebuilding"),
      sub: t("overlay.rebuildingSub"),
      color: "text-primary",
      bg: "bg-primary-50",
    },
    updating: {
      icon: ArrowUp,
      label: t("overlay.updating"),
      sub: t("overlay.updatingSub"),
      color: "text-primary",
      bg: "bg-primary-50",
    },
  };

  const config = overlayConfig[action];
  const Icon = config.icon;

  return (
    <div className="absolute inset-0 z-50 flex items-center justify-center bg-default-50/40 backdrop-blur-xl">
      <div className="flex flex-col items-center gap-4 rounded-xl bg-default-50/40 backdrop-blur-xl border border-default-200/50 px-10 py-8">
        <div className={`w-16 h-16 rounded-[14px] ${config.bg} flex items-center justify-center`}>
          <Icon size={28} strokeWidth={1.5} className={config.color} />
        </div>
        <div className="text-center">
          <p className="text-[14px] font-semibold mb-1">
            {config.label} {pluginName}
          </p>
          <p className="text-[12px] text-default-500">
            {config.sub}
          </p>
        </div>
        <Loader2 size={20} strokeWidth={1.5} className="text-default-500 animate-spin" />
      </div>
    </div>
  );
}
