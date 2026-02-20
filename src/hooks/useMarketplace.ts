import { useCallback } from "react";
import { useAppStore } from "../stores/appStore";
import * as api from "../lib/tauri";
import i18n from "../i18n";

export function useMarketplace() {
  const marketplacePlugins = useAppStore((s) => s.marketplacePlugins);
  const searchQuery = useAppStore((s) => s.searchQuery);
  const isLoading = useAppStore((s) => s.isLoading);

  const refresh = useCallback(async () => {
    useAppStore.getState().setLoading(true);
    try {
      // Load from disk cache first (instant, no network).
      // The update scheduler handles network refreshes in the background.
      await api.marketplaceLoad();
      const results = await api.marketplaceSearch("");
      useAppStore.getState().setMarketplace(results);
    } catch {
      // No cache yet — fall back to network fetch
      try {
        await api.marketplaceRefresh();
        const results = await api.marketplaceSearch("");
        useAppStore.getState().setMarketplace(results);
      } catch (e) {
        useAppStore.getState().addNotification(i18n.t("error.loadMarketplace", { error: e }), "error");
      }
    } finally {
      useAppStore.getState().setLoading(false);
    }
  }, []);

  const search = useCallback(
    async (query: string) => {
      useAppStore.getState().setSearchQuery(query);
      try {
        const results = await api.marketplaceSearch(query);
        useAppStore.getState().setMarketplace(results);
      } catch (e) {
        useAppStore.getState().addNotification(i18n.t("error.searchFailed", { error: e }), "error");
      }
    },
    []
  );

  return {
    plugins: marketplacePlugins,
    searchQuery,
    isLoading,
    refresh,
    search,
  };
}
