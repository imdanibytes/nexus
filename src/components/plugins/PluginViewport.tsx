import { useState } from "react";
import type { InstalledPlugin } from "../../types/plugin";
import type { PluginAction } from "../../stores/appStore";
import { PluginControls } from "./PluginControls";
import { Play, StopCircle, Loader2, Trash2, Square, Terminal, Hammer } from "lucide-react";

const overlayConfig: Record<
  PluginAction,
  { icon: typeof Trash2; label: string; sub: string; color: string; bg: string }
> = {
  removing: {
    icon: Trash2,
    label: "Removing",
    sub: "Stopping container and cleaning up...",
    color: "text-nx-error",
    bg: "bg-nx-error-muted",
  },
  stopping: {
    icon: Square,
    label: "Stopping",
    sub: "Sending shutdown signal...",
    color: "text-nx-warning",
    bg: "bg-nx-warning-muted",
  },
  starting: {
    icon: Play,
    label: "Starting",
    sub: "Launching container...",
    color: "text-nx-success",
    bg: "bg-nx-success-muted",
  },
  rebuilding: {
    icon: Hammer,
    label: "Rebuilding",
    sub: "Building image and restarting...",
    color: "text-nx-accent",
    bg: "bg-nx-accent-muted",
  },
};

interface Props {
  plugin: InstalledPlugin;
  busyAction: PluginAction | null;
  onStart: () => void;
  onStop: () => void;
  onRemove: () => void;
  onShowLogs: () => void;
  onRebuild?: () => void;
  onToggleDevMode?: (enabled: boolean) => void;
}

export function PluginViewport({
  plugin,
  busyAction,
  onStart,
  onStop,
  onRemove,
  onShowLogs,
  onRebuild,
  onToggleDevMode,
}: Props) {
  const isRunning = plugin.status === "running";
  const isBusy = busyAction !== null;
  const hasUi = plugin.manifest.ui !== null;
  const iframeSrc = hasUi
    ? `http://localhost:${plugin.assigned_port}${plugin.manifest.ui!.path}`
    : null;

  return (
    <div className="flex flex-col h-full relative">
      {/* Plugin header */}
      <div className="flex items-center justify-between px-5 py-3 bg-nx-raised/60 border-b border-nx-border">
        <div>
          <h3 className="text-[13px] font-semibold text-nx-text">
            {plugin.manifest.name}
          </h3>
          <p className="text-[11px] text-nx-text-muted">
            {plugin.manifest.author} &middot; <span className="font-mono">v{plugin.manifest.version}</span>
          </p>
        </div>
        <PluginControls
          status={plugin.status}
          disabled={isBusy}
          isLocal={!!plugin.local_manifest_path}
          devMode={plugin.dev_mode}
          onStart={onStart}
          onStop={onStop}
          onRemove={onRemove}
          onShowLogs={onShowLogs}
          onRebuild={onRebuild}
          onToggleDevMode={onToggleDevMode}
        />
      </div>

      {/* Plugin content */}
      <div className="flex-1 relative">
        {isRunning && !isBusy && hasUi ? (
          <iframe
            src={iframeSrc!}
            className="w-full h-full border-0"
            title={plugin.manifest.name}
            sandbox="allow-scripts allow-same-origin allow-forms allow-popups"
            allow="clipboard-read; clipboard-write"
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
                ? "Plugin encountered an error"
                : "Plugin is stopped"}
            </p>
            <button
              onClick={onStart}
              className="flex items-center gap-2 px-4 py-2 bg-nx-accent hover:bg-nx-accent-hover text-nx-deep text-[13px] font-medium rounded-[var(--radius-button)] transition-all duration-150"
            >
              <Play size={14} strokeWidth={1.5} />
              Start Plugin
            </button>
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

function McpToolRow({ name, description }: { name: string; description?: string }) {
  const [expanded, setExpanded] = useState(false);
  const isLong = !!description && description.length > 100;

  return (
    <div className="px-3 py-2 rounded-[var(--radius-button)] bg-nx-surface/50">
      <div className="flex items-start gap-2">
        <span className="text-[11px] font-mono text-nx-accent shrink-0">{name}</span>
        {description && (
          <span className={`text-[10px] text-nx-text-ghost leading-tight ${expanded ? "" : "line-clamp-2"}`}>
            {description}
          </span>
        )}
      </div>
      {isLong && (
        <button
          onClick={() => setExpanded(!expanded)}
          className="text-[10px] text-nx-accent hover:text-nx-accent-hover mt-1"
        >
          {expanded ? "Show less" : "Show more"}
        </button>
      )}
    </div>
  );
}

function HeadlessPluginStatus({ plugin }: { plugin: InstalledPlugin }) {
  const mcpTools = plugin.manifest.mcp?.tools ?? [];

  return (
    <div className="flex flex-col items-center justify-center h-full text-center px-8">
      <div className="w-16 h-16 rounded-[var(--radius-modal)] bg-nx-surface flex items-center justify-center mb-4">
        <Terminal size={28} strokeWidth={1.5} className="text-nx-accent" />
      </div>
      <p className="text-[14px] font-semibold text-nx-text mb-2">
        Headless Service Running
      </p>
      <p className="text-[12px] text-nx-text-muted max-w-md mb-6">
        This plugin runs without a UI. It provides {mcpTools.length}{" "}
        {mcpTools.length === 1 ? "tool" : "tools"} to AI assistants via the
        Model Context Protocol.
      </p>
      {mcpTools.length > 0 && (
        <div className="text-left w-full max-w-sm">
          <p className="text-[11px] font-semibold text-nx-text-muted uppercase tracking-wider mb-2">
            MCP Tools
          </p>
          <div className="space-y-1.5">
            {mcpTools.map((tool) => (
              <McpToolRow key={tool.name} name={tool.name} description={tool.description} />
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

function BusyOverlay({ action, pluginName }: { action: PluginAction; pluginName: string }) {
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
