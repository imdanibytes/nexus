import { useCallback, useState } from "react";
import { useTranslation } from "react-i18next";
import type { InstalledPlugin } from "../../types/plugin";
import type { McpToolDef } from "../../types/mcp";
import type { PluginAction } from "../../stores/appStore";
import { useAppStore } from "../../stores/appStore";
import { usePlugins as usePluginActions } from "../../hooks/usePlugins";
import { getTheme } from "../../lib/theme";
import { Play, StopCircle, Loader2, Trash2, Square, Terminal, Hammer, Expand, Wrench, ScrollText, TriangleAlert } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  Menubar,
  MenubarContent,
  MenubarItem,
  MenubarMenu,
  MenubarSeparator,
  MenubarCheckboxItem,
  MenubarTrigger,
} from "@/components/ui/menubar";
import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
  SheetDescription,
} from "@/components/ui/sheet";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";

interface Props {
  plugin: InstalledPlugin;
  busyAction: PluginAction | null;
  onStart: () => void;
}

export function PluginViewport({
  plugin,
  busyAction,
  onStart,
}: Props) {
  const { t } = useTranslation("plugins");
  const isRunning = plugin.status === "running";
  const isBusy = busyAction !== null;
  const hasUi = plugin.manifest.ui !== null;
  const theme = getTheme();
  const iframeSrc = hasUi
    ? `http://localhost:${plugin.assigned_port}${plugin.manifest.ui!.path}${plugin.manifest.ui!.path.includes("?") ? "&" : "?"}nexus_theme=${theme}`
    : null;
  const [menuOpen, setMenuOpen] = useState(false);

  // Push current theme to plugin iframe on load so it starts with the right accent
  const handleIframeLoad = useCallback((e: React.SyntheticEvent<HTMLIFrameElement>) => {
    const theme = getTheme();
    try {
      e.currentTarget.contentWindow?.postMessage(
        { type: "nexus:system", event: "theme_changed", data: { theme } },
        "*"
      );
    } catch {
      // cross-origin or unmounted — ignore
    }
  }, []);

  return (
    <div className="flex flex-col h-full relative">
      {/* macOS-style menu bar */}
      <PluginMenuBar plugin={plugin} disabled={isBusy} onStart={onStart} onOpenChange={setMenuOpen} />

      {/* Plugin content */}
      <div className="flex-1 relative">
        {/* Transparent overlay to capture clicks when menu is open (iframe swallows pointer events) */}
        {menuOpen && <div className="absolute inset-0 z-10" />}
        {isRunning && !isBusy && hasUi ? (
          <iframe
            src={iframeSrc!}
            className="w-full h-full border-0"
            title={plugin.manifest.name}
            data-nexus-plugin={plugin.manifest.id}
            sandbox="allow-scripts allow-same-origin allow-forms allow-popups"
            allow="clipboard-read; clipboard-write"
            onLoad={handleIframeLoad}
          />
        ) : isRunning && !isBusy && !hasUi ? (
          <HeadlessPluginStatus plugin={plugin} />
        ) : !isBusy ? (
          <div className="flex flex-col items-center justify-center h-full text-center">
            <div className="w-16 h-16 rounded-[var(--radius-modal)] bg-nx-surface flex items-center justify-center mb-4">
              <StopCircle size={28} strokeWidth={1.5} className="text-nx-text-ghost" />
            </div>
            <p className="text-[13px] text-nx-text-secondary mb-4">
              {plugin.status === "error"
                ? t("viewport.pluginError")
                : t("viewport.pluginStopped")}
            </p>
            <Button onClick={onStart}>
              <Play size={14} strokeWidth={1.5} />
              {t("viewport.startPlugin")}
            </Button>
          </div>
        ) : null}
      </div>

      {/* Busy overlay */}
      {busyAction && (
        <BusyOverlay action={busyAction} pluginName={plugin.manifest.name} />
      )}
    </div>
  );
}

function PluginMenuBar({ plugin, disabled, onOpenChange }: { plugin: InstalledPlugin; disabled: boolean; onStart: () => void; onOpenChange?: (open: boolean) => void }) {
  const { t } = useTranslation("plugins");
  const { setShowLogs } = useAppStore();
  const { start, stop, restart, remove, rebuild, toggleDevMode } = usePluginActions();
  const isRunning = plugin.status === "running";
  const isLocal = !!plugin.local_manifest_path;
  const id = plugin.manifest.id;
  const [removeDialogOpen, setRemoveDialogOpen] = useState(false);
  const [aboutDialogOpen, setAboutDialogOpen] = useState(false);

  const handleStart = () => start(id);
  const handleStop = () => stop(id);
  const handleRestart = () => restart(id);
  const handleRebuild = () => rebuild(id);
  const handleToggleDevMode = () => toggleDevMode(id, !plugin.dev_mode);
  const handleRemove = () => { setRemoveDialogOpen(false); remove(id); };

  const m = plugin.manifest;

  return (
    <>
      <Menubar
        className="rounded-none border-x-0 border-t-0 border-b border-nx-border bg-nx-raised/60 shadow-none px-2"
        onValueChange={(value) => onOpenChange?.(value !== "")}
      >
        {/* macOS-style app name menu */}
        <MenubarMenu>
          <MenubarTrigger className="font-semibold text-nx-text">
            {m.name}
          </MenubarTrigger>
          <MenubarContent>
            <MenubarItem onClick={() => setAboutDialogOpen(true)}>
              {t("menu.about", { name: m.name })}
            </MenubarItem>
            <MenubarSeparator />
            {isRunning ? (
              <>
                <MenubarItem onClick={handleRestart} disabled={disabled}>
                  <Play size={14} strokeWidth={1.5} className="text-nx-success" />
                  {t("common:action.restart")}
                </MenubarItem>
                <MenubarItem onClick={handleStop} disabled={disabled}>
                  <Square size={14} strokeWidth={1.5} className="text-nx-warning" />
                  {t("common:action.stop")}
                </MenubarItem>
              </>
            ) : (
              <MenubarItem onClick={handleStart} disabled={disabled}>
                <Play size={14} strokeWidth={1.5} className="text-nx-success" />
                {t("common:action.start")}
              </MenubarItem>
            )}
            <MenubarSeparator />
            <MenubarItem
              variant="destructive"
              onClick={() => setRemoveDialogOpen(true)}
              disabled={disabled}
            >
              <Trash2 size={14} strokeWidth={1.5} />
              {t("menu.remove", { name: m.name })}
            </MenubarItem>
          </MenubarContent>
        </MenubarMenu>

        <MenubarMenu>
          <MenubarTrigger className="text-nx-text-secondary">
            {t("menu.view")}
          </MenubarTrigger>
          <MenubarContent>
            <MenubarItem onClick={() => setShowLogs(id)}>
              <ScrollText size={14} strokeWidth={1.5} />
              {t("menu.logs")}
            </MenubarItem>
          </MenubarContent>
        </MenubarMenu>

        {isLocal && (
          <MenubarMenu>
            <MenubarTrigger className="text-nx-text-secondary">
              {t("menu.dev")}
            </MenubarTrigger>
            <MenubarContent>
              <MenubarItem onClick={handleRebuild} disabled={disabled}>
                <Hammer size={14} strokeWidth={1.5} className="text-nx-accent" />
                {t("menu.rebuild")}
              </MenubarItem>
              <MenubarSeparator />
              <MenubarCheckboxItem
                checked={plugin.dev_mode}
                onCheckedChange={handleToggleDevMode}
                disabled={disabled}
              >
                <Wrench size={14} strokeWidth={1.5} />
                {t("menu.autoRebuild")}
              </MenubarCheckboxItem>
            </MenubarContent>
          </MenubarMenu>
        )}
      </Menubar>

      {/* About dialog */}
      <Dialog open={aboutDialogOpen} onOpenChange={setAboutDialogOpen}>
        <DialogContent className="sm:max-w-sm">
          <DialogHeader className="items-center text-center">
            <div className="w-16 h-16 rounded-[var(--radius-modal)] bg-nx-surface flex items-center justify-center mb-2">
              {m.icon ? (
                <img src={m.icon} alt={m.name} className="w-10 h-10 rounded-md" />
              ) : (
                <Terminal size={28} strokeWidth={1.5} className="text-nx-accent" />
              )}
            </div>
            <DialogTitle className="text-base">{m.name}</DialogTitle>
            <DialogDescription className="text-[12px] text-nx-text-muted" asChild>
              <div className="space-y-3">
                <p>{m.description}</p>
                <div className="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1.5 text-left text-[11px]">
                  <span className="text-nx-text-ghost">{t("about.version")}</span>
                  <span className="font-mono text-nx-text-secondary">{m.version}</span>
                  <span className="text-nx-text-ghost">{t("about.author")}</span>
                  <span className="text-nx-text-secondary">{m.author}</span>
                  <span className="text-nx-text-ghost">{t("about.id")}</span>
                  <span className="font-mono text-nx-text-secondary">{m.id}</span>
                  {m.license && (
                    <>
                      <span className="text-nx-text-ghost">{t("about.license")}</span>
                      <span className="text-nx-text-secondary">{m.license}</span>
                    </>
                  )}
                  <span className="text-nx-text-ghost">{t("about.type")}</span>
                  <span className="text-nx-text-secondary">{m.ui ? t("about.uiPlugin") : t("about.headlessService")}</span>
                  {m.mcp && (
                    <>
                      <span className="text-nx-text-ghost">{t("viewport.mcpTools")}</span>
                      <span className="text-nx-text-secondary">{m.mcp.tools?.length ?? 0}</span>
                    </>
                  )}
                </div>
              </div>
            </DialogDescription>
          </DialogHeader>
        </DialogContent>
      </Dialog>

      {/* Remove confirmation dialog */}
      <Dialog open={removeDialogOpen} onOpenChange={setRemoveDialogOpen}>
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2 text-base">
              <TriangleAlert size={18} className="text-nx-warning" />
              {t("common:confirm.removePlugin", { name: m.name })}
            </DialogTitle>
            <DialogDescription className="text-[13px] leading-relaxed pt-1">
              {t("common:confirm.removePluginDesc")}
            </DialogDescription>
          </DialogHeader>
          <DialogFooter className="pt-2">
            <Button
              variant="secondary"
              size="sm"
              onClick={() => setRemoveDialogOpen(false)}
            >
              {t("common:action.cancel")}
            </Button>
            <Button
              variant="destructive"
              size="sm"
              onClick={handleRemove}
              className="bg-nx-error text-white hover:bg-nx-error/80"
            >
              {t("common:confirm.removeAndDeleteData")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}

function McpToolCard({ tool, onDetail }: {
  tool: McpToolDef;
  onDetail: (tool: McpToolDef) => void;
}) {
  const properties = (tool.input_schema?.properties ?? {}) as Record<string, { type?: string }>;
  const params = Object.keys(properties);

  return (
    <button
      onClick={() => onDetail(tool)}
      className="rounded-[var(--radius-card)] bg-nx-surface/60 border border-nx-border-subtle hover:border-nx-border-strong p-3.5 flex flex-col gap-2 text-left transition-colors cursor-pointer"
    >
      <div className="flex items-center gap-2">
        <Terminal size={13} strokeWidth={1.5} className="text-nx-accent shrink-0" />
        <span className="text-[12px] font-mono font-medium text-nx-accent truncate">{tool.name}</span>
        <Expand size={12} strokeWidth={1.5} className="ml-auto shrink-0 text-nx-text-ghost" />
      </div>
      {tool.description && (
        <p className="text-[11px] text-nx-text-secondary leading-relaxed line-clamp-3">
          {tool.description}
        </p>
      )}
      {params.length > 0 && (
        <div className="flex flex-wrap gap-1 mt-auto pt-1">
          {params.map((p) => (
            <span
              key={p}
              className="text-[9px] font-mono px-1.5 py-0.5 rounded bg-nx-overlay/60 text-nx-text-muted"
            >
              {p}
            </span>
          ))}
        </div>
      )}
    </button>
  );
}

function SchemaBlock({ label, schema }: { label: string; schema: Record<string, unknown> }) {
  return (
    <div>
      <p className="text-[11px] font-semibold text-nx-text-muted uppercase tracking-wider mb-1.5">
        {label}
      </p>
      <pre className="text-[11px] font-mono text-nx-text-secondary bg-nx-deep border border-nx-border rounded-[var(--radius-tag)] p-3 overflow-x-auto whitespace-pre-wrap break-words">
        {JSON.stringify(schema, null, 2)}
      </pre>
    </div>
  );
}

function McpToolDetailSheet({
  tool,
  open,
  onOpenChange,
}: {
  tool: McpToolDef | null;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const { t } = useTranslation("plugins");

  if (!tool) return null;

  const properties = (tool.input_schema?.properties ?? {}) as Record<string, { type?: string; description?: string }>;
  const required = (tool.input_schema?.required ?? []) as string[];
  const params = Object.entries(properties);

  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent
        side="right"
        className="bg-nx-base border-nx-border sm:max-w-md overflow-y-auto"
      >
        <SheetHeader>
          <div className="flex items-center gap-2">
            <Terminal size={15} strokeWidth={1.5} className="text-nx-accent" />
            <SheetTitle className="font-mono text-nx-accent text-[14px]">
              {tool.name}
            </SheetTitle>
          </div>
          {tool.description && (
            <SheetDescription className="text-nx-text-secondary text-[12px] leading-relaxed">
              {tool.description}
            </SheetDescription>
          )}
        </SheetHeader>

        <div className="flex flex-col gap-5 px-4 pb-6">
          {params.length > 0 && (
            <div>
              <p className="text-[11px] font-semibold text-nx-text-muted uppercase tracking-wider mb-2">
                {t("viewport.parameters")}
              </p>
              <div className="space-y-2">
                {params.map(([name, meta]) => (
                  <div
                    key={name}
                    className="rounded-[var(--radius-button)] bg-nx-surface/60 border border-nx-border-subtle px-3 py-2"
                  >
                    <div className="flex items-center gap-2">
                      <span className="text-[11px] font-mono font-medium text-nx-text">
                        {name}
                      </span>
                      {meta.type && (
                        <span className="text-[9px] font-mono px-1.5 py-0.5 rounded bg-nx-overlay/60 text-nx-text-muted">
                          {meta.type}
                        </span>
                      )}
                      {required.includes(name) && (
                        <span className="text-[9px] font-medium px-1.5 py-0.5 rounded bg-nx-accent-muted text-nx-accent">
                          {t("viewport.required")}
                        </span>
                      )}
                    </div>
                    {meta.description && (
                      <p className="text-[10px] text-nx-text-ghost mt-1 leading-relaxed">
                        {meta.description}
                      </p>
                    )}
                  </div>
                ))}
              </div>
            </div>
          )}

          {tool.input_schema && Object.keys(tool.input_schema).length > 0 && (
            <SchemaBlock label={t("viewport.inputSchema")} schema={tool.input_schema} />
          )}

          {tool.permissions.length > 0 && (
            <div>
              <p className="text-[11px] font-semibold text-nx-text-muted uppercase tracking-wider mb-1.5">
                {t("viewport.requiredPermissions")}
              </p>
              <div className="flex flex-wrap gap-1.5">
                {tool.permissions.map((p) => (
                  <span
                    key={p}
                    className="text-[10px] font-mono px-2 py-1 rounded-[var(--radius-tag)] bg-nx-warning-muted text-nx-warning border border-nx-warning/20"
                  >
                    {p}
                  </span>
                ))}
              </div>
            </div>
          )}
        </div>
      </SheetContent>
    </Sheet>
  );
}

function HeadlessPluginStatus({ plugin }: { plugin: InstalledPlugin }) {
  const { t } = useTranslation("plugins");
  const mcpTools = plugin.manifest.mcp?.tools ?? [];
  const [detailTool, setDetailTool] = useState<McpToolDef | null>(null);

  return (
    <div className="h-full overflow-y-auto p-6">
      <div className="flex flex-col items-center text-center mb-6">
        <div className="w-14 h-14 rounded-[var(--radius-modal)] bg-nx-surface flex items-center justify-center mb-3">
          <Terminal size={24} strokeWidth={1.5} className="text-nx-accent" />
        </div>
        <p className="text-[14px] font-semibold text-nx-text mb-1">
          {t("viewport.headlessRunning")}
        </p>
        <p className="text-[12px] text-nx-text-muted max-w-md">
          {t("viewport.headlessDesc", { count: mcpTools.length })}
        </p>
      </div>
      {mcpTools.length > 0 && (
        <div>
          <p className="text-[11px] font-semibold text-nx-text-muted uppercase tracking-wider mb-3">
            {t("viewport.mcpTools")}
          </p>
          <div className="grid grid-cols-[repeat(auto-fill,minmax(220px,1fr))] gap-2.5">
            {mcpTools.map((tool) => (
              <McpToolCard key={tool.name} tool={tool} onDetail={setDetailTool} />
            ))}
          </div>
        </div>
      )}

      <McpToolDetailSheet
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
      color: "text-nx-error",
      bg: "bg-nx-error-muted",
    },
    stopping: {
      icon: Square,
      label: t("overlay.stopping"),
      sub: t("overlay.stoppingSub"),
      color: "text-nx-warning",
      bg: "bg-nx-warning-muted",
    },
    starting: {
      icon: Play,
      label: t("overlay.starting"),
      sub: t("overlay.startingSub"),
      color: "text-nx-success",
      bg: "bg-nx-success-muted",
    },
    rebuilding: {
      icon: Hammer,
      label: t("overlay.rebuilding"),
      sub: t("overlay.rebuildingSub"),
      color: "text-nx-accent",
      bg: "bg-nx-accent-muted",
    },
  };

  const config = overlayConfig[action];
  const Icon = config.icon;

  return (
    <div className="absolute inset-0 z-50 flex flex-col items-center justify-center bg-nx-deep/90 backdrop-blur-sm">
      <div className="flex flex-col items-center gap-4">
        <div className={`w-16 h-16 rounded-[var(--radius-modal)] ${config.bg} flex items-center justify-center`}>
          <Icon size={28} strokeWidth={1.5} className={config.color} />
        </div>
        <div className="text-center">
          <p className="text-[14px] font-semibold text-nx-text mb-1">
            {config.label} {pluginName}
          </p>
          <p className="text-[12px] text-nx-text-muted">
            {config.sub}
          </p>
        </div>
        <Loader2 size={20} strokeWidth={1.5} className="text-nx-text-muted animate-spin" />
      </div>
    </div>
  );
}
