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
import { usePluginActions, usePluginSync } from "./hooks/usePlugins";
import { useExtensionActions, useExtensionSync } from "./hooks/useExtensions";
import { useLifecycleEvents } from "./hooks/useLifecycleEvents";
import { checkEngine, marketplaceRefresh, checkUpdates, getUpdateCheckInterval, pluginLogs } from "./lib/tauri";
import { Package } from "lucide-react";
import { Button } from "@heroui/react";
import { ErrorBoundary } from "./components/ErrorBoundary";
import { InstallOverlay } from "./components/InstallOverlay";
import { NexusProvider } from "@imdanibytes/nexus-ui";
import { useTranslation } from "react-i18next";
import i18n from "./i18n";

const VIEWPORT_TTL_MS = 5 * 60 * 1000; // evict after 5 min inactive
const EVICTION_CHECK_MS = 30_000;       // check every 30s

function PluginsView() {
  const { t } = useTranslation("common");
  const installedPlugins = useAppStore((s) => s.installedPlugins);
  const selectedPluginId = useAppStore((s) => s.selectedPluginId);
  const showLogsPluginId = useAppStore((s) => s.showLogsPluginId);

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
          <div className="w-20 h-20 rounded-[14px] bg-default-100 flex items-center justify-center mb-4">
            <Package size={36} strokeWidth={1.5} className="text-default-400" />
          </div>
          <h3 className="text-[16px] font-semibold text-default-500 mb-1">
            {installedPlugins.length === 0 ? t("empty.noPlugins") : t("empty.selectPlugin")}
          </h3>
          <p className="text-[13px] text-default-500 max-w-sm mb-4">
            {installedPlugins.length === 0
              ? t("empty.noPluginsHint")
              : t("empty.selectPluginHint")}
          </p>
          {installedPlugins.length === 0 && (
            <Button
              color="primary"
              onPress={() => useAppStore.getState().setView("marketplace")}
            >
              {t("nav.addPlugins")}
            </Button>
          )}
        </div>
      )}

      {/* Warm plugin viewports — stacked, only selected is visible. Evicted after 5 min idle. */}
      {installedPlugins.filter((p) => warmPluginIds.has(p.manifest.id)).map((plugin) => {
        const id = plugin.manifest.id;
        const isActive = id === selectedPluginId;
        return (
          <div
            key={id}
            className={`absolute inset-0 ${isActive ? "" : "invisible pointer-events-none"}`}
          >
            <ErrorBoundary label={plugin.manifest.name}>
              <PluginViewport pluginId={id} />
            </ErrorBoundary>
          </div>
        );
      })}

      <PluginLogs
        pluginId={showLogsPluginId}
        getLogs={(id, tail) => pluginLogs(id, tail)}
        onClose={() => useAppStore.getState().setShowLogs(null)}
      />
    </div>
  );
}

function App() {
  const currentView = useAppStore((s) => s.currentView);
  const selectedRegistryEntry = useAppStore((s) => s.selectedRegistryEntry);
  const selectedExtensionEntry = useAppStore((s) => s.selectedExtensionEntry);
  const installedPlugins = useAppStore((s) => s.installedPlugins);
  const updateCheckInterval = useAppStore((s) => s.updateCheckInterval);

  const { refresh } = usePluginActions();
  const { refresh: extensionRefresh } = useExtensionActions();
  useLifecycleEvents();
  usePluginSync();
  useExtensionSync();

  const checkForPluginUpdates = useCallback(async () => {
    try {
      await marketplaceRefresh();
      const updates = await checkUpdates();
      if (updates.length > 0) {
        const { setAvailableUpdates, dismissByCategory, notify } = useAppStore.getState();
        setAvailableUpdates(updates);
        dismissByCategory("updates.plugins");
        dismissByCategory("updates.extensions");
        for (const u of updates) {
          const cat = u.item_type === "plugin" ? "updates.plugins" : "updates.extensions";
          notify(cat, u.item_name, { data: u });
        }
      }
    } catch {
      // Silently ignore — offline or registry unreachable
    }
  }, []);

  // One-time startup: docker check, app update check, initial plugin check, load interval setting
  useEffect(() => {
    refresh();
    extensionRefresh();

    checkEngine()
      .then((status) => {
        const { addNotification, notify } = useAppStore.getState();
        if (!status.installed) {
          const msg = i18n.t("common:notification.engineNotFound");
          addNotification(msg, "error");
          notify("system.engine", msg);
        } else if (!status.running) {
          const msg = i18n.t("common:notification.engineNotRunning");
          addNotification(msg, "error");
          notify("system.engine", msg);
        }
      })
      .catch(() => {});

    import("@tauri-apps/plugin-updater")
      .then(({ check }) => check())
      .then((update) => {
        if (update) {
          const { addNotification, notify } = useAppStore.getState();
          const msg = i18n.t("common:notification.updateAvailable", { version: update.version });
          addNotification(msg, "info");
          notify("updates.app", msg, { data: update });
        }
      })
      .catch(() => {});

    checkForPluginUpdates();
    getUpdateCheckInterval()
      .then((interval) => useAppStore.getState().setUpdateCheckInterval(interval))
      .catch(() => {});
  }, [refresh, extensionRefresh, checkForPluginUpdates]);

  // Reactive timer — restarts whenever the interval setting changes
  useEffect(() => {
    if (updateCheckInterval <= 0) return;
    const id = setInterval(checkForPluginUpdates, updateCheckInterval * 60 * 1000);
    return () => clearInterval(id);
  }, [updateCheckInterval, checkForPluginUpdates]);

  return (
    <NexusProvider>
    <Shell>
      <InstallOverlay />
      {/* Always-mounted views use content-visibility:hidden to skip layout+paint when inactive.
         This is better than opacity-0 (which still paints) and visibility:hidden (which HeroUI
         descendants can override with visibility:visible). content-visibility:hidden creates
         containment that can't be overridden by children, and keeps DOM/iframes alive. */}
      <div
        className="absolute inset-0 overflow-y-auto"
        style={currentView !== "plugins" ? { contentVisibility: "hidden", pointerEvents: "none" } : undefined}
      >
        <ErrorBoundary label="Plugins">
          <PluginsView />
        </ErrorBoundary>
      </div>
      <div
        className="absolute inset-0 overflow-y-auto"
        style={currentView !== "settings" ? { contentVisibility: "hidden", pointerEvents: "none" } : undefined}
      >
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
                const s = useAppStore.getState();
                s.selectRegistryEntry(null);
                s.setView("marketplace");
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
                const s = useAppStore.getState();
                s.selectExtensionEntry(null);
                s.setView("extension-marketplace");
              }}
            />
          </ErrorBoundary>
        </div>
      )}
    </Shell>
    </NexusProvider>
  );
}

export default App;
