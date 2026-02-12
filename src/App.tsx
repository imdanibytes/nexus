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
import { Package } from "lucide-react";

function PluginsView() {
  const { plugins, selectedPlugin, busyPlugins, start, stop, remove, getLogs } =
    usePlugins();
  const { setView } = useAppStore();
  const [showLogs, setShowLogs] = useState<string | null>(null);

  if (!selectedPlugin) {
    return (
      <div className="flex flex-col items-center justify-center h-full text-center p-6">
        <div className="w-20 h-20 rounded-[var(--radius-modal)] bg-nx-surface flex items-center justify-center mb-4">
          <Package size={36} strokeWidth={1.5} className="text-nx-text-ghost" />
        </div>
        <h3 className="text-[16px] font-semibold text-nx-text-secondary mb-1">
          {plugins.length === 0 ? "No plugins installed" : "Select a plugin"}
        </h3>
        <p className="text-[13px] text-nx-text-muted max-w-sm mb-4">
          {plugins.length === 0
            ? "Get started by adding a plugin from the marketplace or a local manifest."
            : "Click on a plugin in the sidebar to view it here."}
        </p>
        {plugins.length === 0 && (
          <button
            onClick={() => setView("marketplace")}
            className="px-4 py-2 bg-nx-accent hover:bg-nx-accent-hover text-nx-deep text-[13px] font-medium rounded-[var(--radius-button)] transition-all duration-150"
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
        busyAction={busyPlugins[selectedPlugin.manifest.id] ?? null}
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
