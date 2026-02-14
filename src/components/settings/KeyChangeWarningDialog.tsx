import { AlertTriangle } from "lucide-react";
import type { AvailableUpdate } from "../../types/updates";

interface KeyChangeWarningDialogProps {
  update: AvailableUpdate;
  onCancel: () => void;
  onForceUpdate: (update: AvailableUpdate) => void;
}

export function KeyChangeWarningDialog({
  update,
  onCancel,
  onForceUpdate,
}: KeyChangeWarningDialogProps) {
  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <div className="bg-[var(--nx-surface)] rounded-[var(--radius-modal)] p-6 max-w-md w-full mx-4 shadow-2xl border border-nx-border">
        {/* Warning icon and title */}
        <div className="flex items-center gap-3 mb-4">
          <AlertTriangle size={20} strokeWidth={1.5} className="text-nx-error shrink-0" />
          <h3 className="text-[15px] font-semibold text-nx-text">
            Security Warning
          </h3>
        </div>

        {/* Explanation */}
        <p className="text-[12px] text-nx-text-secondary mb-4 leading-relaxed">
          The signing key for{" "}
          <strong className="text-nx-text">{update.item_name}</strong> has
          changed. This could indicate a compromised package or a legitimate key
          rotation by the author.
        </p>

        {/* Key details */}
        <div className="bg-nx-error-muted rounded-[var(--radius-card)] p-3 mb-6 text-[11px] font-mono text-nx-text-secondary space-y-1">
          <p>
            <span className="text-nx-text-muted">Extension:</span>{" "}
            {update.item_id}
          </p>
          <p>
            <span className="text-nx-text-muted">New version:</span>{" "}
            {update.available_version}
          </p>
          <p>
            <span className="text-nx-text-muted">Source:</span>{" "}
            {update.registry_source}
          </p>
        </div>

        {/* Actions — Cancel is prominent, force is destructive */}
        <div className="flex justify-end gap-3">
          <button
            onClick={onCancel}
            className="px-4 py-2 text-[12px] font-medium rounded-[var(--radius-button)] bg-nx-accent hover:bg-nx-accent-hover text-nx-deep transition-all duration-150"
          >
            Cancel
          </button>
          <button
            onClick={() => onForceUpdate(update)}
            className="px-4 py-2 text-[12px] font-medium rounded-[var(--radius-button)] border border-nx-error text-nx-error hover:bg-nx-error-muted transition-all duration-150"
          >
            I understand, update anyway
          </button>
        </div>
      </div>
    </div>
  );
}
