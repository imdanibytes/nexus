import { useEffect, useState } from "react";
import { Shell } from "./components/layout/Shell";
import { PluginViewport } from "./components/plugins/PluginViewport";
import { PluginLogs } from "./components/plugins/PluginLogs";
import { MarketplacePage } from "./components/marketplace/MarketplacePage";
import { PluginDetail } from "./components/marketplace/PluginDetail";
import { SettingsPage } from "./components/settings/SettingsPage";
import { useAppStore } from "./stores/appStore";
import { usePlugins } from "./hooks/usePlugins";
import { checkDocker } from "./lib/tauri";
import type { Permission } from "./types/permissions";

function PluginsView() {
  const { plugins, selectedPlugin, start, stop, remove, getLogs } =
    usePlugins();
  const { setView } = useAppStore();
  const [showLogs, setShowLogs] = useState<string | null>(null);

  if (!selectedPlugin) {
    return (
      <div className="flex flex-col items-center justify-center h-full text-center p-6">
        <div className="w-20 h-20 rounded-2xl bg-slate-800 flex items-center justify-center mb-4">
          <svg
            className="w-10 h-10 text-slate-600"
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
        <h3 className="text-lg font-semibold text-slate-300 mb-1">
          {plugins.length === 0 ? "No plugins installed" : "Select a plugin"}
        </h3>
        <p className="text-sm text-slate-500 max-w-sm mb-4">
          {plugins.length === 0
            ? "Get started by adding a plugin from the marketplace or a local manifest."
            : "Click on a plugin in the sidebar to view it here."}
        </p>
        {plugins.length === 0 && (
          <button
            onClick={() => setView("marketplace")}
            className="px-4 py-2 bg-indigo-500 hover:bg-indigo-600 text-white text-sm rounded-lg transition-colors"
          >
            Add Plugins
          </button>
        )}
      </div>
    );
  }

  return (
    <>
      <PluginViewport
        plugin={selectedPlugin}
        onStart={() => start(selectedPlugin.manifest.id)}
        onStop={() => stop(selectedPlugin.manifest.id)}
        onRemove={() => remove(selectedPlugin.manifest.id)}
        onShowLogs={() => setShowLogs(selectedPlugin.manifest.id)}
      />
      {showLogs && (
        <PluginLogs
          pluginId={showLogs}
          getLogs={getLogs}
          onClose={() => setShowLogs(null)}
        />
      )}
    </>
  );
}

function App() {
  const {
    currentView,
    selectedRegistryEntry,
    installedPlugins,
    setView,
    selectRegistryEntry,
  } = useAppStore();
  const { refresh, install } = usePlugins();
  const { addNotification } = useAppStore();

  useEffect(() => {
    refresh();

    checkDocker()
      .then((status) => {
        if (!status.running) {
          addNotification(
            "Docker is not running. Check Settings > Docker.",
            "error"
          );
        }
      })
      .catch(() => {});
  }, []);

  const installedIds = new Set(installedPlugins.map((p) => p.manifest.id));

  function handleInstall(manifestUrl: string, _permissions: Permission[]) {
    install(manifestUrl);
    setView("plugins");
  }

  return (
    <Shell>
      {currentView === "plugins" && <PluginsView />}
      {currentView === "marketplace" && <MarketplacePage />}
      {currentView === "settings" && <SettingsPage />}
      {currentView === "plugin-detail" && selectedRegistryEntry && (
        <PluginDetail
          entry={selectedRegistryEntry}
          isInstalled={installedIds.has(selectedRegistryEntry.id)}
          onInstall={handleInstall}
          onBack={() => {
            selectRegistryEntry(null);
            setView("marketplace");
          }}
        />
      )}
    </Shell>
  );
}

export default App;
