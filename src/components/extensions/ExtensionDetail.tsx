import { useState } from "react";
import type { ExtensionRegistryEntry, ExtensionManifest, Capability } from "../../types/extension";
import { extensionPreview, extensionInstall } from "../../lib/extensions";
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
} from "lucide-react";

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

function capabilityLabel(cap: Capability): string {
  switch (cap.type) {
    case "process_exec": return "Process Execution";
    case "file_read": return "File Read";
    case "file_write": return "File Write";
    case "network_http": return "Network HTTP";
    case "system_info": return "System Info";
    case "native_library": return "Native Library";
    case "custom": return cap.name;
  }
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
  const { addNotification } = useAppStore();
  const [loading, setLoading] = useState(false);
  const [installing, setInstalling] = useState(false);
  const [manifest, setManifest] = useState<ExtensionManifest | null>(null);

  async function handlePreview() {
    setLoading(true);
    try {
      const m = await extensionPreview(entry.manifest_url);
      setManifest(m);
    } catch (e) {
      addNotification(`Failed to fetch extension manifest: ${e}`, "error");
    } finally {
      setLoading(false);
    }
  }

  async function handleInstall() {
    setInstalling(true);
    try {
      await extensionInstall(entry.manifest_url);
      addNotification(`Extension "${entry.name}" installed`, "success");
      onBack();
    } catch (e) {
      addNotification(`Install failed: ${e}`, "error");
    } finally {
      setInstalling(false);
    }
  }

  return (
    <div className="p-6 max-w-2xl mx-auto">
      <button
        onClick={onBack}
        className="flex items-center gap-1.5 text-[12px] font-medium text-nx-text-muted hover:text-nx-text mb-6 transition-colors duration-150"
      >
        <ArrowLeft size={14} strokeWidth={1.5} />
        Back to Extensions
      </button>

      <div className="bg-nx-surface rounded-[var(--radius-card)] border border-nx-border p-6">
        <div className="flex items-start justify-between mb-4">
          <div>
            <h2 className="text-[18px] font-bold text-nx-text">{entry.name}</h2>
            <p className="text-[12px] text-nx-text-muted mt-1 font-mono">
              v{entry.version} &middot; {entry.id}
            </p>
          </div>
          {!manifest ? (
            <button
              onClick={handlePreview}
              disabled={loading}
              className="flex items-center gap-1.5 px-4 py-2 bg-nx-accent hover:bg-nx-accent-hover disabled:opacity-60 text-nx-deep text-[13px] font-medium rounded-[var(--radius-button)] transition-all duration-150"
            >
              {loading ? (
                <Loader2 size={14} strokeWidth={1.5} className="animate-spin" />
              ) : (
                <Shield size={14} strokeWidth={1.5} />
              )}
              {loading ? "Loading..." : "Review & Install"}
            </button>
          ) : (
            <button
              onClick={handleInstall}
              disabled={installing}
              className="flex items-center gap-1.5 px-4 py-2 bg-nx-accent hover:bg-nx-accent-hover disabled:opacity-60 text-nx-deep text-[13px] font-medium rounded-[var(--radius-button)] transition-all duration-150"
            >
              {installing ? (
                <Loader2 size={14} strokeWidth={1.5} className="animate-spin" />
              ) : (
                <Download size={14} strokeWidth={1.5} />
              )}
              {installing ? "Installing..." : "Install Extension"}
            </button>
          )}
        </div>

        <p className="text-nx-text-secondary text-[13px] mb-6 leading-relaxed">
          {entry.description}
        </p>

        {entry.categories.length > 0 && (
          <div className="mb-6">
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

      {/* Manifest preview — shown after clicking "Review & Install" */}
      {manifest && (
        <div className="mt-4 space-y-4">
          {/* Author + signature */}
          <div className="bg-nx-surface rounded-[var(--radius-card)] border border-nx-border p-5">
            <div className="flex items-center gap-2 mb-3">
              <Shield size={13} strokeWidth={1.5} className="text-nx-text-muted" />
              <h4 className="text-[12px] font-semibold text-nx-text">
                Author & Signature
              </h4>
            </div>
            <div className="space-y-2">
              <div className="flex items-center gap-2">
                <span className="text-[11px] text-nx-text-muted w-20">Author</span>
                <span className="text-[11px] text-nx-text font-medium">
                  {manifest.author}
                </span>
              </div>
              <div className="flex items-center gap-2">
                <span className="text-[11px] text-nx-text-muted w-20">License</span>
                <span className="text-[11px] text-nx-text font-medium">
                  {manifest.license ?? "Not specified"}
                </span>
              </div>
              <div className="flex items-start gap-2">
                <span className="text-[11px] text-nx-text-muted w-20 flex-shrink-0">Public key</span>
                <code className="text-[10px] text-nx-text-secondary bg-nx-deep px-2 py-1 rounded-[var(--radius-tag)] font-mono break-all">
                  {manifest.author_public_key.slice(0, 32)}...
                </code>
              </div>
              <div className="flex items-center gap-2">
                <span className="text-[11px] text-nx-text-muted w-20">Platforms</span>
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
                  Declared Capabilities
                </h4>
              </div>
              <p className="text-[10px] text-nx-text-ghost mb-3">
                This extension runs as a native process with these declared capabilities.
                Review carefully before installing.
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
                        {capabilityLabel(cap)}
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
                Operations ({manifest.operations.length})
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
                        scope: {op.scope_key}
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
