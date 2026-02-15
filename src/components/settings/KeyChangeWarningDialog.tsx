import { AlertTriangle } from "lucide-react";
import type { AvailableUpdate } from "../../types/updates";
import {
  AlertDialog,
  AlertDialogContent,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogAction,
  AlertDialogCancel,
} from "@/components/ui/alert-dialog";

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
    <AlertDialog open>
      <AlertDialogContent className="max-w-md">
        <AlertDialogHeader>
          <AlertDialogTitle className="flex items-center gap-3 text-[15px]">
            <AlertTriangle size={20} strokeWidth={1.5} className="text-nx-error shrink-0" />
            Security Warning
          </AlertDialogTitle>
          <AlertDialogDescription className="text-[12px] text-nx-text-secondary leading-relaxed">
            The signing key for{" "}
            <strong className="text-nx-text">{update.item_name}</strong> has
            changed. This could indicate a compromised package or a legitimate key
            rotation by the author.
          </AlertDialogDescription>
        </AlertDialogHeader>

        {/* Key details */}
        <div className="bg-nx-error-muted rounded-[var(--radius-card)] p-3 text-[11px] font-mono text-nx-text-secondary space-y-1">
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

        <AlertDialogFooter>
          <AlertDialogCancel onClick={onCancel} className="bg-primary text-primary-foreground hover:bg-primary/90">
            Cancel
          </AlertDialogCancel>
          <AlertDialogAction
            variant="outline"
            onClick={() => onForceUpdate(update)}
            className="border-nx-error text-nx-error hover:bg-nx-error-muted"
          >
            I understand, update anyway
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
