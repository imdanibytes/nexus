import { useCallback } from "react";
import { useAppStore } from "../stores/appStore";
import { extensionMarketplaceSearch } from "../lib/tauri";
import i18n from "../i18n";

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
      addNotification(i18n.t("error.loadExtensionMarketplace", { error: e }), "error");
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
        addNotification(i18n.t("error.extensionSearchFailed", { error: e }), "error");
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
