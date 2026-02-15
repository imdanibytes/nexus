import { useCallback, useEffect, useState } from "react";
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
import { useExtensions } from "./hooks/useExtensions";
import { useDevRebuild } from "./hooks/useDevRebuild";
import { checkEngine, marketplaceRefresh, checkUpdates, getUpdateCheckInterval, pluginLogs } from "./lib/tauri";
import { Package } from "lucide-react";
import { ErrorBoundary } from "./components/ErrorBoundary";
import { InstallOverlay } from "./components/InstallOverlay";
import { TooltipProvider } from "@/components/ui/tooltip";
import { useTranslation } from "react-i18next";
import i18n from "./i18n";

const VIEWPORT_TTL_MS = 5 * 60 * 1000; // evict after 5 min inactive
const EVICTION_CHECK_MS = 30_000;       // check every 30s

function PluginsView() {
  const { t } = useTranslation("common");
  const { plugins, selectedPluginId, busyPlugins, start } =
    usePlugins();
  const { setView, showLogsPluginId, setShowLogs } = useAppStore();

  // Track warm viewports: plugin ID → last-active timestamp
  const [warmEntries, setWarmEntries] = useState<Record<string, number>>({});

  // Adjust during render: mark the selected plugin as warm
  const [prevSelectedId, setPrevSelectedId] = useState<string | null>(null);
  if (selectedPluginId && selectedPluginId !== prevSelectedId) {
    setPrevSelectedId(selectedPluginId);
    setWarmEntries((prev) => ({ ...prev, [selectedPluginId]: Date.now() }));
  }

  // Periodic eviction of stale viewports
  useEffect(() => {
    const timer = setInterval(() => {
      const now = Date.now();
      const currentId = useAppStore.getState().selectedPluginId;
      setWarmEntries((prev) => {
        const next: Record<string, number> = {};
        let changed = false;
        for (const [id, ts] of Object.entries(prev)) {
          if (id === currentId || now - ts < VIEWPORT_TTL_MS) {
            next[id] = ts;
          } else {
            changed = true;
          }
        }
        return changed ? next : prev;
      });
    }, EVICTION_CHECK_MS);
    return () => clearInterval(timer);
  }, []);

  const warmPluginIds = new Set(Object.keys(warmEntries));

  // Sync warm set to store so the sidebar can read it
  useEffect(() => {
    useAppStore.getState().setWarmViewports(Object.keys(warmEntries));
  }, [warmEntries]);

  return (
    <div className="relative h-full">
      {/* Empty / select prompt — shown when no plugin is selected */}
      {!selectedPluginId && (
        <div className="flex flex-col items-center justify-center h-full text-center p-6">
          <div className="w-20 h-20 rounded-[var(--radius-modal)] bg-nx-surface flex items-center justify-center mb-4">
            <Package size={36} strokeWidth={1.5} className="text-nx-text-ghost" />
          </div>
          <h3 className="text-[16px] font-semibold text-nx-text-secondary mb-1">
            {plugins.length === 0 ? t("empty.noPlugins") : t("empty.selectPlugin")}
          </h3>
          <p className="text-[13px] text-nx-text-muted max-w-sm mb-4">
            {plugins.length === 0
              ? t("empty.noPluginsHint")
              : t("empty.selectPluginHint")}
          </p>
          {plugins.length === 0 && (
            <button
              onClick={() => setView("marketplace")}
              className="px-4 py-2 bg-nx-accent hover:bg-nx-accent-hover text-nx-deep text-[13px] font-medium rounded-[var(--radius-button)] transition-all duration-150"
            >
              {t("nav.addPlugins")}
            </button>
          )}
        </div>
      )}

      {/* Warm plugin viewports — stacked, only selected is visible. Evicted after 5 min idle. */}
      {plugins.filter((p) => warmPluginIds.has(p.manifest.id)).map((plugin) => {
        const id = plugin.manifest.id;
        const isActive = id === selectedPluginId;
        return (
          <div
            key={id}
            className={`absolute inset-0 ${isActive ? "" : "invisible pointer-events-none"}`}
          >
            <ErrorBoundary label={plugin.manifest.name}>
              <PluginViewport
                plugin={plugin}
                busyAction={busyPlugins[id] ?? null}
                onStart={() => start(id)}
              />
            </ErrorBoundary>
          </div>
        );
      })}

      {showLogsPluginId && (
        <PluginLogs
          pluginId={showLogsPluginId}
          getLogs={(id, tail) => pluginLogs(id, tail)}
          onClose={() => setShowLogs(null)}
        />
      )}
    </div>
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
  const { refresh: extensionRefresh } = useExtensions();
  useDevRebuild();
  const { addNotification, setAvailableUpdates, updateCheckInterval, setUpdateCheckInterval } = useAppStore();

  const checkForPluginUpdates = useCallback(async () => {
    try {
      await marketplaceRefresh();
      const updates = await checkUpdates();
      if (updates.length > 0) {
        setAvailableUpdates(updates);
      }
    } catch {
      // Silently ignore — offline or registry unreachable
    }
  }, [setAvailableUpdates]);

  // One-time startup: docker check, app update check, initial plugin check, load interval setting
  useEffect(() => {
    refresh();
    extensionRefresh();

    checkEngine()
      .then((status) => {
        if (!status.installed) {
          addNotification(
            i18n.t("common:notification.engineNotFound"),
            "error"
          );
        } else if (!status.running) {
          addNotification(
            i18n.t("common:notification.engineNotRunning"),
            "error"
          );
        }
      })
      .catch(() => {});

    import("@tauri-apps/plugin-updater")
      .then(({ check }) => check())
      .then((update) => {
        if (update) {
          addNotification(
            i18n.t("common:notification.updateAvailable", { version: update.version }),
            "info"
          );
        }
      })
      .catch(() => {});

    checkForPluginUpdates();
    getUpdateCheckInterval()
      .then(setUpdateCheckInterval)
      .catch(() => {});
  }, [refresh, extensionRefresh, addNotification, checkForPluginUpdates, setUpdateCheckInterval]);

  // Reactive timer — restarts whenever the interval setting changes
  useEffect(() => {
    if (updateCheckInterval <= 0) return;
    const id = setInterval(checkForPluginUpdates, updateCheckInterval * 60 * 1000);
    return () => clearInterval(id);
  }, [updateCheckInterval, checkForPluginUpdates]);

  return (
    <TooltipProvider>
    <Shell>
      <InstallOverlay />
      {/* Always-mounted views — stacked absolutely, hidden with visibility to preserve iframe state */}
      <div className={`absolute inset-0 overflow-y-auto ${currentView === "plugins" ? "" : "invisible pointer-events-none"}`}>
        <ErrorBoundary label="Plugins">
          <PluginsView />
        </ErrorBoundary>
      </div>
      <div className={`absolute inset-0 overflow-y-auto ${currentView === "settings" ? "" : "invisible pointer-events-none"}`}>
        <ErrorBoundary label="Settings">
          <SettingsPage />
        </ErrorBoundary>
      </div>
      {/* Ephemeral views — mount/unmount on demand */}
      {currentView === "marketplace" && (
        <div className="absolute inset-0 overflow-y-auto">
          <ErrorBoundary label="Marketplace">
            <MarketplacePage />
          </ErrorBoundary>
        </div>
      )}
      {currentView === "plugin-detail" && selectedRegistryEntry && (
        <div className="absolute inset-0 overflow-y-auto">
          <ErrorBoundary label="Plugin Detail">
            <PluginDetail
              entry={selectedRegistryEntry}
              installedPlugin={installedPlugins.find((p) => p.manifest.id === selectedRegistryEntry.id) ?? null}
              onBack={() => {
                selectRegistryEntry(null);
                setView("marketplace");
              }}
            />
          </ErrorBoundary>
        </div>
      )}
      {currentView === "extension-marketplace" && (
        <div className="absolute inset-0 overflow-y-auto">
          <ErrorBoundary label="Extension Marketplace">
            <ExtensionMarketplacePage />
          </ErrorBoundary>
        </div>
      )}
      {currentView === "extension-detail" && selectedExtensionEntry && (
        <div className="absolute inset-0 overflow-y-auto">
          <ErrorBoundary label="Extension Detail">
            <ExtensionDetail
              entry={selectedExtensionEntry}
              onBack={() => {
                selectExtensionEntry(null);
                setView("extension-marketplace");
              }}
            />
          </ErrorBoundary>
        </div>
      )}
    </Shell>
    </TooltipProvider>
  );
}

export default App;
