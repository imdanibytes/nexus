import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { useMarketplace } from "../../hooks/useMarketplace";
import { usePlugins } from "../../hooks/usePlugins";
import { useAppStore } from "../../stores/appStore";
import { RegistryPluginCard } from "../plugins/PluginCard";
import { SearchBar } from "./SearchBar";
import { PermissionDialog } from "../permissions/PermissionDialog";
import type { PluginManifest } from "../../types/plugin";
import type { Permission } from "../../types/permissions";
import { FolderOpen, RefreshCw, Package } from "lucide-react";

export function MarketplacePage() {
  const { plugins, isLoading, refresh, search } = useMarketplace();
  const { previewLocal, installLocal } = usePlugins();
  const { installedPlugins, selectRegistryEntry, setView } = useAppStore();
  const [installing, setInstalling] = useState(false);

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
          <h2 className="text-[18px] font-bold text-nx-text">Add Plugins</h2>
          <p className="text-[13px] text-nx-text-secondary">
            Browse the marketplace or install from a local manifest
          </p>
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={handleLocalInstall}
            disabled={installing}
            className="flex items-center gap-1.5 px-3 py-1.5 text-[11px] font-medium rounded-[var(--radius-button)] bg-nx-accent hover:bg-nx-accent-hover disabled:opacity-40 text-nx-deep transition-all duration-150"
          >
            <FolderOpen size={12} strokeWidth={1.5} />
            {installing ? "Installing..." : "Install Local"}
          </button>
          <button
            onClick={refresh}
            disabled={isLoading}
            className="flex items-center gap-1.5 px-3 py-1.5 text-[11px] font-medium rounded-[var(--radius-button)] bg-nx-overlay hover:bg-nx-wash text-nx-text-secondary transition-all duration-150 disabled:opacity-50"
          >
            <RefreshCw size={12} strokeWidth={1.5} className={isLoading ? "animate-spin" : ""} />
            {isLoading ? "Refreshing..." : "Refresh"}
          </button>
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
              ? "Loading plugins..."
              : "No marketplace plugins available yet."}
          </p>
          <p className="text-nx-text-muted text-[11px] mb-4">
            You can install a plugin from a local <code className="bg-nx-deep text-nx-text-secondary px-1.5 py-0.5 rounded-[var(--radius-tag)] font-mono">plugin.json</code> manifest.
          </p>
          <button
            onClick={handleLocalInstall}
            disabled={installing}
            className="flex items-center gap-2 mx-auto px-4 py-2 bg-nx-accent hover:bg-nx-accent-hover disabled:opacity-40 text-nx-deep text-[13px] font-medium rounded-[var(--radius-button)] transition-all duration-150"
          >
            <FolderOpen size={14} strokeWidth={1.5} />
            {installing ? "Installing..." : "Install Local Plugin"}
          </button>
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
    </div>
  );
}
