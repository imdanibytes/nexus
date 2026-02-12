import type { InstalledPlugin } from "../../types/plugin";
import { PluginControls } from "./PluginControls";
import { Play, StopCircle, Loader2, Trash2 } from "lucide-react";

interface Props {
  plugin: InstalledPlugin;
  isRemoving: boolean;
  onStart: () => void;
  onStop: () => void;
  onRemove: () => void;
  onShowLogs: () => void;
}

export function PluginViewport({
  plugin,
  isRemoving,
  onStart,
  onStop,
  onRemove,
  onShowLogs,
}: Props) {
  const isRunning = plugin.status === "running";
  const iframeSrc = `http://localhost:${plugin.assigned_port}${plugin.manifest.ui.path}`;

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
          disabled={isRemoving}
          onStart={onStart}
          onStop={onStop}
          onRemove={onRemove}
          onShowLogs={onShowLogs}
        />
      </div>

      {/* Plugin content */}
      <div className="flex-1 relative">
        {isRunning && !isRemoving ? (
          <iframe
            src={iframeSrc}
            className="w-full h-full border-0"
            title={plugin.manifest.name}
            sandbox="allow-scripts allow-same-origin allow-forms allow-popups"
            allow="clipboard-read; clipboard-write"
          />
        ) : !isRemoving ? (
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

      {/* Removing overlay */}
      {isRemoving && (
        <div className="absolute inset-0 z-50 flex flex-col items-center justify-center bg-nx-deep/90 backdrop-blur-sm">
          <div className="flex flex-col items-center gap-4">
            <div className="w-16 h-16 rounded-[var(--radius-modal)] bg-nx-error-muted flex items-center justify-center">
              <Trash2 size={28} strokeWidth={1.5} className="text-nx-error" />
            </div>
            <div className="text-center">
              <p className="text-[14px] font-semibold text-nx-text mb-1">
                Removing {plugin.manifest.name}
              </p>
              <p className="text-[12px] text-nx-text-muted">
                Stopping container and cleaning up...
              </p>
            </div>
            <Loader2 size={20} strokeWidth={1.5} className="text-nx-text-muted animate-spin" />
          </div>
        </div>
      )}
    </div>
  );
}
