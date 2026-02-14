import { Loader2 } from "lucide-react";
import { useAppStore } from "../stores/appStore";

export function InstallOverlay() {
  const { installStatus } = useAppStore();

  if (!installStatus.active) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm">
      <div className="flex flex-col items-center gap-4 px-8 py-6 rounded-[var(--radius-modal)] bg-nx-surface border border-nx-border shadow-lg">
        <Loader2 size={28} strokeWidth={1.5} className="animate-spin text-nx-accent" />
        <p className="text-[13px] font-medium text-nx-text">
          {installStatus.message}
        </p>
      </div>
    </div>
  );
}
