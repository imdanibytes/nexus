import { useCallback } from "react";
import { useAppStore } from "../stores/appStore";
import { extensionMarketplaceSearch } from "../lib/tauri";
import i18n from "../i18n";

export function useExtensionMarketplace() {
  const extensionMarketplaceEntries = useAppStore((s) => s.extensionMarketplaceEntries);
  const isLoading = useAppStore((s) => s.isLoading);

  const refresh = useCallback(async () => {
    useAppStore.getState().setLoading(true);
    try {
      const results = await extensionMarketplaceSearch("");
      useAppStore.getState().setExtensionMarketplace(results);
    } catch (e) {
      useAppStore.getState().addNotification(i18n.t("error.loadExtensionMarketplace", { error: e }), "error");
    } finally {
      useAppStore.getState().setLoading(false);
    }
  }, []);

  const search = useCallback(
    async (query: string) => {
      try {
        const results = await extensionMarketplaceSearch(query);
        useAppStore.getState().setExtensionMarketplace(results);
      } catch (e) {
        useAppStore.getState().addNotification(i18n.t("error.extensionSearchFailed", { error: e }), "error");
      }
    },
    []
  );

  return {
    extensions: extensionMarketplaceEntries,
    isLoading,
    refresh,
    search,
  };
}
