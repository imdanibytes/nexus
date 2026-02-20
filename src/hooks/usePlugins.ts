import { useCallback, useEffect, useRef } from "react";
import { useAppStore } from "../stores/appStore";
import type { Permission } from "../types/permissions";
import type { PluginManifest } from "../types/plugin";
import * as api from "../lib/tauri";
import i18n from "../i18n";

const SYNC_INTERVAL_MS = 30_000;

/**
 * Stable action callbacks for plugin operations.
 * Does NOT subscribe to the Zustand store — zero re-renders from state changes.
 * Use `useAppStore.getState()` inside callbacks to access store setters.
 */
export function usePluginActions() {
  const refresh = useCallback(async () => {
    try {
      const plugins = await api.pluginList();
      useAppStore.getState().setPlugins(plugins);
    } catch (e) {
      useAppStore.getState().addNotification(
        i18n.t("error.loadPlugins", { error: e }),
        "error",
      );
    }
  }, []);

  const previewLocal = useCallback(
    async (manifestPath: string): Promise<PluginManifest | null> => {
      try {
        return await api.pluginPreviewLocal(manifestPath);
      } catch (e) {
        useAppStore.getState().addNotification(
          i18n.t("error.readManifest", { error: e }),
          "error",
        );
        return null;
      }
    },
    [],
  );

  const previewRemote = useCallback(
    async (manifestUrl: string): Promise<PluginManifest | null> => {
      try {
        return await api.pluginPreviewRemote(manifestUrl);
      } catch (e) {
        useAppStore.getState().addNotification(
          i18n.t("error.fetchManifest", { error: e }),
          "error",
        );
        return null;
      }
    },
    [],
  );

  const install = useCallback(
    async (
      manifestUrl: string,
      approvedPermissions: Permission[],
      deferredPermissions?: Permission[],
      buildContext?: string,
    ) => {
      await api.pluginInstall(manifestUrl, approvedPermissions, deferredPermissions, buildContext);
    },
    [],
  );

  const installLocal = useCallback(
    async (
      manifestPath: string,
      approvedPermissions: Permission[],
      deferredPermissions?: Permission[],
    ) => {
      await api.pluginInstallLocal(manifestPath, approvedPermissions, deferredPermissions);
    },
    [],
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
        useAppStore.getState().addNotification(
          enable
            ? i18n.t("notification.devModeEnabled")
            : i18n.t("notification.devModeDisabled"),
          "info",
        );
        const plugins = await api.pluginList();
        useAppStore.getState().setPlugins(plugins);
      } catch (e) {
        useAppStore.getState().addNotification(
          i18n.t("error.devModeToggleFailed", { error: e }),
          "error",
        );
      }
    },
    [],
  );

  const getLogs = useCallback(
    async (pluginId: string, tail?: number) => {
      try {
        return await api.pluginLogs(pluginId, tail);
      } catch (e) {
        useAppStore.getState().addNotification(
          i18n.t("error.logsFailed", { error: e }),
          "error",
        );
        return [];
      }
    },
    [],
  );

  return {
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

/**
 * 30-second fallback poll for plugin status.
 * Call exactly ONCE in the app root — do not use in child components.
 * Does not subscribe to the Zustand store.
 */
export function usePluginSync() {
  const syncingRef = useRef(false);
  useEffect(() => {
    const timer = setInterval(async () => {
      if (syncingRef.current) return;
      syncingRef.current = true;
      try {
        const plugins = await api.pluginSyncStatus();
        useAppStore.getState().setPlugins(plugins);
      } catch {
        // Docker may not be running — silently ignore
      } finally {
        syncingRef.current = false;
      }
    }, SYNC_INTERVAL_MS);
    return () => clearInterval(timer);
  }, []);
}
