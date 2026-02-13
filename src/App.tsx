import { useEffect, useState } from "react";
import { Shell } from "./components/layout/Shell";
import { PluginViewport } from "./components/plugins/PluginViewport";
import { PluginLogs } from "./components/plugins/PluginLogs";
import { MarketplacePage } from "./components/marketplace/MarketplacePage";
import { PluginDetail } from "./components/marketplace/PluginDetail";
import { SettingsPage } from "./components/settings/SettingsPage";
import { ExtensionMarketplacePage } from "./components/extensions/ExtensionMarketplacePage";
import { ExtensionDetail } from "./components/extensions/ExtensionDetail";
import { useAppStore } from "./stores/appStore";
import { usePlugins } from "./hooks/usePlugins";
import { checkDocker } from "./lib/tauri";
import { Package } from "lucide-react";
import { ErrorBoundary } from "./components/ErrorBoundary";

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
      <ErrorBoundary label={selectedPlugin.manifest.name}>
        <PluginViewport
          plugin={selectedPlugin}
          busyAction={busyPlugins[selectedPlugin.manifest.id] ?? null}
          onStart={() => start(selectedPlugin.manifest.id)}
          onStop={() => stop(selectedPlugin.manifest.id)}
          onRemove={() => remove(selectedPlugin.manifest.id)}
          onShowLogs={() => setShowLogs(selectedPlugin.manifest.id)}
        />
      </ErrorBoundary>
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
    selectedExtensionEntry,
    installedPlugins,
    setView,
    selectRegistryEntry,
    selectExtensionEntry,
  } = useAppStore();
  const { refresh } = usePlugins();
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

    // Background update check — non-blocking, just notifies
    import("@tauri-apps/plugin-updater")
      .then(({ check }) => check())
      .then((update) => {
        if (update) {
          addNotification(
            `Update available: v${update.version}. Go to Settings to install.`,
            "info"
          );
        }
      })
      .catch(() => {});
  }, []);

  const installedIds = new Set(installedPlugins.map((p) => p.manifest.id));

  return (
    <Shell>
      {currentView === "plugins" && (
        <ErrorBoundary label="Plugins">
          <PluginsView />
        </ErrorBoundary>
      )}
      {currentView === "marketplace" && (
        <ErrorBoundary label="Marketplace">
          <MarketplacePage />
        </ErrorBoundary>
      )}
      {currentView === "settings" && (
        <ErrorBoundary label="Settings">
          <SettingsPage />
        </ErrorBoundary>
      )}
      {currentView === "plugin-detail" && selectedRegistryEntry && (
        <ErrorBoundary label="Plugin Detail">
          <PluginDetail
            entry={selectedRegistryEntry}
            isInstalled={installedIds.has(selectedRegistryEntry.id)}
            onBack={() => {
              selectRegistryEntry(null);
              setView("marketplace");
            }}
          />
        </ErrorBoundary>
      )}
      {currentView === "extension-marketplace" && (
        <ErrorBoundary label="Extension Marketplace">
          <ExtensionMarketplacePage />
        </ErrorBoundary>
      )}
      {currentView === "extension-detail" && selectedExtensionEntry && (
        <ErrorBoundary label="Extension Detail">
          <ExtensionDetail
            entry={selectedExtensionEntry}
            onBack={() => {
              selectExtensionEntry(null);
              setView("extension-marketplace");
            }}
          />
        </ErrorBoundary>
      )}
    </Shell>
  );
}

export default App;
