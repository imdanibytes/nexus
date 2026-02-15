import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { open } from "@tauri-apps/plugin-dialog";
import { useMarketplace } from "../../hooks/useMarketplace";
import { usePlugins } from "../../hooks/usePlugins";
import { useAppStore } from "../../stores/appStore";
import { RegistryPluginCard } from "../plugins/PluginCard";
import { SearchBar } from "./SearchBar";
import { PermissionDialog } from "../permissions/PermissionDialog";
import type { PluginManifest } from "../../types/plugin";
import type { Permission } from "../../types/permissions";
import { FolderOpen, RefreshCw, Package, Wand2 } from "lucide-react";
import { McpWrapWizard } from "./McpWrapWizard";
import { Button } from "@/components/ui/button";

export function MarketplacePage() {
  const { t } = useTranslation("plugins");
  const { plugins, isLoading, refresh, search } = useMarketplace();
  const { previewLocal, installLocal } = usePlugins();
  const { installedPlugins, selectRegistryEntry, setView } = useAppStore();
  const [installing, setInstalling] = useState(false);
  const [showMcpWizard, setShowMcpWizard] = useState(false);

  // Two-step local install state
  const [pendingManifest, setPendingManifest] = useState<PluginManifest | null>(null);
  const [pendingPath, setPendingPath] = useState<string | null>(null);

  const installedIds = new Set(installedPlugins.map((p) => p.manifest.id));

  useEffect(() => {
    refresh();
  }, [refresh]);

  async function handleLocalInstall() {
    const selected = await open({
      multiple: false,
      filters: [{ name: "Plugin Manifest", extensions: ["json"] }],
    });
    if (!selected) return;

    // Step 1: Preview the manifest
    const manifest = await previewLocal(selected);
    if (!manifest) return;

    // Show the install dialog
    setPendingPath(selected);
    setPendingManifest(manifest);
  }

  async function handleApprove(approvedPermissions: Permission[], deferredPermissions: Permission[]) {
    if (!pendingPath) return;

    setPendingManifest(null);
    setInstalling(true);
    await installLocal(pendingPath, approvedPermissions, deferredPermissions);
    setPendingPath(null);
    setInstalling(false);
    setView("plugins");
  }

  function handleDeny() {
    setPendingManifest(null);
    setPendingPath(null);
  }

  return (
    <div className="p-6">
      <div className="flex items-center justify-between mb-6">
        <div>
          <h2 className="text-[18px] font-bold text-nx-text">{t("marketplace.title")}</h2>
          <p className="text-[13px] text-nx-text-secondary">
            {t("marketplace.subtitle")}
          </p>
        </div>
        <div className="flex items-center gap-2">
          <Button
            size="sm"
            onClick={() => setShowMcpWizard(true)}
          >
            <Wand2 size={12} strokeWidth={1.5} />
            {t("marketplace.wrapMcp")}
          </Button>
          <Button
            variant="secondary"
            size="sm"
            onClick={handleLocalInstall}
            disabled={installing}
          >
            <FolderOpen size={12} strokeWidth={1.5} />
            {installing ? t("common:action.installing") : t("marketplace.installLocal")}
          </Button>
          <Button
            variant="secondary"
            size="sm"
            onClick={refresh}
            disabled={isLoading}
          >
            <RefreshCw size={12} strokeWidth={1.5} className={isLoading ? "animate-spin" : ""} />
            {isLoading ? t("common:action.refreshing") : t("common:action.refresh")}
          </Button>
        </div>
      </div>

      <div className="mb-6">
        <SearchBar onSearch={search} />
      </div>

      {plugins.length === 0 ? (
        <div className="text-center py-16">
          <div className="w-16 h-16 rounded-[var(--radius-modal)] bg-nx-surface flex items-center justify-center mb-4 mx-auto">
            <Package size={28} strokeWidth={1.5} className="text-nx-text-ghost" />
          </div>
          <p className="text-nx-text-secondary text-[13px] mb-1">
            {isLoading
              ? t("marketplace.loadingPlugins")
              : t("marketplace.noPluginsAvailable")}
          </p>
          <p className="text-nx-text-muted text-[11px] mb-4">
            {t("marketplace.localManifestHint", {
              interpolation: { escapeValue: false },
            })}
          </p>
          <Button
            onClick={handleLocalInstall}
            disabled={installing}
            className="mx-auto"
          >
            <FolderOpen size={14} strokeWidth={1.5} />
            {installing ? t("common:action.installing") : t("marketplace.installLocalPlugin")}
          </Button>
        </div>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          {plugins.map((entry) => (
            <RegistryPluginCard
              key={entry.id}
              entry={entry}
              isInstalled={installedIds.has(entry.id)}
              onSelect={() => {
                selectRegistryEntry(entry);
                setView("plugin-detail");
              }}
            />
          ))}
        </div>
      )}

      {pendingManifest && (
        <PermissionDialog
          manifest={pendingManifest}
          onApprove={handleApprove}
          onDeny={handleDeny}
        />
      )}

      {showMcpWizard && (
        <McpWrapWizard
          onClose={() => setShowMcpWizard(false)}
          onInstalled={() => setView("plugins")}
        />
      )}
    </div>
  );
}
