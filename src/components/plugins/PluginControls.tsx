import { useState } from "react";
import type { PluginStatus } from "../../types/plugin";
import { Play, Square, Trash2, ScrollText } from "lucide-react";

interface Props {
  status: PluginStatus;
  disabled?: boolean;
  onStart: () => void;
  onStop: () => void;
  onRemove: () => void;
  onShowLogs: () => void;
}

export function PluginControls({
  status,
  disabled = false,
  onStart,
  onStop,
  onRemove,
  onShowLogs,
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
