import { useCallback, useEffect, useRef } from "react";
import { useAppStore } from "../stores/appStore";
import type { Permission } from "../types/permissions";
import type { PluginManifest } from "../types/plugin";
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

  // Step 1: Preview a manifest (local or remote) before installing
  const previewLocal = useCallback(
    async (manifestPath: string): Promise<PluginManifest | null> => {
      try {
        return await api.pluginPreviewLocal(manifestPath);
      } catch (e) {
        addNotification(`Failed to read manifest: ${e}`, "error");
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
        addNotification(`Failed to fetch manifest: ${e}`, "error");
        return null;
      }
    },
    [addNotification]
  );

  // Step 2: Install with user-approved and deferred permissions
  const install = useCallback(
    async (manifestUrl: string, approvedPermissions: Permission[], deferredPermissions?: Permission[]) => {
      try {
        await api.pluginInstall(manifestUrl, approvedPermissions, deferredPermissions);
        addNotification("Plugin installed", "success");
        await refresh();
      } catch (e) {
        addNotification(`Install failed: ${e}`, "error");
      }
    },
    [refresh, addNotification]
  );

  const installLocal = useCallback(
    async (manifestPath: string, approvedPermissions: Permission[], deferredPermissions?: Permission[]) => {
      try {
        await api.pluginInstallLocal(manifestPath, approvedPermissions, deferredPermissions);
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
    previewLocal,
    previewRemote,
    install,
    installLocal,
    start,
    stop,
    remove,
    getLogs,
  };
}
