import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { open } from "@tauri-apps/plugin-dialog";
import { useExtensionMarketplace } from "../../hooks/useExtensionMarketplace";
import { useAppStore } from "../../stores/appStore";
import { extensionInstallLocal } from "../../lib/tauri";
import { ExtensionRegistryCard } from "./ExtensionCard";
import { SearchBar } from "../marketplace/SearchBar";
import { FolderOpen, RefreshCw, Blocks } from "lucide-react";
import { Button } from "@heroui/react";

export function ExtensionMarketplacePage() {
  const { t } = useTranslation("plugins");
  const { extensions, isLoading, refresh, search } = useExtensionMarketplace();
  const { selectExtensionEntry, setView, addNotification, setInstallStatus } = useAppStore();
  const [installing, setInstalling] = useState(false);

  useEffect(() => {
    refresh();
  }, [refresh]);

  async function handleLocalInstall() {
    const manifestPath = await open({
      multiple: false,
      title: t("extensions.selectManifest"),
      filters: [{ name: "Extension Manifest", extensions: ["json"] }],
    });
    if (!manifestPath) return;

    setInstalling(true);
    setInstallStatus(t("extensions.installingExtension"));
    try {
      await extensionInstallLocal(manifestPath);
      addNotification(t("common:notification.extensionInstalledLocal"), "success");
      setView("settings");
    } catch (e) {
      addNotification(t("common:error.localInstallFailed", { error: e }), "error");
    } finally {
      setInstalling(false);
      setInstallStatus(null);
    }
  }

  return (
    <div className="p-6">
      <div className="flex items-center justify-between mb-6">
        <div>
          <h2 className="text-[18px] font-bold">{t("extensions.title")}</h2>
          <p className="text-[13px] text-default-500">
            {t("extensions.subtitle")}
          </p>
        </div>
        <div className="flex items-center gap-2">
          <Button
            onPress={handleLocalInstall}
            isDisabled={installing}
          >
            <FolderOpen size={12} strokeWidth={1.5} />
            {installing ? t("common:action.installing") : t("marketplace.installLocal")}
          </Button>
          <Button
            onPress={refresh}
            isDisabled={isLoading}
          >
            <RefreshCw size={12} strokeWidth={1.5} className={isLoading ? "animate-spin" : ""} />
            {isLoading ? t("common:action.refreshing") : t("common:action.refresh")}
          </Button>
        </div>
      </div>

      <div className="mb-6">
        <SearchBar onSearch={search} />
      </div>

      {extensions.length === 0 ? (
        <div className="text-center py-16">
          <div className="w-16 h-16 rounded-[14px] bg-default-100 flex items-center justify-center mb-4 mx-auto">
            <Blocks size={28} strokeWidth={1.5} className="text-default-400" />
          </div>
          <p className="text-default-500 text-[13px] mb-1">
            {isLoading
              ? t("extensions.loadingExtensions")
              : t("extensions.noExtensions")}
          </p>
          <p
            className="text-default-500 text-[11px] mb-4"
            dangerouslySetInnerHTML={{ __html: t("extensions.localManifestHint") }}
          />
          <Button
            onPress={handleLocalInstall}
            isDisabled={installing}
            className="mx-auto"
          >
            <FolderOpen size={14} strokeWidth={1.5} />
            {installing ? t("common:action.installing") : t("extensions.installLocalExtension")}
          </Button>
        </div>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          {extensions.map((entry) => (
            <ExtensionRegistryCard
              key={entry.id}
              entry={entry}
              onSelect={() => {
                selectExtensionEntry(entry);
                setView("extension-detail");
              }}
            />
          ))}
        </div>
      )}
    </div>
  );
}
