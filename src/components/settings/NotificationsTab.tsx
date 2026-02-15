import { useState } from "react";
import { Bell, BellOff } from "lucide-react";
import { Switch } from "@/components/ui/switch";
import {
  notificationsEnabled,
  setNotificationsEnabled,
} from "../../hooks/useOsNotification";

export function NotificationsTab() {
  const [enabled, setEnabled] = useState(notificationsEnabled);

  function handleToggle(checked: boolean) {
    setEnabled(checked);
    setNotificationsEnabled(checked);
  }

  return (
    <div className="space-y-6">
      <section className="bg-nx-surface rounded-[var(--radius-card)] border border-nx-border p-5">
        <div className="flex items-center gap-2 mb-4">
          <Bell size={15} strokeWidth={1.5} className="text-nx-text-muted" />
          <h3 className="text-[14px] font-semibold text-nx-text">
            OS Notifications
          </h3>
        </div>

        <div className="flex items-center justify-between">
          <div>
            <p className="text-[13px] text-nx-text">
              Show native notifications
            </p>
            <p className="text-[11px] text-nx-text-ghost mt-0.5">
              Approval requests trigger an OS notification when Nexus is not
              focused
            </p>
          </div>

          <Switch checked={enabled} onCheckedChange={handleToggle} />
        </div>

        {!enabled && (
          <div className="flex items-center gap-2 mt-4 pt-4 border-t border-nx-border-subtle">
            <BellOff
              size={13}
              strokeWidth={1.5}
              className="text-nx-text-ghost"
            />
            <p className="text-[11px] text-nx-text-ghost">
              Notifications are disabled. You will still see in-app approval
              dialogs.
            </p>
          </div>
        )}
      </section>
    </div>
  );
}
