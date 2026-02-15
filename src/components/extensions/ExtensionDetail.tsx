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
import { Button } from "@/components/ui/button";

const RISK_STYLES: Record<string, { bg: string; text: string }> = {
  low: { bg: "bg-nx-success-muted", text: "text-nx-success" },
  medium: { bg: "bg-nx-warning-muted", text: "text-nx-warning" },
  high: { bg: "bg-nx-error-muted", text: "text-nx-error" },
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
  const { addNotification, setInstallStatus } = useAppStore();
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
    setInstallStatus(t("extensions.installingExtension"));
    try {
      await extensionInstall(entry.manifest_url);
      addNotification(t("common:notification.extensionInstalled", { name: entry.name }), "success");
      onBack();
    } catch (e) {
      addNotification(t("common:error.installFailed", { error: e }), "error");
    } finally {
      setInstalling(false);
      setInstallStatus(null);
    }
  }

  return (
    <div className="p-6 max-w-2xl mx-auto">
      <Button
        variant="ghost"
        size="sm"
        onClick={onBack}
        className="text-nx-text-muted hover:text-nx-text mb-6"
      >
        <ArrowLeft size={14} strokeWidth={1.5} />
        {t("extensions.backToExtensions")}
      </Button>

      <div className="bg-nx-surface rounded-[var(--radius-card)] border border-nx-border p-6">
        <div className="flex items-start justify-between mb-4">
          <div>
            <h2 className="text-[18px] font-bold text-nx-text">{entry.name}</h2>
            <p className="text-[12px] text-nx-text-muted mt-1 font-mono">
              v{entry.version} &middot; {entry.id}
            </p>
          </div>
          {!manifest ? (
            manifestReachable === false ? (
              <span className="flex items-center gap-1.5 px-3 py-1.5 text-[11px] font-medium rounded-[var(--radius-button)] bg-nx-error-muted text-nx-error">
                <AlertTriangle size={12} strokeWidth={1.5} />
                {t("extensions.unavailable")}
              </span>
            ) : (
              <Button
                onClick={handlePreview}
                disabled={loading || manifestReachable === null}
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
              onClick={handleInstall}
              disabled={installing}
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
        <div className="flex flex-wrap items-center gap-x-4 gap-y-1.5 mb-4 text-[11px] text-nx-text-muted">
          {entry.author && (
            <span className="flex items-center gap-1">
              <User size={11} strokeWidth={1.5} />
              {entry.author_url ? (
                <a
                  href={entry.author_url}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="hover:text-nx-text transition-colors duration-150"
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

        <p className="text-nx-text-secondary text-[13px] mb-6 leading-relaxed">
          {entry.description}
        </p>

        {entry.categories.length > 0 && (
          <div className="mb-6">
            <h4 className="text-[10px] font-semibold text-nx-text-muted uppercase tracking-wider mb-2">
              {t("marketplace.categories")}
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

      {/* Manifest preview -- shown after clicking "Review & Install" */}
      {manifest && (
        <div className="mt-4 space-y-4">
          {/* Author + signature */}
          <div className="bg-nx-surface rounded-[var(--radius-card)] border border-nx-border p-5">
            <div className="flex items-center gap-2 mb-3">
              <Shield size={13} strokeWidth={1.5} className="text-nx-text-muted" />
              <h4 className="text-[12px] font-semibold text-nx-text">
                {t("extensions.authorAndSignature")}
              </h4>
            </div>
            <div className="space-y-2">
              <div className="flex items-center gap-2">
                <span className="text-[11px] text-nx-text-muted w-20">{t("about.author")}</span>
                <span className="text-[11px] text-nx-text font-medium">
                  {manifest.author}
                </span>
              </div>
              <div className="flex items-center gap-2">
                <span className="text-[11px] text-nx-text-muted w-20">{t("about.license")}</span>
                <span className="text-[11px] text-nx-text font-medium">
                  {manifest.license ?? t("common:status.notSpecified")}
                </span>
              </div>
              <div className="flex items-start gap-2">
                <span className="text-[11px] text-nx-text-muted w-20 flex-shrink-0">{t("extensions.publicKey")}</span>
                <code className="text-[10px] text-nx-text-secondary bg-nx-deep px-2 py-1 rounded-[var(--radius-tag)] font-mono break-all">
                  {manifest.author_public_key.slice(0, 32)}...
                </code>
              </div>
              <div className="flex items-center gap-2">
                <span className="text-[11px] text-nx-text-muted w-20">{t("extensions.platforms")}</span>
                <div className="flex gap-1.5">
                  {Object.keys(manifest.binaries).map((platform) => (
                    <span
                      key={platform}
                      className="text-[10px] px-1.5 py-0.5 rounded-[var(--radius-tag)] bg-nx-overlay text-nx-text-secondary font-mono"
                    >
                      {platform}
                    </span>
                  ))}
                </div>
              </div>
            </div>
          </div>

          {/* Capabilities */}
          {manifest.capabilities.length > 0 && (
            <div className="bg-nx-surface rounded-[var(--radius-card)] border border-nx-border p-5">
              <div className="flex items-center gap-2 mb-3">
                <Shield size={13} strokeWidth={1.5} className="text-nx-warning" />
                <h4 className="text-[12px] font-semibold text-nx-text">
                  {t("extensions.declaredCapabilities")}
                </h4>
              </div>
              <p className="text-[10px] text-nx-text-ghost mb-3">
                {t("extensions.capabilitiesWarning")}
              </p>
              <div className="space-y-1">
                {manifest.capabilities.map((cap, i) => {
                  const Icon = capabilityIcon(cap);
                  const detail = capabilityDetail(cap);
                  return (
                    <div
                      key={i}
                      className="flex items-center gap-3 px-3 py-2 rounded-[var(--radius-button)] bg-nx-deep border border-nx-border-subtle"
                    >
                      <Icon size={13} strokeWidth={1.5} className="text-nx-text-muted flex-shrink-0" />
                      <span className="text-[11px] text-nx-text font-medium flex-shrink-0">
                        {capabilityLabel(cap, t)}
                      </span>
                      {detail && (
                        <span className="text-[10px] text-nx-text-ghost truncate font-mono">
                          {detail}
                        </span>
                      )}
                    </div>
                  );
                })}
              </div>
            </div>
          )}

          {/* Operations */}
          <div className="bg-nx-surface rounded-[var(--radius-card)] border border-nx-border p-5">
            <div className="flex items-center gap-2 mb-3">
              <Blocks size={13} strokeWidth={1.5} className="text-nx-text-muted" />
              <h4 className="text-[12px] font-semibold text-nx-text">
                {t("extensions.operations", { count: manifest.operations.length })}
              </h4>
            </div>
            <div className="space-y-1">
              {manifest.operations.map((op) => {
                const risk = RISK_STYLES[op.risk_level] ?? RISK_STYLES.medium;
                return (
                  <div
                    key={op.name}
                    className="flex items-center gap-3 px-3 py-2 rounded-[var(--radius-button)] bg-nx-deep border border-nx-border-subtle"
                  >
                    <span className="text-[12px] text-nx-text font-mono flex-shrink-0">
                      {op.name}
                    </span>
                    <span
                      className={`text-[9px] font-semibold px-1.5 py-0.5 rounded-[var(--radius-tag)] flex-shrink-0 ${risk.bg} ${risk.text}`}
                    >
                      {op.risk_level}
                    </span>
                    {op.scope_key && (
                      <span className="text-[9px] px-1.5 py-0.5 rounded-[var(--radius-tag)] bg-nx-overlay text-nx-text-muted font-mono flex-shrink-0">
                        {t("extensions.scope", { key: op.scope_key })}
                      </span>
                    )}
                    <span className="text-[11px] text-nx-text-ghost truncate flex-1">
                      {op.description}
                    </span>
                  </div>
                );
              })}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
