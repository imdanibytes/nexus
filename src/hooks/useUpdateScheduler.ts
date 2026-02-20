import { useCallback, useEffect } from "react";
import { useAppStore } from "../stores/appStore";
import { marketplaceRefresh, marketplaceLoad, checkUpdates, getUpdateCheckInterval } from "../lib/tauri";

/**
 * Central hook that owns the full update-check lifecycle:
 * - Loads the auto-check interval setting on mount
 * - Loads the registry from disk cache on mount (instant, no network)
 * - Runs a background refresh + update check on mount
 * - Owns the setInterval timer (restarts when interval changes)
 * - Exposes checkNow() for manual triggers (UpdatesTab button)
 */
export function useUpdateScheduler() {
  const updateCheckInterval = useAppStore((s) => s.updateCheckInterval);

  const refreshAndCheck = useCallback(async () => {
    try {
      await marketplaceRefresh();
      const updates = await checkUpdates();
      if (updates.length > 0) {
        const { setAvailableUpdates, dismissByCategory, notify } = useAppStore.getState();
        setAvailableUpdates(updates);
        dismissByCategory("updates.plugins");
        dismissByCategory("updates.extensions");
        for (const u of updates) {
          const cat = u.item_type === "plugin" ? "updates.plugins" : "updates.extensions";
          notify(cat, u.item_name, { data: u });
        }
      }
    } catch {
      // Silently ignore — offline or registry unreachable
    }
  }, []);

  // On mount: load interval setting, load disk cache, then background refresh
  useEffect(() => {
    getUpdateCheckInterval()
      .then((interval) => useAppStore.getState().setUpdateCheckInterval(interval))
      .catch(() => {});

    marketplaceLoad().catch(() => {});

    refreshAndCheck().catch(() => {});
  }, [refreshAndCheck]);

  // Timer lifecycle — restarts whenever the interval setting changes
  useEffect(() => {
    if (updateCheckInterval <= 0) return;
    const id = setInterval(refreshAndCheck, updateCheckInterval * 60 * 1000);
    return () => clearInterval(id);
  }, [updateCheckInterval, refreshAndCheck]);

  return { checkNow: refreshAndCheck };
}
