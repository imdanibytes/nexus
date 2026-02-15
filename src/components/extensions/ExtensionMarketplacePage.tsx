import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { useExtensionMarketplace } from "../../hooks/useExtensionMarketplace";
import { useAppStore } from "../../stores/appStore";
import { extensionInstallLocal } from "../../lib/tauri";
import { ExtensionRegistryCard } from "./ExtensionCard";
import { SearchBar } from "../marketplace/SearchBar";
import { FolderOpen, RefreshCw, Blocks } from "lucide-react";
import { Button } from "@/components/ui/button";

export function ExtensionMarketplacePage() {
  const { extensions, isLoading, refresh, search } = useExtensionMarketplace();
  const { selectExtensionEntry, setView, addNotification, setInstallStatus } = useAppStore();
  const [installing, setInstalling] = useState(false);

  useEffect(() => {
    refresh();
  }, [refresh]);

  async function handleLocalInstall() {
    const manifestPath = await open({
      multiple: false,
      title: "Select extension manifest.json",
      filters: [{ name: "Extension Manifest", extensions: ["json"] }],
    });
    if (!manifestPath) return;

    setInstalling(true);
    setInstallStatus("Installing extension...");
    try {
      await extensionInstallLocal(manifestPath);
      addNotification("Extension installed from local manifest", "success");
      setView("settings");
    } catch (e) {
      addNotification(`Local install failed: ${e}`, "error");
    } finally {
      setInstalling(false);
      setInstallStatus(null);
    }
  }

  return (
    <div className="p-6">
      <div className="flex items-center justify-between mb-6">
        <div>
          <h2 className="text-[18px] font-bold text-nx-text">Add Host Extension</h2>
          <p className="text-[13px] text-nx-text-secondary">
            Browse extensions or install from local manifest + binary
          </p>
        </div>
        <div className="flex items-center gap-2">
          <Button
            size="sm"
            onClick={handleLocalInstall}
            disabled={installing}
          >
            <FolderOpen size={12} strokeWidth={1.5} />
            {installing ? "Installing..." : "Install Local"}
          </Button>
          <Button
            variant="secondary"
            size="sm"
            onClick={refresh}
            disabled={isLoading}
          >
            <RefreshCw size={12} strokeWidth={1.5} className={isLoading ? "animate-spin" : ""} />
            {isLoading ? "Refreshing..." : "Refresh"}
          </Button>
        </div>
      </div>

      <div className="mb-6">
        <SearchBar onSearch={search} />
      </div>

      {extensions.length === 0 ? (
        <div className="text-center py-16">
          <div className="w-16 h-16 rounded-[var(--radius-modal)] bg-nx-surface flex items-center justify-center mb-4 mx-auto">
            <Blocks size={28} strokeWidth={1.5} className="text-nx-text-ghost" />
          </div>
          <p className="text-nx-text-secondary text-[13px] mb-1">
            {isLoading
              ? "Loading extensions..."
              : "No marketplace extensions available yet."}
          </p>
          <p className="text-nx-text-muted text-[11px] mb-4">
            You can install an extension from a local{" "}
            <code className="bg-nx-deep text-nx-text-secondary px-1.5 py-0.5 rounded-[var(--radius-tag)] font-mono">
              manifest.json
            </code>{" "}
            + binary.
          </p>
          <Button
            onClick={handleLocalInstall}
            disabled={installing}
            className="mx-auto"
          >
            <FolderOpen size={14} strokeWidth={1.5} />
            {installing ? "Installing..." : "Install Local Extension"}
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
