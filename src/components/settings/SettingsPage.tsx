import { useEffect, useState } from "react";
import { useAppStore } from "../../stores/appStore";
import { appVersion, type AppVersionInfo } from "../../lib/tauri";
import { PermissionList } from "../permissions/PermissionList";
import { DockerSettings } from "./DockerSettings";
import { UpdateCheck } from "./UpdateCheck";

export function SettingsPage() {
  const { installedPlugins } = useAppStore();
  const [version, setVersion] = useState<AppVersionInfo | null>(null);

  useEffect(() => {
    appVersion().then(setVersion).catch(() => {});
  }, []);

  return (
    <div className="p-6 max-w-2xl mx-auto space-y-6">
      <div>
        <h2 className="text-lg font-bold text-white mb-1">Settings</h2>
        <p className="text-sm text-slate-400">
          Manage your Nexus installation
        </p>
      </div>

      {/* Docker */}
      <DockerSettings />

      {/* App info */}
      <section className="bg-slate-800 rounded-xl border border-slate-700 p-5">
        <h3 className="text-sm font-semibold text-white mb-3">About</h3>
        <div className="space-y-2 text-sm text-slate-300">
          <div className="flex justify-between">
            <span className="text-slate-400">Version</span>
            <span>{version?.version ?? "..."}</span>
          </div>
          <div className="flex justify-between">
            <span className="text-slate-400">App</span>
            <span>{version?.name ?? "Nexus"}</span>
          </div>
        </div>
        <div className="mt-4">
          <UpdateCheck />
        </div>
      </section>

      {/* Plugin permissions */}
      <section className="bg-slate-800 rounded-xl border border-slate-700 p-5">
        <h3 className="text-sm font-semibold text-white mb-3">
          Plugin Permissions
        </h3>
        {installedPlugins.length === 0 ? (
          <p className="text-xs text-slate-500">No plugins installed</p>
        ) : (
          <div className="space-y-4">
            {installedPlugins.map((plugin) => (
              <div key={plugin.manifest.id}>
                <h4 className="text-sm text-slate-200 mb-2">
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
