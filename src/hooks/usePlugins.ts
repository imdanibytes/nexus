import { useCallback, useEffect, useRef } from "react";
import { useAppStore } from "../stores/appStore";
import * as api from "../lib/tauri";

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
      addNotification(`Failed to load plugins: ${e}`, "error");
    }
  }, [setPlugins, addNotification]);

  const install = useCallback(
    async (manifestUrl: string) => {
      try {
        await api.pluginInstall(manifestUrl);
        addNotification("Plugin installed", "success");
        await refresh();
      } catch (e) {
        addNotification(`Install failed: ${e}`, "error");
      }
    },
    [refresh, addNotification]
  );

  const installLocal = useCallback(
    async (manifestPath: string) => {
      try {
        await api.pluginInstallLocal(manifestPath);
        addNotification("Plugin installed from local manifest", "success");
        await refresh();
      } catch (e) {
        addNotification(`Local install failed: ${e}`, "error");
      }
    },
    [refresh, addNotification]
  );

  const start = useCallback(
    async (pluginId: string) => {
      setBusy(pluginId, "starting");
      try {
        await api.pluginStart(pluginId);
        addNotification("Plugin started", "success");
        await refresh();
      } catch (e) {
        addNotification(`Start failed: ${e}`, "error");
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
        addNotification("Plugin stopped", "info");
        await refresh();
      } catch (e) {
        addNotification(`Stop failed: ${e}`, "error");
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
        addNotification("Plugin removed", "info");
      } catch (e) {
        addNotification(`Remove failed: ${e}`, "error");
      } finally {
        setBusy(pluginId, null);
      }
    },
    [removeFromStore, setBusy, addNotification]
  );

  const getLogs = useCallback(
    async (pluginId: string, tail?: number) => {
      try {
        return await api.pluginLogs(pluginId, tail);
      } catch (e) {
        addNotification(`Failed to get logs: ${e}`, "error");
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
    install,
    installLocal,
    start,
    stop,
    remove,
    getLogs,
  };
}
