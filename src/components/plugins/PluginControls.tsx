import { useState } from "react";
import type { PluginStatus } from "../../types/plugin";
import { Play, Square, Trash2, ScrollText, Hammer, Wrench } from "lucide-react";
import { Button } from "@/components/ui/button";

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
      <Button
        variant="secondary"
        size="xs"
        onClick={onShowLogs}
        disabled={disabled}
        title="View logs"
      >
        <ScrollText size={12} strokeWidth={1.5} />
        Logs
      </Button>

      {isLocal && onRebuild && (
        <Button
          variant="secondary"
          size="xs"
          onClick={onRebuild}
          disabled={disabled}
          className="bg-nx-accent-muted text-nx-accent hover:bg-nx-accent/20"
          title="Rebuild from source"
        >
          <Hammer size={12} strokeWidth={1.5} />
          Rebuild
        </Button>
      )}

      {isLocal && onToggleDevMode && (
        <Button
          variant={devMode ? "default" : "secondary"}
          size="xs"
          onClick={() => onToggleDevMode(!devMode)}
          disabled={disabled}
          title={devMode ? "Disable dev mode (auto-rebuild on file changes)" : "Enable dev mode (auto-rebuild on file changes)"}
        >
          <Wrench size={12} strokeWidth={1.5} />
          Dev
        </Button>
      )}

      {status === "running" ? (
        <Button
          variant="secondary"
          size="xs"
          onClick={onStop}
          disabled={disabled}
          className="bg-nx-warning-muted text-nx-warning hover:bg-nx-warning/20"
        >
          <Square size={12} strokeWidth={1.5} />
          Stop
        </Button>
      ) : (
        <Button
          variant="secondary"
          size="xs"
          onClick={onStart}
          disabled={disabled}
          className="bg-nx-success-muted text-nx-success hover:bg-nx-success/20"
        >
          <Play size={12} strokeWidth={1.5} />
          Start
        </Button>
      )}

      {showConfirm ? (
        <div className="flex items-center gap-1">
          <Button
            variant="destructive"
            size="xs"
            onClick={() => {
              onRemove();
              setShowConfirm(false);
            }}
            disabled={disabled}
            className="bg-nx-error text-white hover:bg-nx-error/80"
          >
            Confirm
          </Button>
          <Button
            variant="secondary"
            size="xs"
            onClick={() => setShowConfirm(false)}
            disabled={disabled}
          >
            Cancel
          </Button>
        </div>
      ) : (
        <Button
          variant="destructive"
          size="xs"
          onClick={() => setShowConfirm(true)}
          disabled={disabled}
        >
          <Trash2 size={12} strokeWidth={1.5} />
          Remove
        </Button>
      )}
    </div>
  );
}
