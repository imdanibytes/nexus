import { useCallback, useEffect, useRef } from "react";
import { useAppStore } from "../stores/appStore";
import * as api from "../lib/tauri";

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
      addNotification(`Failed to load extensions: ${e}`, "error");
    }
  }, [setExtensions, addNotification]);

  const enable = useCallback(
    async (extId: string) => {
      setExtensionBusy(extId, "enabling");
      try {
        await api.extensionEnable(extId);
        addNotification("Extension enabled", "success");
        await refresh();
      } catch (e) {
        addNotification(`Enable failed: ${e}`, "error");
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
        addNotification("Extension disabled", "info");
        await refresh();
      } catch (e) {
        addNotification(`Disable failed: ${e}`, "error");
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
        addNotification("Extension removed", "info");
      } catch (e) {
        addNotification(`Remove failed: ${e}`, "error");
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
