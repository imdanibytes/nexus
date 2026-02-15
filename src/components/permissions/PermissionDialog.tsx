import { useCallback, useState } from "react";
import { useTranslation } from "react-i18next";
import type { Permission } from "../../types/permissions";
import type { PluginManifest } from "../../types/plugin";
import { getPermissionInfo, allPermissions } from "../../types/permissions";
import { useAppStore } from "../../stores/appStore";
import { Dialog, DialogContent } from "@/components/ui/dialog";
import { Switch } from "@/components/ui/switch";
import { Button } from "@/components/ui/button";
import {
  ShieldCheck,
  ShieldX,
  ArrowLeft,
  ArrowRight,
  Package,
  ExternalLink,
  Cpu,
  Wrench,
  Puzzle,
  AlertTriangle,
  Check,
} from "lucide-react";

const riskColors = {
  low: "text-nx-success bg-nx-success-muted",
  medium: "text-nx-warning bg-nx-warning-muted",
  high: "text-nx-error bg-nx-error-muted",
};

type Step = "info" | "permissions" | "mcp_tools";

interface Props {
  manifest: PluginManifest;
  onApprove: (approved: Permission[], deferred: Permission[]) => void;
  onDeny: () => void;
}

export function PermissionDialog({ manifest, onApprove, onDeny }: Props) {
  const { t } = useTranslation("permissions");
  const requestedPermissions = allPermissions(manifest) as Permission[];
  const hasPermissions = requestedPermissions.length > 0;
  const mcpTools = manifest.mcp?.tools ?? [];
  const hasMcpTools = mcpTools.length > 0;
  const [step, setStep] = useState<Step>("info");
  // Track the final approved/deferred split from the permissions step
  const [approvedPerms, setApprovedPerms] = useState<Permission[]>(requestedPermissions);
  const [deferredPerms, setDeferredPerms] = useState<Permission[]>([]);

  function handleInfoNext() {
    if (hasPermissions) {
      setStep("permissions");
    } else if (hasMcpTools) {
      setStep("mcp_tools");
    } else {
      onApprove([], []);
    }
  }

  function handlePermissionsNext(approved: Permission[], deferred: Permission[]) {
    setApprovedPerms(approved);
    setDeferredPerms(deferred);
    if (hasMcpTools) {
      setStep("mcp_tools");
    } else {
      onApprove(approved, deferred);
    }
  }

  // Determine which steps are visible for the step indicator
  const steps: { id: Step; label: string; count?: number }[] = [
    { id: "info", label: t("dialog.pluginInfo") },
  ];
  if (hasPermissions) {
    steps.push({ id: "permissions", label: t("dialog.permissions"), count: requestedPermissions.length });
  }
  if (hasMcpTools) {
    steps.push({ id: "mcp_tools", label: t("dialog.mcpTools"), count: mcpTools.length });
  }

  return (
    <Dialog open onOpenChange={(open) => { if (!open) onDeny(); }}>
      <DialogContent showCloseButton={false} className="max-w-md p-0 gap-0 overflow-hidden">
        {/* Step indicator */}
        <div className="flex border-b border-nx-border-subtle">
          {steps.map((s) => (
            <div
              key={s.id}
              className={`flex-1 px-4 py-2.5 text-[11px] font-semibold text-center uppercase tracking-wider transition-colors duration-150 ${
                step === s.id
                  ? "text-nx-accent border-b-2 border-nx-accent"
                  : "text-nx-text-ghost"
              }`}
            >
              {s.label}{s.count != null ? ` (${s.count})` : ""}
            </div>
          ))}
        </div>

        <div className="p-6">
          {step === "info" && (
            <InfoStep
              manifest={manifest}
              hasMoreSteps={hasPermissions || hasMcpTools}
              onNext={handleInfoNext}
              onDeny={onDeny}
            />
          )}
          {step === "permissions" && (
            <PermissionsStep
              manifest={manifest}
              permissions={requestedPermissions}
              hasMcpTools={hasMcpTools}
              onNext={handlePermissionsNext}
              onApprove={onApprove}
              onDeny={onDeny}
              onBack={() => setStep("info")}
            />
          )}
          {step === "mcp_tools" && (
            <McpToolsStep
              manifest={manifest}
              onApprove={() => onApprove(approvedPerms, deferredPerms)}
              onDeny={onDeny}
              onBack={() => setStep(hasPermissions ? "permissions" : "info")}
            />
          )}
        </div>
      </DialogContent>
    </Dialog>
  );
}

function InfoStep({
  manifest,
  hasMoreSteps,
  onNext,
  onDeny,
}: {
  manifest: PluginManifest;
  hasMoreSteps: boolean;
  onNext: () => void;
  onDeny: () => void;
}) {
  const { t } = useTranslation("permissions");
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
        <InfoRow label={t("dialog.author")} value={manifest.author} />
        <InfoRow label={t("dialog.license")} value={manifest.license ?? t("common:status.notSpecified")} />
        <InfoRow label={t("dialog.image")} value={manifest.image} mono />
        {manifest.homepage && (
          <div className="flex items-center justify-between py-2 border-b border-nx-border-subtle">
            <span className="text-[12px] text-nx-text-muted">{t("dialog.homepage")}</span>
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
          <span className="text-[12px] text-nx-text-muted">{t("common:status.verified")}</span>
          <span className="text-[11px] px-2 py-0.5 rounded-[var(--radius-tag)] bg-nx-overlay text-nx-text-ghost font-medium">
            {t("common:status.unsigned")}
          </span>
        </div>
      </div>

      <div className="flex gap-3 justify-end">
        <Button variant="secondary" onClick={onDeny}>
          {t("common:action.cancel")}
        </Button>
        <Button onClick={onNext}>
          {hasMoreSteps ? (
            <>
              {t("common:action.continue")}
              <ArrowRight size={14} strokeWidth={1.5} />
            </>
          ) : (
            <>
              <ShieldCheck size={14} strokeWidth={1.5} />
              {t("common:action.install")}
            </>
          )}
        </Button>
      </div>
    </>
  );
}

/** Group permissions into built-in and extension groups. */
function groupPermissions(permissions: Permission[]) {
  const builtIn: Permission[] = [];
  const extGroups: Record<string, Permission[]> = {};

  for (const perm of permissions) {
    if (typeof perm === "string" && perm.startsWith("ext:")) {
      const parts = perm.slice(4).split(":");
      const extId = parts[0] ?? "unknown";
      if (!extGroups[extId]) extGroups[extId] = [];
      extGroups[extId].push(perm);
    } else {
      builtIn.push(perm);
    }
  }

  return { builtIn, extGroups };
}

function PermissionsStep({
  manifest,
  permissions,
  hasMcpTools,
  onNext,
  onApprove,
  onDeny,
  onBack,
}: {
  manifest: PluginManifest;
  permissions: Permission[];
  hasMcpTools: boolean;
  onNext: (approved: Permission[], deferred: Permission[]) => void;
  onApprove: (approved: Permission[], deferred: Permission[]) => void;
  onDeny: () => void;
  onBack: () => void;
}) {
  const { t } = useTranslation("permissions");
  const [hasSeenAll, setHasSeenAll] = useState(false);
  // Per-permission toggle: true = approved (active), false = deferred
  const [toggles, setToggles] = useState<Record<string, boolean>>(() => {
    const initial: Record<string, boolean> = {};
    for (const perm of permissions) {
      initial[perm] = true; // Default: all ON (approved)
    }
    return initial;
  });

  // Extension availability — read from centralized store (polled by useExtensions)
  const storeExtensions = useAppStore((s) => s.installedExtensions);
  const installedExtensions = new Set(storeExtensions.map((e) => e.id));
  const extensionsLoaded = true;

  const { builtIn, extGroups } = groupPermissions(permissions);
  const declaredExtensions = Object.keys(manifest.extensions ?? {});

  const listRef = useCallback((el: HTMLDivElement | null) => {
    if (!el) return;
    if (el.scrollHeight <= el.clientHeight) {
      setHasSeenAll(true);
    }
  }, []);

  function handleScroll(e: React.UIEvent<HTMLDivElement>) {
    const el = e.currentTarget;
    if (el.scrollTop + el.clientHeight >= el.scrollHeight - 4) {
      setHasSeenAll(true);
    }
  }

  function togglePerm(perm: string) {
    setToggles((prev) => ({ ...prev, [perm]: !prev[perm] }));
  }

  function computeApprovedDeferred(): [Permission[], Permission[]] {
    const approved: Permission[] = [];
    const deferred: Permission[] = [];
    for (const perm of permissions) {
      if (toggles[perm]) {
        approved.push(perm);
      } else {
        deferred.push(perm);
      }
    }
    return [approved, deferred];
  }

  const deferredCount = permissions.filter((p) => !toggles[p]).length;

  return (
    <>
      <h3 className="text-[16px] font-bold text-nx-text mb-1">
        {manifest.name}
      </h3>
      <p className="text-[13px] text-nx-text-secondary mb-1">
        {t("dialog.requestsPermissions")}
      </p>
      {deferredCount > 0 && (
        <p className="text-[11px] text-nx-warning mb-4">
          {t("dialog.deferredCount", { count: deferredCount })}
        </p>
      )}
      {deferredCount === 0 && <div className="mb-4" />}

      <div
        ref={listRef}
        onScroll={handleScroll}
        className="space-y-2 mb-4 max-h-64 overflow-y-auto"
      >
        {/* Built-in permissions */}
        {builtIn.map((perm) => (
          <PermissionToggleRow
            key={perm}
            perm={perm}
            enabled={toggles[perm]}
            onToggle={() => togglePerm(perm)}
          />
        ))}

        {/* Extension permission groups */}
        {Object.entries(extGroups).map(([extId, extPerms]) => {
          const isMissing = extensionsLoaded && !installedExtensions.has(extId);

          return (
            <div key={extId} className="space-y-1.5">
              <div className="flex items-center gap-2 pt-2 pb-1">
                <Puzzle size={12} strokeWidth={1.5} className="text-nx-text-muted" />
                <span className="text-[11px] font-semibold text-nx-text-muted uppercase tracking-wider">
                  {extId}
                </span>
                {isMissing ? (
                  <span className="flex items-center gap-1 text-[10px] font-medium px-1.5 py-0.5 rounded-[var(--radius-tag)] bg-nx-warning-muted text-nx-warning">
                    <AlertTriangle size={10} strokeWidth={1.5} />
                    {t("dialog.notInstalled")}
                  </span>
                ) : extensionsLoaded ? (
                  <span className="flex items-center gap-1 text-[10px] font-medium px-1.5 py-0.5 rounded-[var(--radius-tag)] bg-nx-success-muted text-nx-success">
                    <Check size={10} strokeWidth={1.5} />
                    {t("common:status.installed")}
                  </span>
                ) : null}
              </div>
              {extPerms.map((perm) => (
                <PermissionToggleRow
                  key={perm}
                  perm={perm}
                  enabled={toggles[perm]}
                  onToggle={() => togglePerm(perm)}
                />
              ))}
            </div>
          );
        })}
      </div>

      {/* Missing extension warnings */}
      {extensionsLoaded && declaredExtensions.some((e) => !installedExtensions.has(e)) && (
        <div className="mb-4 p-2.5 rounded-[var(--radius-button)] bg-nx-warning-muted/50 border border-nx-warning/20">
          <p className="text-[11px] text-nx-warning leading-relaxed">
            {t("dialog.extensionsMissing")}
          </p>
        </div>
      )}

      {!hasSeenAll && (
        <p className="text-[11px] text-nx-text-ghost text-center mb-3">
          {t("dialog.scrollToReview")}
        </p>
      )}

      <div className="flex justify-between">
        <Button variant="ghost" onClick={onBack} className="text-nx-text-muted hover:text-nx-text-secondary">
          <ArrowLeft size={14} strokeWidth={1.5} />
          {t("dialog.pluginInfo")}
        </Button>
        <div className="flex gap-3">
          <Button variant="secondary" onClick={onDeny}>
            <ShieldX size={14} strokeWidth={1.5} />
            {t("common:action.deny")}
          </Button>
          {hasMcpTools ? (
            <Button
              disabled={!hasSeenAll}
              onClick={() => {
                const [approved, deferred] = computeApprovedDeferred();
                onNext(approved, deferred);
              }}
            >
              {t("dialog.reviewMcpTools")}
              <ArrowRight size={14} strokeWidth={1.5} />
            </Button>
          ) : (
            <Button
              disabled={!hasSeenAll}
              onClick={() => {
                const [approved, deferred] = computeApprovedDeferred();
                onApprove(approved, deferred);
              }}
            >
              <ShieldCheck size={14} strokeWidth={1.5} />
              {t("dialog.approveAndInstall")}
            </Button>
          )}
        </div>
      </div>
    </>
  );
}

/** A single permission row with toggle switch. */
function PermissionToggleRow({
  perm,
  enabled,
  onToggle,
}: {
  perm: string;
  enabled: boolean;
  onToggle: () => void;
}) {
  const { t } = useTranslation("permissions");
  const info = getPermissionInfo(perm);

  return (
    <div
      className={`flex items-center justify-between p-3 rounded-[var(--radius-button)] border transition-colors duration-150 ${
        enabled
          ? "bg-nx-deep border-nx-border-subtle"
          : "bg-nx-deep/50 border-nx-border-subtle/50 opacity-60"
      }`}
    >
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-2">
          <p className="text-[12px] text-nx-text font-medium font-mono">
            {perm}
          </p>
          <span
            className={`text-[10px] px-2 py-0.5 rounded-[var(--radius-tag)] font-semibold capitalize ${riskColors[info.risk]}`}
          >
            {info.risk}
          </span>
        </div>
        <p className="text-[11px] text-nx-text-muted mt-0.5">
          {info.description}
        </p>
      </div>
      <Switch
        checked={enabled}
        onCheckedChange={() => onToggle()}
        className="ml-3"
        aria-label={enabled ? t("dialog.approvedClickToDefer") : t("dialog.deferredClickToApprove")}
      />
    </div>
  );
}

function McpToolsStep({
  manifest,
  onApprove,
  onDeny,
  onBack,
}: {
  manifest: PluginManifest;
  onApprove: () => void;
  onDeny: () => void;
  onBack: () => void;
}) {
  const { t } = useTranslation("permissions");
  const mcpTools = manifest.mcp?.tools ?? [];

  return (
    <>
      <div className="flex items-center gap-2 mb-1">
        <Cpu size={16} strokeWidth={1.5} className="text-nx-accent" />
        <h3 className="text-[16px] font-bold text-nx-text">
          {t("dialog.mcpTools")}
        </h3>
      </div>
      <p className="text-[13px] text-nx-text-secondary mb-5">
        {t("dialog.mcpToolsDesc", { count: mcpTools.length })}
      </p>

      <div className="space-y-2 mb-6 max-h-64 overflow-y-auto">
        {mcpTools.map((tool) => (
          <div
            key={tool.name}
            className="p-3 rounded-[var(--radius-button)] bg-nx-deep border border-nx-border-subtle"
          >
            <div className="flex items-center gap-2 mb-1">
              <Wrench size={11} strokeWidth={1.5} className="text-nx-text-muted flex-shrink-0" />
              <p className="text-[12px] text-nx-text font-medium font-mono">
                {tool.name}
              </p>
            </div>
            <p className="text-[11px] text-nx-text-muted mb-1.5 ml-[19px]">
              {tool.description}
            </p>
            {tool.permissions.length > 0 && (
              <div className="flex flex-wrap gap-1 ml-[19px]">
                {tool.permissions.map((perm) => {
                  const info = getPermissionInfo(perm);
                  return (
                    <span
                      key={perm}
                      className={`text-[9px] font-medium px-1.5 py-0.5 rounded-[var(--radius-tag)] ${riskColors[info.risk]}`}
                    >
                      {perm}
                    </span>
                  );
                })}
              </div>
            )}
          </div>
        ))}
      </div>

      <p className="text-[11px] text-nx-text-ghost mb-5">
        {t("dialog.mcpToolsCanToggle")}
      </p>

      <div className="flex justify-between">
        <Button variant="ghost" onClick={onBack} className="text-nx-text-muted hover:text-nx-text-secondary">
          <ArrowLeft size={14} strokeWidth={1.5} />
          {t("common:action.back")}
        </Button>
        <div className="flex gap-3">
          <Button variant="secondary" onClick={onDeny}>
            <ShieldX size={14} strokeWidth={1.5} />
            {t("common:action.deny")}
          </Button>
          <Button onClick={onApprove}>
            <ShieldCheck size={14} strokeWidth={1.5} />
            {t("dialog.approveAndInstall")}
          </Button>
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
