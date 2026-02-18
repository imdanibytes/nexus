import { useCallback, useEffect, useRef } from "react";
import { useAppStore } from "../stores/appStore";
import * as api from "../lib/tauri";
import i18n from "../i18n";

const SYNC_INTERVAL_MS = 30_000;

export function useExtensions() {
  const {
    installedExtensions,
    busyExtensions,
    setExtensions,
    addNotification,
  } = useAppStore();

  // Fallback poll — lifecycle events handle most updates, this is a safety net
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

  const enable = useCallback(async (extId: string) => {
    await api.extensionEnable(extId);
  }, []);

  const disable = useCallback(async (extId: string) => {
    await api.extensionDisable(extId);
  }, []);

  const remove = useCallback(async (extId: string) => {
    await api.extensionRemove(extId);
  }, []);

  return {
    extensions: installedExtensions,
    busyExtensions,
    refresh,
    enable,
    disable,
    remove,
  };
}
