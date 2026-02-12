import { useCallback } from "react";
import { useAppStore } from "../stores/appStore";
import * as api from "../lib/tauri";

export function useMarketplace() {
  const {
    marketplacePlugins,
    searchQuery,
    isLoading,
    setMarketplace,
    setSearchQuery,
    setLoading,
    addNotification,
  } = useAppStore();

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      await api.marketplaceRefresh();
      const results = await api.marketplaceSearch("");
      setMarketplace(results);
    } catch (e) {
      addNotification(`Failed to load marketplace: ${e}`, "error");
    } finally {
      setLoading(false);
    }
  }, [setMarketplace, setLoading, addNotification]);

  const search = useCallback(
    async (query: string) => {
      setSearchQuery(query);
      try {
        const results = await api.marketplaceSearch(query);
        setMarketplace(results);
      } catch (e) {
        addNotification(`Search failed: ${e}`, "error");
      }
    },
    [setMarketplace, setSearchQuery, addNotification]
  );

  return {
    plugins: marketplacePlugins,
    searchQuery,
    isLoading,
    refresh,
    search,
  };
}
