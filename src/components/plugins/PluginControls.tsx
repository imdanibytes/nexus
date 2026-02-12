import { useState } from "react";
import type { PluginStatus } from "../../types/plugin";

interface Props {
  status: PluginStatus;
  onStart: () => void;
  onStop: () => void;
  onRemove: () => void;
  onShowLogs: () => void;
}

export function PluginControls({
  status,
  onStart,
  onStop,
  onRemove,
  onShowLogs,
}: Props) {
  const [showConfirm, setShowConfirm] = useState(false);

  return (
    <div className="flex items-center gap-2">
      <button
        onClick={onShowLogs}
        className="px-3 py-1.5 text-xs rounded-lg bg-slate-700 hover:bg-slate-600 text-slate-300 transition-colors"
        title="View logs"
      >
        Logs
      </button>

      {status === "running" ? (
        <button
          onClick={onStop}
          className="px-3 py-1.5 text-xs rounded-lg bg-yellow-500/20 hover:bg-yellow-500/30 text-yellow-400 transition-colors"
        >
          Stop
        </button>
      ) : (
        <button
          onClick={onStart}
          className="px-3 py-1.5 text-xs rounded-lg bg-green-500/20 hover:bg-green-500/30 text-green-400 transition-colors"
        >
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
            className="px-3 py-1.5 text-xs rounded-lg bg-red-500 hover:bg-red-600 text-white transition-colors"
          >
            Confirm
          </button>
          <button
            onClick={() => setShowConfirm(false)}
            className="px-3 py-1.5 text-xs rounded-lg bg-slate-700 hover:bg-slate-600 text-slate-300 transition-colors"
          >
            Cancel
          </button>
        </div>
      ) : (
        <button
          onClick={() => setShowConfirm(true)}
          className="px-3 py-1.5 text-xs rounded-lg bg-red-500/20 hover:bg-red-500/30 text-red-400 transition-colors"
        >
          Remove
        </button>
      )}
    </div>
  );
}
