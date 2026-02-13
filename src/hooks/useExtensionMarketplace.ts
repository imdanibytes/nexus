import { useCallback } from "react";
import { useAppStore } from "../stores/appStore";
import { extensionMarketplaceSearch } from "../lib/extensions";

export function useExtensionMarketplace() {
  const {
    extensionMarketplaceEntries,
    isLoading,
    setExtensionMarketplace,
    setLoading,
    addNotification,
  } = useAppStore();

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const results = await extensionMarketplaceSearch("");
      setExtensionMarketplace(results);
    } catch (e) {
      addNotification(`Failed to load extension marketplace: ${e}`, "error");
    } finally {
      setLoading(false);
    }
  }, [setExtensionMarketplace, setLoading, addNotification]);

  const search = useCallback(
    async (query: string) => {
      try {
        const results = await extensionMarketplaceSearch(query);
        setExtensionMarketplace(results);
      } catch (e) {
        addNotification(`Extension search failed: ${e}`, "error");
      }
    },
    [setExtensionMarketplace, addNotification]
  );

  return {
    extensions: extensionMarketplaceEntries,
    isLoading,
    refresh,
    search,
  };
}
