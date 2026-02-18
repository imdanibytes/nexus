import { useCallback, useEffect, useRef } from "react";
import { useAppStore } from "../stores/appStore";
import type { Permission } from "../types/permissions";
import type { PluginManifest } from "../types/plugin";
import * as api from "../lib/tauri";
import i18n from "../i18n";

const SYNC_INTERVAL_MS = 30_000;

export function usePlugins() {
  const {
    installedPlugins,
    selectedPluginId,
    busyPlugins,
    setPlugins,
    selectPlugin,
    addNotification,
  } = useAppStore();

  const selectedPlugin = installedPlugins.find(
    (p) => p.manifest.id === selectedPluginId
  );

  // Fallback poll — lifecycle events handle most updates, this is a safety net
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

  // Step 2: Install — fire and forget, lifecycle events handle status
  const install = useCallback(
    async (manifestUrl: string, approvedPermissions: Permission[], deferredPermissions?: Permission[], buildContext?: string) => {
      await api.pluginInstall(manifestUrl, approvedPermissions, deferredPermissions, buildContext);
    },
    []
  );

  const installLocal = useCallback(
    async (manifestPath: string, approvedPermissions: Permission[], deferredPermissions?: Permission[]) => {
      await api.pluginInstallLocal(manifestPath, approvedPermissions, deferredPermissions);
    },
    []
  );

  const start = useCallback(async (pluginId: string) => {
    await api.pluginStart(pluginId);
  }, []);

  const stop = useCallback(async (pluginId: string) => {
    await api.pluginStop(pluginId);
  }, []);

  const remove = useCallback(async (pluginId: string) => {
    await api.pluginRemove(pluginId);
  }, []);

  const restart = useCallback(async (pluginId: string) => {
    await api.pluginStop(pluginId);
    await api.pluginStart(pluginId);
  }, []);

  const rebuild = useCallback(async (pluginId: string) => {
    await api.pluginRebuild(pluginId);
  }, []);

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
        // No lifecycle event for dev mode — refresh to pick up the change
        const plugins = await api.pluginList();
        setPlugins(plugins);
      } catch (e) {
        addNotification(i18n.t("error.devModeToggleFailed", { error: e }), "error");
      }
    },
    [setPlugins, addNotification]
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
