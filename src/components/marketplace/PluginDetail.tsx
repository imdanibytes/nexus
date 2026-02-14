import { useState, useEffect } from "react";
import type { RegistryEntry, PluginManifest } from "../../types/plugin";
import type { Permission } from "../../types/permissions";
import { PermissionDialog } from "../permissions/PermissionDialog";
import { usePlugins } from "../../hooks/usePlugins";
import { checkImageAvailable } from "../../lib/tauri";
import { ArrowLeft, Download, Check, Loader2, AlertTriangle } from "lucide-react";

interface Props {
  entry: RegistryEntry;
  isInstalled: boolean;
  onBack: () => void;
}

export function PluginDetail({ entry, isInstalled, onBack }: Props) {
  const { previewRemote, install } = usePlugins();
  const [loading, setLoading] = useState(false);
  const [pendingManifest, setPendingManifest] = useState<PluginManifest | null>(null);
  const [imageAvailable, setImageAvailable] = useState<boolean | null>(null);

  useEffect(() => {
    if (isInstalled) return;
    let cancelled = false;
    checkImageAvailable(entry.image).then((available) => {
      if (!cancelled) setImageAvailable(available);
    });
    return () => { cancelled = true; };
  }, [entry.image, isInstalled]);

  async function handleInstallClick() {
    setLoading(true);
    const manifest = await previewRemote(entry.manifest_url);
    setLoading(false);
    if (manifest) {
      setPendingManifest(manifest);
    }
  }

  async function handleApprove(approvedPermissions: Permission[], deferredPermissions: Permission[]) {
    setPendingManifest(null);
    await install(entry.manifest_url, approvedPermissions, deferredPermissions);
    onBack();
  }

  return (
    <div className="p-6 max-w-2xl mx-auto">
      <button
        onClick={onBack}
        className="flex items-center gap-1.5 text-[12px] font-medium text-nx-text-muted hover:text-nx-text mb-6 transition-colors duration-150"
      >
        <ArrowLeft size={14} strokeWidth={1.5} />
        Back to Marketplace
      </button>

      <div className="bg-nx-surface rounded-[var(--radius-card)] border border-nx-border p-6">
        <div className="flex items-start justify-between mb-4">
          <div>
            <h2 className="text-[18px] font-bold text-nx-text">{entry.name}</h2>
            <p className="text-[12px] text-nx-text-muted mt-1 font-mono">
              v{entry.version} &middot; {entry.id}
            </p>
          </div>
          {isInstalled ? (
            <span className="flex items-center gap-1.5 px-3 py-1.5 text-[11px] font-medium rounded-[var(--radius-button)] bg-nx-accent-muted text-nx-accent">
              <Check size={12} strokeWidth={1.5} />
              Installed
            </span>
          ) : imageAvailable === false ? (
            <span className="flex items-center gap-1.5 px-3 py-1.5 text-[11px] font-medium rounded-[var(--radius-button)] bg-nx-error-muted text-nx-error">
              <AlertTriangle size={12} strokeWidth={1.5} />
              Image Unavailable
            </span>
          ) : (
            <button
              onClick={handleInstallClick}
              disabled={loading || imageAvailable === null}
              className="flex items-center gap-1.5 px-4 py-2 bg-nx-accent hover:bg-nx-accent-hover disabled:opacity-60 text-nx-deep text-[13px] font-medium rounded-[var(--radius-button)] transition-all duration-150"
            >
              {loading || imageAvailable === null ? (
                <Loader2 size={14} strokeWidth={1.5} className="animate-spin" />
              ) : (
                <Download size={14} strokeWidth={1.5} />
              )}
              {imageAvailable === null ? "Checking..." : loading ? "Loading..." : "Install"}
            </button>
          )}
        </div>

        <p className="text-nx-text-secondary text-[13px] mb-6 leading-relaxed">{entry.description}</p>

        <div className="space-y-4">
          <div>
            <h4 className="text-[10px] font-semibold text-nx-text-muted uppercase tracking-wider mb-2">
              Docker Image
            </h4>
            <code className="text-[12px] bg-nx-deep text-nx-text-secondary px-2.5 py-1 rounded-[var(--radius-tag)] font-mono">
              {entry.image}
            </code>
          </div>

          {entry.image_digest && (
            <div>
              <h4 className="text-[10px] font-semibold text-nx-text-muted uppercase tracking-wider mb-2">
                Image Digest
              </h4>
              <code className="text-[12px] bg-nx-deep text-nx-text-secondary px-2.5 py-1 rounded-[var(--radius-tag)] font-mono break-all">
                {entry.image_digest}
              </code>
            </div>
          )}

          {entry.categories.length > 0 && (
            <div>
              <h4 className="text-[10px] font-semibold text-nx-text-muted uppercase tracking-wider mb-2">
                Categories
              </h4>
              <div className="flex gap-2">
                {entry.categories.map((cat) => (
                  <span
                    key={cat}
                    className="text-[11px] px-2 py-1 rounded-[var(--radius-tag)] bg-nx-overlay text-nx-text-secondary"
                  >
                    {cat}
                  </span>
                ))}
              </div>
            </div>
          )}
        </div>
      </div>

      {pendingManifest && (
        <PermissionDialog
          manifest={pendingManifest}
          onApprove={handleApprove}
          onDeny={() => setPendingManifest(null)}
        />
      )}
    </div>
  );
}
