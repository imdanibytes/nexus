import { useState } from "react";
import type { RegistryEntry } from "../../types/plugin";
import type { Permission } from "../../types/permissions";
import { PermissionDialog } from "../permissions/PermissionDialog";

interface Props {
  entry: RegistryEntry;
  isInstalled: boolean;
  onInstall: (manifestUrl: string, permissions: Permission[]) => void;
  onBack: () => void;
}

export function PluginDetail({
  entry,
  isInstalled,
  onInstall,
  onBack,
}: Props) {
  const [showPermissions, setShowPermissions] = useState(false);

  return (
    <div className="p-6 max-w-2xl mx-auto">
      <button
        onClick={onBack}
        className="flex items-center gap-1 text-sm text-slate-400 hover:text-white mb-6 transition-colors"
      >
        <svg
          className="w-4 h-4"
          fill="none"
          viewBox="0 0 24 24"
          stroke="currentColor"
        >
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth={2}
            d="M15 19l-7-7 7-7"
          />
        </svg>
        Back to Marketplace
      </button>

      <div className="bg-slate-800 rounded-xl border border-slate-700 p-6">
        <div className="flex items-start justify-between mb-4">
          <div>
            <h2 className="text-xl font-bold text-white">{entry.name}</h2>
            <p className="text-sm text-slate-400 mt-1">
              v{entry.version} &middot; {entry.id}
            </p>
          </div>
          {isInstalled ? (
            <span className="px-3 py-1.5 text-xs rounded-lg bg-indigo-500/20 text-indigo-400 font-medium">
              Installed
            </span>
          ) : (
            <button
              onClick={() => setShowPermissions(true)}
              className="px-4 py-2 bg-indigo-500 hover:bg-indigo-600 text-white text-sm rounded-lg transition-colors"
            >
              Install
            </button>
          )}
        </div>

        <p className="text-slate-300 text-sm mb-6">{entry.description}</p>

        <div className="space-y-4">
          <div>
            <h4 className="text-xs font-semibold text-slate-400 uppercase tracking-wider mb-2">
              Docker Image
            </h4>
            <code className="text-xs bg-slate-900 text-slate-300 px-2 py-1 rounded">
              {entry.image}
            </code>
          </div>

          {entry.categories.length > 0 && (
            <div>
              <h4 className="text-xs font-semibold text-slate-400 uppercase tracking-wider mb-2">
                Categories
              </h4>
              <div className="flex gap-2">
                {entry.categories.map((cat) => (
                  <span
                    key={cat}
                    className="text-xs px-2 py-1 rounded bg-slate-700 text-slate-300"
                  >
                    {cat}
                  </span>
                ))}
              </div>
            </div>
          )}
        </div>
      </div>

      {showPermissions && (
        <PermissionDialog
          pluginName={entry.name}
          requestedPermissions={[]}
          onApprove={(perms) => {
            onInstall(entry.manifest_url, perms);
            setShowPermissions(false);
          }}
          onDeny={() => setShowPermissions(false)}
        />
      )}
    </div>
  );
}
