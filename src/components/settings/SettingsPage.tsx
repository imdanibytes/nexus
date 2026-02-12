import { useEffect, useState } from "react";
import { useAppStore } from "../../stores/appStore";
import { appVersion, type AppVersionInfo } from "../../lib/tauri";
import { PermissionList } from "../permissions/PermissionList";
import { DockerSettings } from "./DockerSettings";
import { RegistrySettings } from "./RegistrySettings";
import { UpdateCheck } from "./UpdateCheck";
import { Info, Shield } from "lucide-react";

export function SettingsPage() {
  const { installedPlugins } = useAppStore();
  const [version, setVersion] = useState<AppVersionInfo | null>(null);

  useEffect(() => {
    appVersion().then(setVersion).catch(() => {});
  }, []);

  return (
    <div className="p-6 max-w-2xl mx-auto space-y-6">
      <div>
        <h2 className="text-[18px] font-bold text-nx-text mb-1">Settings</h2>
        <p className="text-[13px] text-nx-text-secondary">
          Manage your Nexus installation
        </p>
      </div>

      {/* Registries */}
      <RegistrySettings />

      {/* Docker */}
      <DockerSettings />

      {/* App info */}
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
        </div>
        <div className="mt-4 pt-4 border-t border-nx-border-subtle">
          <UpdateCheck />
        </div>
      </section>

      {/* Plugin permissions */}
      <section className="bg-nx-surface rounded-[var(--radius-card)] border border-nx-border p-5">
        <div className="flex items-center gap-2 mb-4">
          <Shield size={15} strokeWidth={1.5} className="text-nx-text-muted" />
          <h3 className="text-[14px] font-semibold text-nx-text">
            Plugin Permissions
          </h3>
        </div>
        {installedPlugins.length === 0 ? (
          <p className="text-[11px] text-nx-text-ghost">No plugins installed</p>
        ) : (
          <div className="space-y-5">
            {installedPlugins.map((plugin) => (
              <div key={plugin.manifest.id}>
                <h4 className="text-[13px] text-nx-text font-medium mb-2">
                  {plugin.manifest.name}
                </h4>
                <PermissionList pluginId={plugin.manifest.id} />
              </div>
            ))}
          </div>
        )}
      </section>
    </div>
  );
}
