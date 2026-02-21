import { useCallback, useEffect, useMemo, useState } from "react";
import { Shell } from "./components/layout/Shell";
import { PluginViewport } from "./components/plugins/PluginViewport";
import { PluginLogs } from "./components/plugins/PluginLogs";
import { MarketplacePage } from "./components/marketplace/MarketplacePage";
import { PluginDetail } from "./components/marketplace/PluginDetail";
import { SettingsPage } from "./components/settings/SettingsPage";
import { ExtensionMarketplacePage } from "./components/extensions/ExtensionMarketplacePage";
import { ExtensionDetail } from "./components/extensions/ExtensionDetail";
import { WorkflowsPage } from "./components/workflows/WorkflowsPage";
import { useAppStore } from "./stores/appStore";
import { usePluginActions, usePluginSync } from "./hooks/usePlugins";
import { useExtensionActions, useExtensionSync } from "./hooks/useExtensions";
import { useLifecycleEvents } from "./hooks/useLifecycleEvents";
import { useUpdateScheduler } from "./hooks/useUpdateScheduler";
import { checkEngine, pluginLogs } from "./lib/tauri";
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

  const handleMarketplacePress = useCallback(() => {
    useAppStore.getState().setView("marketplace");
  }, []);

  const handleLogsClose = useCallback(() => {
    useAppStore.getState().setShowLogs(null);
  }, []);

  const getLogsForPlugin = useCallback((id: string, tail: number) => pluginLogs(id, tail), []);

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
              onPress={handleMarketplacePress}
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
        getLogs={getLogsForPlugin}
        onClose={handleLogsClose}
      />
    </div>
  );
}

function App() {
  const currentView = useAppStore((s) => s.currentView);
  const selectedRegistryEntry = useAppStore((s) => s.selectedRegistryEntry);
  const selectedExtensionEntry = useAppStore((s) => s.selectedExtensionEntry);
  const installedPlugins = useAppStore((s) => s.installedPlugins);

  const { refresh } = usePluginActions();
  const { refresh: extensionRefresh } = useExtensionActions();
  useLifecycleEvents();
  usePluginSync();
  useExtensionSync();
  useUpdateScheduler();

  // One-time startup: docker check, app update check, plugin/extension list
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
  }, [refresh, extensionRefresh]);

  const pluginsStyle = useMemo(
    () => currentView !== "plugins" ? { contentVisibility: "hidden" as const, pointerEvents: "none" as const } : undefined,
    [currentView],
  );

  const settingsStyle = useMemo(
    () => currentView !== "settings" ? { contentVisibility: "hidden" as const, pointerEvents: "none" as const } : undefined,
    [currentView],
  );

  const installedPlugin = useMemo(
    () => selectedRegistryEntry
      ? (installedPlugins.find((p) => p.manifest.id === selectedRegistryEntry.id) ?? null)
      : null,
    [installedPlugins, selectedRegistryEntry],
  );

  const handlePluginDetailBack = useCallback(() => {
    const s = useAppStore.getState();
    s.selectRegistryEntry(null);
    s.setView("marketplace");
  }, []);

  const handleExtensionDetailBack = useCallback(() => {
    const s = useAppStore.getState();
    s.selectExtensionEntry(null);
    s.setView("extension-marketplace");
  }, []);

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
        style={pluginsStyle}
      >
        <ErrorBoundary label="Plugins">
          <PluginsView />
        </ErrorBoundary>
      </div>
      <div
        className="absolute inset-0 overflow-y-auto"
        style={settingsStyle}
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
              installedPlugin={installedPlugin}
              onBack={handlePluginDetailBack}
            />
          </ErrorBoundary>
        </div>
      )}
      {currentView === "workflows" && (
        <div className="absolute inset-0 overflow-y-auto">
          <ErrorBoundary label="Workflows">
            <WorkflowsPage />
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
              onBack={handleExtensionDetailBack}
            />
          </ErrorBoundary>
        </div>
      )}
    </Shell>
    </NexusProvider>
  );
}

export default App;
