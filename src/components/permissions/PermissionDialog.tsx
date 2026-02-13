import { useState } from "react";
import type { Permission } from "../../types/permissions";
import type { PluginManifest } from "../../types/plugin";
import { PERMISSION_INFO } from "../../types/permissions";
import { ShieldCheck, ShieldX, ArrowLeft, ArrowRight, Package, ExternalLink } from "lucide-react";

const riskColors = {
  low: "text-nx-success bg-nx-success-muted",
  medium: "text-nx-warning bg-nx-warning-muted",
  high: "text-nx-error bg-nx-error-muted",
};

type Step = "info" | "permissions";

interface Props {
  manifest: PluginManifest;
  onApprove: (permissions: Permission[]) => void;
  onDeny: () => void;
}

export function PermissionDialog({ manifest, onApprove, onDeny }: Props) {
  const requestedPermissions = (manifest.permissions ?? []) as Permission[];
  const hasPermissions = requestedPermissions.length > 0;
  const [step, setStep] = useState<Step>("info");

  function handleInfoNext() {
    if (hasPermissions) {
      setStep("permissions");
    } else {
      onApprove([]);
    }
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      <div
        className="absolute inset-0 bg-black/50 backdrop-blur-sm"
        onClick={onDeny}
      />
      <div
        className="relative bg-nx-surface border border-nx-border rounded-[var(--radius-modal)] shadow-[var(--shadow-modal)] max-w-md w-full mx-4 overflow-hidden"
        style={{ animation: "toast-enter 200ms ease-out" }}
      >
        {/* Step indicator */}
        <div className="flex border-b border-nx-border-subtle">
          <div
            className={`flex-1 px-4 py-2.5 text-[11px] font-semibold text-center uppercase tracking-wider transition-colors duration-150 ${
              step === "info"
                ? "text-nx-accent border-b-2 border-nx-accent"
                : "text-nx-text-ghost"
            }`}
          >
            Plugin Info
          </div>
          <div
            className={`flex-1 px-4 py-2.5 text-[11px] font-semibold text-center uppercase tracking-wider transition-colors duration-150 ${
              step === "permissions"
                ? "text-nx-accent border-b-2 border-nx-accent"
                : "text-nx-text-ghost"
            }`}
          >
            Permissions{hasPermissions ? ` (${requestedPermissions.length})` : ""}
          </div>
        </div>

        <div className="p-6">
          {step === "info" ? (
            <InfoStep
              manifest={manifest}
              hasPermissions={hasPermissions}
              onNext={handleInfoNext}
              onDeny={onDeny}
            />
          ) : (
            <PermissionsStep
              manifest={manifest}
              permissions={requestedPermissions}
              onApprove={onApprove}
              onDeny={onDeny}
              onBack={() => setStep("info")}
            />
          )}
        </div>
      </div>
    </div>
  );
}

function InfoStep({
  manifest,
  hasPermissions,
  onNext,
  onDeny,
}: {
  manifest: PluginManifest;
  hasPermissions: boolean;
  onNext: () => void;
  onDeny: () => void;
}) {
  return (
    <>
      <div className="flex items-start gap-4 mb-5">
        <div className="w-12 h-12 rounded-[var(--radius-card)] bg-nx-deep border border-nx-border-subtle flex items-center justify-center flex-shrink-0">
          <Package size={22} strokeWidth={1.5} className="text-nx-text-muted" />
        </div>
        <div className="min-w-0">
          <h3 className="text-[18px] font-bold text-nx-text truncate">
            {manifest.name}
          </h3>
          <p className="text-[12px] text-nx-text-muted font-mono">
            v{manifest.version} &middot; {manifest.id}
          </p>
        </div>
      </div>

      <p className="text-[13px] text-nx-text-secondary leading-relaxed mb-5">
        {manifest.description}
      </p>

      <div className="space-y-2 mb-6">
        <InfoRow label="Author" value={manifest.author} />
        <InfoRow label="License" value={manifest.license ?? "Not specified"} />
        <InfoRow label="Image" value={manifest.image} mono />
        {manifest.homepage && (
          <div className="flex items-center justify-between py-2 border-b border-nx-border-subtle">
            <span className="text-[12px] text-nx-text-muted">Homepage</span>
            <a
              href={manifest.homepage}
              target="_blank"
              rel="noopener noreferrer"
              className="text-[12px] text-nx-accent hover:underline flex items-center gap-1"
            >
              {new URL(manifest.homepage).hostname}
              <ExternalLink size={10} strokeWidth={1.5} />
            </a>
          </div>
        )}
        {/* TODO: Signature verification status */}
        <div className="flex items-center justify-between py-2">
          <span className="text-[12px] text-nx-text-muted">Verified</span>
          <span className="text-[11px] px-2 py-0.5 rounded-[var(--radius-tag)] bg-nx-overlay text-nx-text-ghost font-medium">
            Unsigned
          </span>
        </div>
      </div>

      <div className="flex gap-3 justify-end">
        <button
          onClick={onDeny}
          className="px-4 py-2 text-[13px] font-medium rounded-[var(--radius-button)] bg-nx-overlay hover:bg-nx-wash text-nx-text-secondary transition-all duration-150"
        >
          Cancel
        </button>
        <button
          onClick={onNext}
          className="flex items-center gap-1.5 px-4 py-2 text-[13px] font-medium rounded-[var(--radius-button)] bg-nx-accent hover:bg-nx-accent-hover text-nx-deep transition-all duration-150"
        >
          {hasPermissions ? (
            <>
              Review Permissions
              <ArrowRight size={14} strokeWidth={1.5} />
            </>
          ) : (
            <>
              <ShieldCheck size={14} strokeWidth={1.5} />
              Install
            </>
          )}
        </button>
      </div>
    </>
  );
}

function PermissionsStep({
  manifest,
  permissions,
  onApprove,
  onDeny,
  onBack,
}: {
  manifest: PluginManifest;
  permissions: Permission[];
  onApprove: (perms: Permission[]) => void;
  onDeny: () => void;
  onBack: () => void;
}) {
  return (
    <>
      <h3 className="text-[16px] font-bold text-nx-text mb-1">
        {manifest.name}
      </h3>
      <p className="text-[13px] text-nx-text-secondary mb-5">
        This plugin requests the following permissions:
      </p>

      <div className="space-y-2 mb-6">
        {permissions.map((perm) => {
          const info = PERMISSION_INFO[perm];
          if (!info) return null;
          return (
            <div
              key={perm}
              className="flex items-center justify-between p-3 rounded-[var(--radius-button)] bg-nx-deep border border-nx-border-subtle"
            >
              <div>
                <p className="text-[12px] text-nx-text font-medium font-mono">
                  {perm}
                </p>
                <p className="text-[11px] text-nx-text-muted mt-0.5">
                  {info.description}
                </p>
              </div>
              <span
                className={`text-[10px] px-2 py-0.5 rounded-[var(--radius-tag)] font-semibold capitalize ${riskColors[info.risk]}`}
              >
                {info.risk}
              </span>
            </div>
          );
        })}
      </div>

      <div className="flex justify-between">
        <button
          onClick={onBack}
          className="flex items-center gap-1.5 px-4 py-2 text-[13px] font-medium rounded-[var(--radius-button)] text-nx-text-muted hover:text-nx-text-secondary transition-colors duration-150"
        >
          <ArrowLeft size={14} strokeWidth={1.5} />
          Plugin Info
        </button>
        <div className="flex gap-3">
          <button
            onClick={onDeny}
            className="flex items-center gap-1.5 px-4 py-2 text-[13px] font-medium rounded-[var(--radius-button)] bg-nx-overlay hover:bg-nx-wash text-nx-text-secondary transition-all duration-150"
          >
            <ShieldX size={14} strokeWidth={1.5} />
            Deny
          </button>
          <button
            onClick={() => onApprove(permissions)}
            className="flex items-center gap-1.5 px-4 py-2 text-[13px] font-medium rounded-[var(--radius-button)] bg-nx-accent hover:bg-nx-accent-hover text-nx-deep transition-all duration-150"
          >
            <ShieldCheck size={14} strokeWidth={1.5} />
            Approve & Install
          </button>
        </div>
      </div>
    </>
  );
}

function InfoRow({
  label,
  value,
  mono,
}: {
  label: string;
  value: string;
  mono?: boolean;
}) {
  return (
    <div className="flex items-center justify-between py-2 border-b border-nx-border-subtle">
      <span className="text-[12px] text-nx-text-muted">{label}</span>
      <span
        className={`text-[12px] text-nx-text ${mono ? "font-mono" : ""}`}
      >
        {value}
      </span>
    </div>
  );
}
