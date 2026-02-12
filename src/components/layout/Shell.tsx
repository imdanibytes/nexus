import type { ReactNode } from "react";
import { Sidebar } from "./Sidebar";
import { useAppStore } from "../../stores/appStore";

export function Shell({ children }: { children: ReactNode }) {
  const { notifications, removeNotification } = useAppStore();

  return (
    <div className="flex h-screen bg-slate-900">
      <Sidebar />
      <main className="flex-1 overflow-y-auto">{children}</main>

      {/* Toast notifications */}
      <div className="fixed bottom-4 right-4 space-y-2 z-50">
        {notifications.map((notif) => (
          <div
            key={notif.id}
            className={`px-4 py-2 rounded-lg shadow-lg text-sm flex items-center gap-2 cursor-pointer transition-opacity ${
              notif.type === "error"
                ? "bg-red-500/90 text-white"
                : notif.type === "success"
                  ? "bg-green-500/90 text-white"
                  : "bg-slate-700 text-slate-200"
            }`}
            onClick={() => removeNotification(notif.id)}
          >
            {notif.message}
          </div>
        ))}
      </div>
    </div>
  );
}
