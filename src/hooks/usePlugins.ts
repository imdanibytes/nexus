import { useCallback, useEffect, useRef } from "react";
import { useAppStore } from "../stores/appStore";
import type { Permission } from "../types/permissions";
import type { PluginManifest } from "../types/plugin";
import * as api from "../lib/tauri";
import i18n from "../i18n";

const SYNC_INTERVAL_MS = 5000;

export function usePlugins() {
  const {
    installedPlugins,
    selectedPluginId,
    busyPlugins,
    setPlugins,
    selectPlugin,
    removePlugin: removeFromStore,
    setBusy,
    addNotification,
    setInstallStatus,
  } = useAppStore();

  const selectedPlugin = installedPlugins.find(
    (p) => p.manifest.id === selectedPluginId
  );

  // Poll Docker state every 5 seconds
  const syncingRef = useRef(false);
  useEffect(() => {
    const timer = setInterval(async () => {
      if (syncingRef.current) return;
      syncingRef.current = true;
      try {
        const plugins = await api.pluginSyncStatus();
        setPlugins(plugins);
      } catch {
        // Docker may not be running — silently ignore
      } finally {
        syncingRef.current = false;
      }
    }, SYNC_INTERVAL_MS);
    return () => clearInterval(timer);
  }, [setPlugins]);

  const refresh = useCallback(async () => {
    try {
      const plugins = await api.pluginList();
      setPlugins(plugins);
    } catch (e) {
      addNotification(i18n.t("error.loadPlugins", { error: e }), "error");
    }
  }, [setPlugins, addNotification]);

  // Step 1: Preview a manifest (local or remote) before installing
  const previewLocal = useCallback(
    async (manifestPath: string): Promise<PluginManifest | null> => {
      try {
        return await api.pluginPreviewLocal(manifestPath);
      } catch (e) {
        addNotification(i18n.t("error.readManifest", { error: e }), "error");
        return null;
      }
    },
    [addNotification]
  );

  const previewRemote = useCallback(
    async (manifestUrl: string): Promise<PluginManifest | null> => {
      try {
        return await api.pluginPreviewRemote(manifestUrl);
      } catch (e) {
        addNotification(i18n.t("error.fetchManifest", { error: e }), "error");
        return null;
      }
    },
    [addNotification]
  );

  // Step 2: Install with user-approved and deferred permissions
  const install = useCallback(
    async (manifestUrl: string, approvedPermissions: Permission[], deferredPermissions?: Permission[], buildContext?: string) => {
      setInstallStatus(buildContext ? i18n.t("plugins:installStatus.building") : i18n.t("plugins:installStatus.installing"));
      try {
        await api.pluginInstall(manifestUrl, approvedPermissions, deferredPermissions, buildContext);
        addNotification(i18n.t("notification.pluginInstalled"), "success");
        await refresh();
      } catch (e) {
        addNotification(i18n.t("error.installFailed", { error: e }), "error");
      } finally {
        setInstallStatus(null);
      }
    },
    [refresh, addNotification, setInstallStatus]
  );

  const installLocal = useCallback(
    async (manifestPath: string, approvedPermissions: Permission[], deferredPermissions?: Permission[]) => {
      setInstallStatus(i18n.t("plugins:installStatus.building"));
      try {
        await api.pluginInstallLocal(manifestPath, approvedPermissions, deferredPermissions);
        addNotification(i18n.t("notification.pluginInstalledLocal"), "success");
        await refresh();
      } catch (e) {
        addNotification(i18n.t("error.localInstallFailed", { error: e }), "error");
      } finally {
        setInstallStatus(null);
      }
    },
    [refresh, addNotification, setInstallStatus]
  );

  const start = useCallback(
    async (pluginId: string) => {
      setBusy(pluginId, "starting");
      try {
        await api.pluginStart(pluginId);
        addNotification(i18n.t("notification.pluginStarted"), "success");
        await refresh();
      } catch (e) {
        addNotification(i18n.t("error.startFailed", { error: e }), "error");
      } finally {
        setBusy(pluginId, null);
      }
    },
    [refresh, setBusy, addNotification]
  );

  const stop = useCallback(
    async (pluginId: string) => {
      setBusy(pluginId, "stopping");
      try {
        await api.pluginStop(pluginId);
        addNotification(i18n.t("notification.pluginStopped"), "info");
        await refresh();
      } catch (e) {
        addNotification(i18n.t("error.stopFailed", { error: e }), "error");
      } finally {
        setBusy(pluginId, null);
      }
    },
    [refresh, setBusy, addNotification]
  );

  const remove = useCallback(
    async (pluginId: string) => {
      setBusy(pluginId, "removing");
      try {
        await api.pluginRemove(pluginId);
        removeFromStore(pluginId);
        addNotification(i18n.t("notification.pluginRemoved"), "info");
      } catch (e) {
        addNotification(i18n.t("error.removeFailed", { error: e }), "error");
      } finally {
        setBusy(pluginId, null);
      }
    },
    [removeFromStore, setBusy, addNotification]
  );

  const restart = useCallback(
    async (pluginId: string) => {
      setBusy(pluginId, "stopping");
      try {
        await api.pluginStop(pluginId);
        setBusy(pluginId, "starting");
        await api.pluginStart(pluginId);
        addNotification(i18n.t("notification.pluginStarted"), "success");
        await refresh();
      } catch (e) {
        addNotification(i18n.t("error.startFailed", { error: e }), "error");
      } finally {
        setBusy(pluginId, null);
      }
    },
    [refresh, setBusy, addNotification]
  );

  const rebuild = useCallback(
    async (pluginId: string) => {
      setBusy(pluginId, "rebuilding");
      try {
        await api.pluginRebuild(pluginId);
        // Toast + busy clear handled by useDevRebuild() hook on "complete" event.
        // The command returns immediately (rebuild is spawned in background).
      } catch (e) {
        addNotification(i18n.t("error.rebuildFailed", { error: e }), "error");
        setBusy(pluginId, null);
      }
    },
    [setBusy, addNotification]
  );

  const toggleDevMode = useCallback(
    async (pluginId: string, enable: boolean) => {
      try {
        await api.pluginDevModeToggle(pluginId, enable);
        addNotification(
          enable
            ? i18n.t("notification.devModeEnabled")
            : i18n.t("notification.devModeDisabled"),
          "info"
        );
        await refresh();
      } catch (e) {
        addNotification(i18n.t("error.devModeToggleFailed", { error: e }), "error");
      }
    },
    [refresh, addNotification]
  );

  const getLogs = useCallback(
    async (pluginId: string, tail?: number) => {
      try {
        return await api.pluginLogs(pluginId, tail);
      } catch (e) {
        addNotification(i18n.t("error.logsFailed", { error: e }), "error");
        return [];
      }
    },
    [addNotification]
  );

  return {
    plugins: installedPlugins,
    selectedPlugin,
    selectedPluginId,
    busyPlugins,
    selectPlugin,
    refresh,
    previewLocal,
    previewRemote,
    install,
    installLocal,
    start,
    stop,
    restart,
    rebuild,
    toggleDevMode,
    remove,
    getLogs,
  };
}
