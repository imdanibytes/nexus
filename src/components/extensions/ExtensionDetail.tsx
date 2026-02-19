import { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import type { ExtensionRegistryEntry, ExtensionManifest, Capability } from "../../types/extension";
import { extensionPreview, extensionInstall } from "../../lib/tauri";
import { checkUrlReachable } from "../../lib/tauri";
import { useAppStore } from "../../stores/appStore";
import {
  ArrowLeft,
  Download,
  Loader2,
  Shield,
  Blocks,
  Terminal,
  FileText,
  FilePen,
  Globe,
  Cpu,
  Library,
  Puzzle,
  AlertTriangle,
  User,
  Clock,
} from "lucide-react";
import { timeAgo } from "../../lib/timeAgo";
import { Button, Card, CardBody, Chip } from "@heroui/react";

const RISK_CHIP_COLORS: Record<string, "success" | "warning" | "danger"> = {
  low: "success",
  medium: "warning",
  high: "danger",
};

function capabilityIcon(cap: Capability) {
  switch (cap.type) {
    case "process_exec": return Terminal;
    case "file_read": return FileText;
    case "file_write": return FilePen;
    case "network_http": return Globe;
    case "system_info": return Cpu;
    case "native_library": return Library;
    case "custom": return Puzzle;
  }
}

function capabilityLabel(cap: Capability, t: (key: string) => string): string {
  if (cap.type === "custom") return cap.name;
  return t(`plugins:capability.${cap.type}`);
}

function capabilityDetail(cap: Capability): string | null {
  switch (cap.type) {
    case "system_info": return null;
    case "custom": return cap.description;
    default:
      if ("scope" in cap && cap.scope.length > 0) {
        return cap.scope.join(", ");
      }
      return null;
  }
}

interface Props {
  entry: ExtensionRegistryEntry;
  onBack: () => void;
}

export function ExtensionDetail({ entry, onBack }: Props) {
  const { t } = useTranslation("plugins");
  const { addNotification } = useAppStore();
  const [loading, setLoading] = useState(false);
  const [installing, setInstalling] = useState(false);
  const [manifest, setManifest] = useState<ExtensionManifest | null>(null);
  const [manifestReachable, setManifestReachable] = useState<boolean | null>(null);

  useEffect(() => {
    let cancelled = false;
    checkUrlReachable(entry.manifest_url).then((reachable) => {
      if (!cancelled) setManifestReachable(reachable);
    });
    return () => { cancelled = true; };
  }, [entry.manifest_url]);

  async function handlePreview() {
    setLoading(true);
    try {
      const m = await extensionPreview(entry.manifest_url);
      setManifest(m);
    } catch (e) {
      addNotification(t("common:error.fetchExtensionManifest", { error: e }), "error");
    } finally {
      setLoading(false);
    }
  }

  async function handleInstall() {
    setInstalling(true);
    try {
      await extensionInstall(entry.manifest_url);
      onBack();
    } catch (e) {
      addNotification(t("common:error.installFailed", { error: e }), "error");
    } finally {
      setInstalling(false);
    }
  }

  return (
    <div className="p-6 max-w-2xl mx-auto">
      <Button
        onPress={onBack}
        className="mb-6"
      >
        <ArrowLeft size={14} strokeWidth={1.5} />
        {t("extensions.backToExtensions")}
      </Button>

      <Card><CardBody className="p-6">
        <div className="flex items-start justify-between mb-4">
          <div>
            <h2 className="text-[18px] font-bold">{entry.name}</h2>
            <p className="text-[12px] text-default-500 mt-1 font-mono">
              v{entry.version} &middot; {entry.id}
            </p>
          </div>
          {!manifest ? (
            manifestReachable === false ? (
              <span className="flex items-center gap-1.5 px-3 py-1.5 text-[11px] font-medium rounded-[8px] bg-danger-50 text-danger">
                <AlertTriangle size={12} strokeWidth={1.5} />
                {t("extensions.unavailable")}
              </span>
            ) : (
              <Button
                onPress={handlePreview}
                isDisabled={loading || manifestReachable === null}
              >
                {loading || manifestReachable === null ? (
                  <Loader2 size={14} strokeWidth={1.5} className="animate-spin" />
                ) : (
                  <Shield size={14} strokeWidth={1.5} />
                )}
                {manifestReachable === null ? t("common:action.checking") : loading ? t("common:action.loading") : t("extensions.reviewAndInstall")}
              </Button>
            )
          ) : (
            <Button
              onPress={handleInstall}
              isDisabled={installing}
            >
              {installing ? (
                <Loader2 size={14} strokeWidth={1.5} className="animate-spin" />
              ) : (
                <Download size={14} strokeWidth={1.5} />
              )}
              {installing ? t("common:action.installing") : t("extensions.installExtension")}
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
              {t("extensions.published", { time: timeAgo(entry.created_at) })}
            </span>
          )}
        </div>

        <p className="text-default-500 text-[13px] mb-6 leading-relaxed">
          {entry.description}
        </p>

        {entry.categories.length > 0 && (
          <div className="mb-6">
            <h4 className="text-[10px] font-semibold text-default-500 uppercase tracking-wider mb-2">
              {t("marketplace.categories")}
            </h4>
            <div className="flex gap-2">
              {entry.categories.map((cat) => (
                <Chip key={cat} size="sm" variant="flat">
                  {cat}
                </Chip>
              ))}
            </div>
          </div>
        )}
      </CardBody></Card>

      {/* Manifest preview -- shown after clicking "Review & Install" */}
      {manifest && (
        <div className="mt-4 space-y-4">
          {/* Author + signature */}
          <Card><CardBody className="p-5">
            <div className="flex items-center gap-2 mb-3">
              <Shield size={13} strokeWidth={1.5} className="text-default-500" />
              <h4 className="text-[12px] font-semibold">
                {t("extensions.authorAndSignature")}
              </h4>
            </div>
            <div className="space-y-2">
              <div className="flex items-center gap-2">
                <span className="text-[11px] text-default-500 w-20">{t("about.author")}</span>
                <span className="text-[11px] font-medium">
                  {manifest.author}
                </span>
              </div>
              <div className="flex items-center gap-2">
                <span className="text-[11px] text-default-500 w-20">{t("about.license")}</span>
                <span className="text-[11px] font-medium">
                  {manifest.license ?? t("common:status.notSpecified")}
                </span>
              </div>
              <div className="flex items-start gap-2">
                <span className="text-[11px] text-default-500 w-20 flex-shrink-0">{t("extensions.publicKey")}</span>
                <code className="text-[10px] text-default-500 bg-background px-2 py-1 rounded-[6px] font-mono break-all">
                  {manifest.author_public_key.slice(0, 32)}...
                </code>
              </div>
              <div className="flex items-center gap-2">
                <span className="text-[11px] text-default-500 w-20">{t("extensions.platforms")}</span>
                <div className="flex gap-1.5">
                  {Object.keys(manifest.binaries).map((platform) => (
                    <Chip key={platform} size="sm" variant="flat">
                      {platform}
                    </Chip>
                  ))}
                </div>
              </div>
            </div>
          </CardBody></Card>

          {/* Capabilities */}
          {manifest.capabilities.length > 0 && (
            <Card><CardBody className="p-5">
              <div className="flex items-center gap-2 mb-3">
                <Shield size={13} strokeWidth={1.5} className="text-warning" />
                <h4 className="text-[12px] font-semibold">
                  {t("extensions.declaredCapabilities")}
                </h4>
              </div>
              <p className="text-[10px] text-default-400 mb-3">
                {t("extensions.capabilitiesWarning")}
              </p>
              <div className="space-y-1">
                {manifest.capabilities.map((cap, i) => {
                  const Icon = capabilityIcon(cap);
                  const detail = capabilityDetail(cap);
                  return (
                    <div
                      key={i}
                      className="flex items-center gap-3 px-3 py-2 rounded-[8px] bg-background border border-default-100"
                    >
                      <Icon size={13} strokeWidth={1.5} className="text-default-500 flex-shrink-0" />
                      <span className="text-[11px] font-medium flex-shrink-0">
                        {capabilityLabel(cap, t)}
                      </span>
                      {detail && (
                        <span className="text-[10px] text-default-400 truncate font-mono">
                          {detail}
                        </span>
                      )}
                    </div>
                  );
                })}
              </div>
          </CardBody></Card>
          )}

          {/* Operations */}
          <Card><CardBody className="p-5">
            <div className="flex items-center gap-2 mb-3">
              <Blocks size={13} strokeWidth={1.5} className="text-default-500" />
              <h4 className="text-[12px] font-semibold">
                {t("extensions.operations", { count: manifest.operations.length })}
              </h4>
            </div>
            <div className="space-y-1">
              {manifest.operations.map((op) => {
                return (
                  <div
                    key={op.name}
                    className="flex items-center gap-3 px-3 py-2 rounded-[8px] bg-background border border-default-100"
                  >
                    <span className="text-[12px] font-mono flex-shrink-0">
                      {op.name}
                    </span>
                    <Chip size="sm" variant="flat" color={RISK_CHIP_COLORS[op.risk_level] ?? "warning"}>
                      {op.risk_level}
                    </Chip>
                    {op.scope_key && (
                      <Chip size="sm">
                        {t("extensions.scope", { key: op.scope_key })}
                      </Chip>
                    )}
                    <span className="text-[11px] text-default-400 truncate flex-1">
                      {op.description}
                    </span>
                  </div>
                );
              })}
            </div>
          </CardBody></Card>
        </div>
      )}
    </div>
  );
}
