import type { ReactNode } from "react";
import { Sidebar } from "./Sidebar";
import { RuntimeApprovalDialog } from "../permissions/RuntimeApprovalDialog";
import { useAppStore } from "../../stores/appStore";
import { X } from "lucide-react";

const toastBorder: Record<string, string> = {
  error: "border-l-nx-error",
  success: "border-l-nx-success",
  info: "border-l-nx-info",
};

const toastIcon: Record<string, string> = {
  error: "text-nx-error",
  success: "text-nx-success",
  info: "text-nx-info",
};

export function Shell({ children }: { children: ReactNode }) {
  const { notifications, removeNotification } = useAppStore();

  return (
    <div className="flex h-screen bg-nx-deep">
      <Sidebar />
      <main className="flex-1 overflow-y-auto bg-nx-base">{children}</main>

      <RuntimeApprovalDialog />

      {/* Toast notifications */}
      <div className="fixed bottom-4 right-4 space-y-2 z-50 max-w-sm">
        {notifications.map((notif) => (
          <div
            key={notif.id}
            style={{ animation: "toast-enter 300ms cubic-bezier(0.16,1,0.3,1)" }}
            className={`
              flex items-start gap-3 px-4 py-3
              bg-nx-raised border border-nx-border border-l-4
              ${toastBorder[notif.type] ?? "border-l-nx-info"}
              rounded-[var(--radius-card)] shadow-[var(--shadow-toast)]
            `}
          >
            <p className={`text-[13px] text-nx-text flex-1 leading-snug ${toastIcon[notif.type]}`}>
              {notif.message}
            </p>
            <button
              onClick={() => removeNotification(notif.id)}
              className="text-nx-text-muted hover:text-nx-text transition-colors shrink-0 mt-0.5"
            >
              <X size={14} strokeWidth={1.5} />
            </button>
          </div>
        ))}
      </div>
    </div>
  );
}
