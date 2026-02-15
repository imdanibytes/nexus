import { useEffect, useState } from "react";
import { appVersion, type AppVersionInfo } from "../../lib/tauri";
import { RegistrySettings } from "./RegistrySettings";
import { UpdateCheck } from "./UpdateCheck";
import { Info, Bug, Bell, BellOff } from "lucide-react";
import { Switch } from "@/components/ui/switch";
import {
  notificationsEnabled,
  setNotificationsEnabled,
} from "../../hooks/useOsNotification";

export function GeneralTab() {
  const [version, setVersion] = useState<AppVersionInfo | null>(null);
  const [notifEnabled, setNotifEnabled] = useState(notificationsEnabled);

  useEffect(() => {
    appVersion().then(setVersion).catch(() => {});
  }, []);

  function handleNotifToggle(checked: boolean) {
    setNotifEnabled(checked);
    setNotificationsEnabled(checked);
  }

  return (
    <div className="space-y-6">
      {/* About */}
      <section className="bg-nx-surface rounded-[var(--radius-card)] border border-nx-border p-5">
        <div className="flex items-center gap-2 mb-4">
          <Info size={15} strokeWidth={1.5} className="text-nx-text-muted" />
          <h3 className="text-[14px] font-semibold text-nx-text">About</h3>
        </div>
        <div className="space-y-2.5">
          <div className="flex justify-between items-center">
            <span className="text-[12px] text-nx-text-muted">Version</span>
            <span className="text-[13px] text-nx-text font-mono">
              {version?.version ?? "..."}
            </span>
          </div>
          <div className="flex justify-between items-center">
            <span className="text-[12px] text-nx-text-muted">App</span>
            <span className="text-[13px] text-nx-text">
              {version?.name ?? "Nexus"}
            </span>
          </div>
          {version?.commit && (
            <div className="flex justify-between items-center">
              <span className="text-[12px] text-nx-text-muted">Build</span>
              <span className="text-[13px] text-nx-text font-mono">
                {version.commit}
              </span>
            </div>
          )}
        </div>
        <div className="mt-4 pt-4 border-t border-nx-border-subtle">
          <UpdateCheck />
        </div>
        <div className="mt-4 pt-4 border-t border-nx-border-subtle flex items-center justify-between">
          <div>
            <span className="text-[12px] text-nx-text-muted">Found a problem?</span>
          </div>
          <a
            href={`https://github.com/imdanibytes/nexus/issues/new?template=bug_report.md&labels=bug&title=&body=${encodeURIComponent(`**Nexus Version:** ${version?.version ?? "unknown"}\n**OS:** ${navigator.platform}\n\n**Describe the bug**\n\n\n**Steps to reproduce**\n1. \n2. \n3. \n\n**Expected behavior**\n\n\n**Screenshots**\n`)}`}
            target="_blank"
            rel="noopener noreferrer"
            className="flex items-center gap-1.5 px-3 py-1.5 text-[12px] font-medium text-nx-text-muted hover:text-nx-text rounded-[var(--radius-button)] border border-nx-border hover:bg-nx-wash/40 transition-all duration-150"
          >
            <Bug size={13} strokeWidth={1.5} />
            Report a Bug
          </a>
        </div>
      </section>

      {/* Notifications */}
      <section className="bg-nx-surface rounded-[var(--radius-card)] border border-nx-border p-5">
        <div className="flex items-center gap-2 mb-4">
          <Bell size={15} strokeWidth={1.5} className="text-nx-text-muted" />
          <h3 className="text-[14px] font-semibold text-nx-text">
            Notifications
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
          <Switch checked={notifEnabled} onCheckedChange={handleNotifToggle} />
        </div>
        {!notifEnabled && (
          <div className="flex items-center gap-2 mt-4 pt-4 border-t border-nx-border-subtle">
            <BellOff size={13} strokeWidth={1.5} className="text-nx-text-ghost" />
            <p className="text-[11px] text-nx-text-ghost">
              Notifications are disabled. You will still see in-app approval
              dialogs.
            </p>
          </div>
        )}
      </section>

      {/* Registries */}
      <RegistrySettings />
    </div>
  );
}
