import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { useMarketplace } from "../../hooks/useMarketplace";
import { usePlugins } from "../../hooks/usePlugins";
import { useAppStore } from "../../stores/appStore";
import { RegistryPluginCard } from "../plugins/PluginCard";
import { SearchBar } from "./SearchBar";

export function MarketplacePage() {
  const { plugins, isLoading, refresh, search } = useMarketplace();
  const { installLocal } = usePlugins();
  const { installedPlugins, selectRegistryEntry, setView } = useAppStore();
  const [installing, setInstalling] = useState(false);

  const installedIds = new Set(installedPlugins.map((p) => p.manifest.id));

  useEffect(() => {
    refresh();
  }, [refresh]);

  async function handleLocalInstall() {
    const selected = await open({
      multiple: false,
      filters: [{ name: "Plugin Manifest", extensions: ["json"] }],
    });
    if (selected) {
      setInstalling(true);
      await installLocal(selected);
      setInstalling(false);
      setView("plugins");
    }
  }

  return (
    <div className="p-6">
      <div className="flex items-center justify-between mb-6">
        <div>
          <h2 className="text-lg font-bold text-white">Add Plugins</h2>
          <p className="text-sm text-slate-400">
            Browse the marketplace or install from a local manifest
          </p>
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={handleLocalInstall}
            disabled={installing}
            className="px-3 py-1.5 text-xs rounded-lg bg-indigo-500 hover:bg-indigo-600 disabled:opacity-50 text-white transition-colors"
          >
            {installing ? "Installing..." : "Install Local"}
          </button>
          <button
            onClick={refresh}
            disabled={isLoading}
            className="px-3 py-1.5 text-xs rounded-lg bg-slate-700 hover:bg-slate-600 text-slate-300 transition-colors disabled:opacity-50"
          >
            {isLoading ? "Refreshing..." : "Refresh"}
          </button>
        </div>
      </div>

      <div className="mb-6">
        <SearchBar onSearch={search} />
      </div>

      {plugins.length === 0 ? (
        <div className="text-center py-16">
          <div className="w-16 h-16 rounded-2xl bg-slate-800 flex items-center justify-center mb-4 mx-auto">
            <svg
              className="w-8 h-8 text-slate-600"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={1.5}
                d="M20 7l-8-4-8 4m16 0l-8 4m8-4v10l-8 4m0-10L4 7m8 4v10M4 7v10l8 4"
              />
            </svg>
          </div>
          <p className="text-slate-400 text-sm mb-1">
            {isLoading
              ? "Loading plugins..."
              : "No marketplace plugins available yet."}
          </p>
          <p className="text-slate-500 text-xs mb-4">
            You can install a plugin from a local <code className="bg-slate-800 px-1 rounded">plugin.json</code> manifest.
          </p>
          <button
            onClick={handleLocalInstall}
            disabled={installing}
            className="px-4 py-2 bg-indigo-500 hover:bg-indigo-600 disabled:opacity-50 text-white text-sm rounded-lg transition-colors"
          >
            {installing ? "Installing..." : "Install Local Plugin"}
          </button>
        </div>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          {plugins.map((entry) => (
            <RegistryPluginCard
              key={entry.id}
              entry={entry}
              isInstalled={installedIds.has(entry.id)}
              onSelect={() => {
                selectRegistryEntry(entry);
                setView("plugin-detail");
              }}
            />
          ))}
        </div>
      )}
    </div>
  );
}
