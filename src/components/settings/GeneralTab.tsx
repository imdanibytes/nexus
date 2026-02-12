import { useEffect, useState } from "react";
import { appVersion, type AppVersionInfo } from "../../lib/tauri";
import { RegistrySettings } from "./RegistrySettings";
import { UpdateCheck } from "./UpdateCheck";
import { Info } from "lucide-react";

export function GeneralTab() {
  const [version, setVersion] = useState<AppVersionInfo | null>(null);

  useEffect(() => {
    appVersion().then(setVersion).catch(() => {});
  }, []);

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
      </section>

      {/* Registries */}
      <RegistrySettings />
    </div>
  );
}
