import { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import type { RegistryEntry, PluginManifest, InstalledPlugin } from "../../types/plugin";
import type { Permission } from "../../types/permissions";
import { PermissionDialog } from "../permissions/PermissionDialog";
import { usePluginActions } from "../../hooks/usePlugins";
import { checkImageAvailable } from "../../lib/tauri";
import { ArrowLeft, Download, Loader2, AlertTriangle, ExternalLink, User, Clock, Scale, Hammer, RefreshCw, HardDrive, Cloud } from "lucide-react";
import { timeAgo } from "../../lib/timeAgo";
import { Button, Card, CardBody, Chip } from "@heroui/react";

interface Props {
  entry: RegistryEntry;
  installedPlugin: InstalledPlugin | null;
  onBack: () => void;
}

export function PluginDetail({ entry, installedPlugin, onBack }: Props) {
  const { t } = useTranslation("plugins");
  const { previewRemote, install } = usePluginActions();
  const [loading, setLoading] = useState(false);
  const [pendingManifest, setPendingManifest] = useState<PluginManifest | null>(null);
  const [imageAvailable, setImageAvailable] = useState<boolean | null>(null);

  const isInstalled = !!installedPlugin;
  const isLocalSource = installedPlugin?.local_manifest_path != null;
  const canBuild = !!entry.build_context;

  useEffect(() => {
    if (canBuild) {
      queueMicrotask(() => setImageAvailable(true));
      return;
    }
    let cancelled = false;
    checkImageAvailable(entry.image).then((available) => {
      if (!cancelled) setImageAvailable(available);
    });
    return () => { cancelled = true; };
  }, [entry.image, canBuild]);

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
    await install(entry.manifest_url, approvedPermissions, deferredPermissions, entry.build_context);
    onBack();
  }

  return (
    <div className="p-6 max-w-2xl mx-auto">
      <Button
        onPress={onBack}
        className="mb-6"
      >
        <ArrowLeft size={14} strokeWidth={1.5} />
        {t("marketplace.backToMarketplace")}
      </Button>

      <Card><CardBody className="p-6">
        <div className="flex items-start justify-between mb-4">
          <div>
            <h2 className="text-[18px] font-bold">{entry.name}</h2>
            <p className="text-[12px] text-default-500 mt-1 font-mono">
              v{entry.version} &middot; {entry.id}
            </p>
          </div>
          {isInstalled ? (
            <div className="flex items-center gap-2">
              <Chip
                size="sm"
                variant="flat"
                color={isLocalSource ? "warning" : "primary"}
                startContent={isLocalSource ? <HardDrive size={10} strokeWidth={1.5} /> : <Cloud size={10} strokeWidth={1.5} />}
              >
                {isLocalSource ? t("marketplace.localDev") : t("common:status.registry")}
              </Chip>
              {imageAvailable === false ? (
                <span className="flex items-center gap-1.5 px-3 py-1.5 text-[11px] font-medium rounded-[8px] bg-danger-50 text-danger">
                  <AlertTriangle size={12} strokeWidth={1.5} />
                  {t("marketplace.imageUnavailable")}
                </span>
              ) : (
                <Button
                  onPress={handleInstallClick}
                  isDisabled={loading || imageAvailable === null}
                >
                  {loading || imageAvailable === null ? (
                    <Loader2 size={14} strokeWidth={1.5} className="animate-spin" />
                  ) : (
                    <RefreshCw size={14} strokeWidth={1.5} />
                  )}
                  {imageAvailable === null ? t("common:action.checking") : loading ? t("common:action.loading") : t("marketplace.reinstall")}
                </Button>
              )}
            </div>
          ) : imageAvailable === false ? (
            <span className="flex items-center gap-1.5 px-3 py-1.5 text-[11px] font-medium rounded-[8px] bg-danger-50 text-danger">
              <AlertTriangle size={12} strokeWidth={1.5} />
              {t("marketplace.imageUnavailable")}
            </span>
          ) : (
            <Button
              onPress={handleInstallClick}
              isDisabled={loading || imageAvailable === null}
            >
              {loading || imageAvailable === null ? (
                <Loader2 size={14} strokeWidth={1.5} className="animate-spin" />
              ) : canBuild ? (
                <Hammer size={14} strokeWidth={1.5} />
              ) : (
                <Download size={14} strokeWidth={1.5} />
              )}
              {imageAvailable === null ? t("common:action.checking") : loading ? t("marketplace.building") : canBuild ? t("marketplace.buildAndInstall") : t("common:action.install")}
            </Button>
          )}
        </div>

        {/* Metadata row */}
        <div className="flex flex-wrap items-center gap-x-4 gap-y-1.5 mb-4 text-[11px] text-default-500">
          {entry.author && (
            <span className="flex items-center gap-1">
              <User size={11} strokeWidth={1.5} />
              {entry.author_url ? (
                <a
                  href={entry.author_url}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="transition-colors duration-150"
                >
                  {entry.author}
                </a>
              ) : (
                entry.author
              )}
            </span>
          )}
          {entry.created_at && (
            <span className="flex items-center gap-1" title={entry.created_at}>
              <Clock size={11} strokeWidth={1.5} />
              {t("marketplace.published", { time: timeAgo(entry.created_at) })}
            </span>
          )}
          {entry.license && (
            <span className="flex items-center gap-1">
              <Scale size={11} strokeWidth={1.5} />
              {entry.license}
            </span>
          )}
          {entry.homepage && (
            <a
              href={entry.homepage}
              target="_blank"
              rel="noopener noreferrer"
              className="flex items-center gap-1 transition-colors duration-150"
            >
              <ExternalLink size={11} strokeWidth={1.5} />
              {t("marketplace.repository")}
            </a>
          )}
        </div>

        <p className="text-default-500 text-[13px] mb-6 leading-relaxed">{entry.description}</p>

        <div className="space-y-4">
          <div>
            <h4 className="text-[10px] font-semibold text-default-500 uppercase tracking-wider mb-2">
              {t("marketplace.containerImage")}
            </h4>
            <code className="text-[12px] bg-background text-default-500 px-2.5 py-1 rounded-[6px] font-mono">
              {entry.image}
            </code>
            {canBuild && (
              <span className="ml-2 text-[10px] text-default-500">
                {t("marketplace.builtFromSource")}
              </span>
            )}
          </div>

          {entry.image_digest && (
            <div>
              <h4 className="text-[10px] font-semibold text-default-500 uppercase tracking-wider mb-2">
                {t("marketplace.imageDigest")}
              </h4>
              <code className="text-[12px] bg-background text-default-500 px-2.5 py-1 rounded-[6px] font-mono break-all">
                {entry.image_digest}
              </code>
            </div>
          )}

          {entry.categories.length > 0 && (
            <div>
              <h4 className="text-[10px] font-semibold text-default-500 uppercase tracking-wider mb-2">
                {t("marketplace.categories")}
              </h4>
              <div className="flex gap-2 flex-wrap">
                {entry.categories.map((cat) => (
                  <Chip key={cat} size="sm" variant="flat">
                    {cat}
                  </Chip>
                ))}
              </div>
            </div>
          )}
        </div>
      </CardBody></Card>

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
