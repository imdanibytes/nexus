import { useCallback, useEffect, useRef } from "react";
import { useAppStore } from "../stores/appStore";
import * as api from "../lib/tauri";
import i18n from "../i18n";

const SYNC_INTERVAL_MS = 10_000;

export function useExtensions() {
  const {
    installedExtensions,
    busyExtensions,
    setExtensions,
    removeExtension: removeFromStore,
    setExtensionBusy,
    addNotification,
  } = useAppStore();

  // Poll extension state every 10 seconds
  const syncingRef = useRef(false);
  useEffect(() => {
    const timer = setInterval(async () => {
      if (syncingRef.current) return;
      syncingRef.current = true;
      try {
        const exts = await api.extensionList();
        setExtensions(exts);
      } catch {
        // Extension system may not be available — silently ignore
      } finally {
        syncingRef.current = false;
      }
    }, SYNC_INTERVAL_MS);
    return () => clearInterval(timer);
  }, [setExtensions]);

  const refresh = useCallback(async () => {
    try {
      const exts = await api.extensionList();
      setExtensions(exts);
    } catch (e) {
      addNotification(i18n.t("error.loadExtensions", { error: e }), "error");
    }
  }, [setExtensions, addNotification]);

  const enable = useCallback(
    async (extId: string) => {
      setExtensionBusy(extId, "enabling");
      try {
        await api.extensionEnable(extId);
        addNotification(i18n.t("notification.extensionEnabled"), "success");
        await refresh();
      } catch (e) {
        addNotification(i18n.t("error.enableFailed", { error: e }), "error");
      } finally {
        setExtensionBusy(extId, null);
      }
    },
    [refresh, setExtensionBusy, addNotification]
  );

  const disable = useCallback(
    async (extId: string) => {
      setExtensionBusy(extId, "disabling");
      try {
        await api.extensionDisable(extId);
        addNotification(i18n.t("notification.extensionDisabled"), "info");
        await refresh();
      } catch (e) {
        addNotification(i18n.t("error.disableFailed", { error: e }), "error");
      } finally {
        setExtensionBusy(extId, null);
      }
    },
    [refresh, setExtensionBusy, addNotification]
  );

  const remove = useCallback(
    async (extId: string) => {
      setExtensionBusy(extId, "removing");
      try {
        await api.extensionRemove(extId);
        removeFromStore(extId);
        addNotification(i18n.t("notification.extensionRemoved"), "info");
      } catch (e) {
        addNotification(i18n.t("error.removeFailed", { error: e }), "error");
      } finally {
        setExtensionBusy(extId, null);
      }
    },
    [removeFromStore, setExtensionBusy, addNotification]
  );

  return {
    extensions: installedExtensions,
    busyExtensions,
    refresh,
    enable,
    disable,
    remove,
  };
}
