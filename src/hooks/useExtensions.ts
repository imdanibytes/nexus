import { useCallback, useEffect, useRef } from "react";
import { useAppStore } from "../stores/appStore";
import * as api from "../lib/tauri";
import i18n from "../i18n";

const SYNC_INTERVAL_MS = 30_000;

/**
 * Stable action callbacks for extension operations.
 * Does NOT subscribe to the Zustand store — zero re-renders from state changes.
 */
export function useExtensionActions() {
  const refresh = useCallback(async () => {
    try {
      const exts = await api.extensionList();
      useAppStore.getState().setExtensions(exts);
    } catch (e) {
      useAppStore.getState().addNotification(
        i18n.t("error.loadExtensions", { error: e }),
        "error",
      );
    }
  }, []);

  const enable = useCallback(async (extId: string) => {
    await api.extensionEnable(extId);
  }, []);

  const disable = useCallback(async (extId: string) => {
    await api.extensionDisable(extId);
  }, []);

  const remove = useCallback(async (extId: string) => {
    await api.extensionRemove(extId);
  }, []);

  return { refresh, enable, disable, remove };
}

/**
 * 30-second fallback poll for extension status.
 * Call exactly ONCE in the app root — do not use in child components.
 */
export function useExtensionSync() {
  const syncingRef = useRef(false);
  useEffect(() => {
    const timer = setInterval(async () => {
      if (syncingRef.current) return;
      syncingRef.current = true;
      try {
        const exts = await api.extensionList();
        useAppStore.getState().setExtensions(exts);
      } catch {
        // Extension system may not be available
      } finally {
        syncingRef.current = false;
      }
    }, SYNC_INTERVAL_MS);
    return () => clearInterval(timer);
  }, []);
}
