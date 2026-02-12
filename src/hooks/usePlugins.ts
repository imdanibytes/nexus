import { useCallback } from "react";
import { useAppStore } from "../stores/appStore";
import * as api from "../lib/tauri";

export function usePlugins() {
  const {
    installedPlugins,
    selectedPluginId,
    setPlugins,
    selectPlugin,
    removePlugin: removeFromStore,
    addNotification,
  } = useAppStore();

  const selectedPlugin = installedPlugins.find(
    (p) => p.manifest.id === selectedPluginId
  );

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
      try {
        await api.pluginStart(pluginId);
        addNotification("Plugin started", "success");
        await refresh();
      } catch (e) {
        addNotification(`Start failed: ${e}`, "error");
      }
    },
    [refresh, addNotification]
  );

  const stop = useCallback(
    async (pluginId: string) => {
      try {
        await api.pluginStop(pluginId);
        addNotification("Plugin stopped", "info");
        await refresh();
      } catch (e) {
        addNotification(`Stop failed: ${e}`, "error");
      }
    },
    [refresh, addNotification]
  );

  const remove = useCallback(
    async (pluginId: string) => {
      try {
        await api.pluginRemove(pluginId);
        removeFromStore(pluginId);
        addNotification("Plugin removed", "info");
      } catch (e) {
        addNotification(`Remove failed: ${e}`, "error");
      }
    },
    [removeFromStore, addNotification]
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
