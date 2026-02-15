import { useState } from "react";
import type { PluginStatus } from "../../types/plugin";
import { Play, Square, Trash2, ScrollText, Hammer, Wrench } from "lucide-react";

interface Props {
  status: PluginStatus;
  disabled?: boolean;
  isLocal?: boolean;
  devMode?: boolean;
  onStart: () => void;
  onStop: () => void;
  onRemove: () => void;
  onShowLogs: () => void;
  onRebuild?: () => void;
  onToggleDevMode?: (enabled: boolean) => void;
}

export function PluginControls({
  status,
  disabled = false,
  isLocal = false,
  devMode = false,
  onStart,
  onStop,
  onRemove,
  onShowLogs,
  onRebuild,
  onToggleDevMode,
}: Props) {
  const [showConfirm, setShowConfirm] = useState(false);

  return (
    <div className={`flex items-center gap-2 ${disabled ? "opacity-40 pointer-events-none" : ""}`}>
      <button
        onClick={onShowLogs}
        disabled={disabled}
        className="flex items-center gap-1.5 px-2.5 py-1.5 text-[11px] font-medium rounded-[var(--radius-button)] bg-nx-overlay hover:bg-nx-wash text-nx-text-secondary transition-all duration-150"
        title="View logs"
      >
        <ScrollText size={12} strokeWidth={1.5} />
        Logs
      </button>

      {isLocal && onRebuild && (
        <button
          onClick={onRebuild}
          disabled={disabled}
          className="flex items-center gap-1.5 px-2.5 py-1.5 text-[11px] font-medium rounded-[var(--radius-button)] bg-nx-accent-muted hover:bg-nx-accent/20 text-nx-accent transition-all duration-150"
          title="Rebuild from source"
        >
          <Hammer size={12} strokeWidth={1.5} />
          Rebuild
        </button>
      )}

      {isLocal && onToggleDevMode && (
        <button
          onClick={() => onToggleDevMode(!devMode)}
          disabled={disabled}
          className={`flex items-center gap-1.5 px-2.5 py-1.5 text-[11px] font-medium rounded-[var(--radius-button)] transition-all duration-150 ${
            devMode
              ? "bg-nx-accent text-nx-deep"
              : "bg-nx-overlay hover:bg-nx-wash text-nx-text-secondary"
          }`}
          title={devMode ? "Disable dev mode (auto-rebuild on file changes)" : "Enable dev mode (auto-rebuild on file changes)"}
        >
          <Wrench size={12} strokeWidth={1.5} />
          Dev
        </button>
      )}

      {status === "running" ? (
        <button
          onClick={onStop}
          disabled={disabled}
          className="flex items-center gap-1.5 px-2.5 py-1.5 text-[11px] font-medium rounded-[var(--radius-button)] bg-nx-warning-muted hover:bg-nx-warning/20 text-nx-warning transition-all duration-150"
        >
          <Square size={12} strokeWidth={1.5} />
          Stop
        </button>
      ) : (
        <button
          onClick={onStart}
          disabled={disabled}
          className="flex items-center gap-1.5 px-2.5 py-1.5 text-[11px] font-medium rounded-[var(--radius-button)] bg-nx-success-muted hover:bg-nx-success/20 text-nx-success transition-all duration-150"
        >
          <Play size={12} strokeWidth={1.5} />
          Start
        </button>
      )}

      {showConfirm ? (
        <div className="flex items-center gap-1">
          <button
            onClick={() => {
              onRemove();
              setShowConfirm(false);
            }}
            disabled={disabled}
            className="px-2.5 py-1.5 text-[11px] font-medium rounded-[var(--radius-button)] bg-nx-error hover:bg-nx-error/80 text-white transition-all duration-150"
          >
            Confirm
          </button>
          <button
            onClick={() => setShowConfirm(false)}
            disabled={disabled}
            className="px-2.5 py-1.5 text-[11px] font-medium rounded-[var(--radius-button)] bg-nx-overlay hover:bg-nx-wash text-nx-text-secondary transition-all duration-150"
          >
            Cancel
          </button>
        </div>
      ) : (
        <button
          onClick={() => setShowConfirm(true)}
          disabled={disabled}
          className="flex items-center gap-1.5 px-2.5 py-1.5 text-[11px] font-medium rounded-[var(--radius-button)] bg-nx-error-muted hover:bg-nx-error/20 text-nx-error transition-all duration-150"
        >
          <Trash2 size={12} strokeWidth={1.5} />
          Remove
        </button>
      )}
    </div>
  );
}
