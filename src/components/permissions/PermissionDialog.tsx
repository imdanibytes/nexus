import { useCallback, useState } from "react";
import { useTranslation } from "react-i18next";
import type { Permission } from "../../types/permissions";
import type { PluginManifest } from "../../types/plugin";
import { getPermissionInfo, allPermissions, getManifestScopes } from "../../types/permissions";
import { useAppStore } from "../../stores/appStore";
import { Modal, ModalContent, Switch, Button, Chip } from "@heroui/react";
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
  Link,
} from "lucide-react";

const riskChipColors: Record<string, "success" | "warning" | "danger"> = {
  low: "success",
  medium: "warning",
  high: "danger",
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

  const handleModalOpenChange = useCallback((open: boolean) => { if (!open) onDeny(); }, [onDeny]);
  const handleBackToInfo = useCallback(() => setStep("info"), []);
  const handleBackFromMcp = useCallback(() => setStep(hasPermissions ? "permissions" : "info"), [hasPermissions]);
  const handleMcpApprove = useCallback(() => onApprove(approvedPerms, deferredPerms), [onApprove, approvedPerms, deferredPerms]);

  return (
    <Modal
      isOpen
      onOpenChange={handleModalOpenChange}
      hideCloseButton
    >
      <ModalContent>
        {/* Step indicator */}
        <div className="flex border-b border-default-100">
          {steps.map((s) => (
            <div
              key={s.id}
              className={`flex-1 px-4 py-2.5 text-[11px] font-semibold text-center uppercase tracking-wider transition-colors duration-150 ${
                step === s.id
                  ? "text-primary border-b-2 border-primary"
                  : "text-default-400"
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
              onBack={handleBackToInfo}
            />
          )}
          {step === "mcp_tools" && (
            <McpToolsStep
              manifest={manifest}
              onApprove={handleMcpApprove}
              onDeny={onDeny}
              onBack={handleBackFromMcp}
            />
          )}
        </div>
      </ModalContent>
    </Modal>
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
        <div className="w-12 h-12 rounded-[14px] bg-background border border-default-100 flex items-center justify-center flex-shrink-0">
          <Package size={22} strokeWidth={1.5} className="text-default-500" />
        </div>
        <div className="min-w-0">
          <h3 className="text-[18px] font-bold truncate">
            {manifest.name}
          </h3>
          <p className="text-[12px] text-default-500 font-mono">
            v{manifest.version} &middot; {manifest.id}
          </p>
        </div>
      </div>

      <p className="text-[13px] text-default-500 leading-relaxed mb-5">
        {manifest.description}
      </p>

      <div className="space-y-2 mb-6">
        <InfoRow label={t("dialog.author")} value={manifest.author} />
        <InfoRow label={t("dialog.license")} value={manifest.license ?? t("common:status.notSpecified")} />
        <InfoRow label={t("dialog.image")} value={manifest.image} mono />
        {manifest.homepage && (
          <div className="flex items-center justify-between py-2 border-b border-default-100">
            <span className="text-[12px] text-default-500">{t("dialog.homepage")}</span>
            <a
              href={manifest.homepage}
              target="_blank"
              rel="noopener noreferrer"
              className="text-[12px] text-primary hover:underline flex items-center gap-1"
            >
              {new URL(manifest.homepage).hostname}
              <ExternalLink size={10} strokeWidth={1.5} />
            </a>
          </div>
        )}
        {/* TODO: Signature verification status */}
        <div className="flex items-center justify-between py-2">
          <span className="text-[12px] text-default-500">{t("common:status.verified")}</span>
          <Chip size="sm" variant="flat">
          {t("common:status.unsigned")}
          </Chip>
        </div>
      </div>

      <div className="flex gap-3 justify-end">
        <Button variant="flat" onPress={onDeny}>
          {t("common:action.cancel")}
        </Button>
        <Button color="primary" onPress={onNext}>
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

/** Group permissions into built-in, extension, and MCP access groups. */
function groupPermissions(permissions: Permission[]) {
  const builtIn: Permission[] = [];
  const extGroups: Record<string, Permission[]> = {};
  const mcpAccess: Permission[] = [];

  for (const perm of permissions) {
    if (typeof perm === "string" && perm.startsWith("ext:")) {
      const parts = perm.slice(4).split(":");
      const extId = parts[0] ?? "unknown";
      if (!extGroups[extId]) extGroups[extId] = [];
      extGroups[extId].push(perm);
    } else if (typeof perm === "string" && perm.startsWith("mcp:") && perm !== "mcp:call") {
      mcpAccess.push(perm);
    } else {
      builtIn.push(perm);
    }
  }

  return { builtIn, extGroups, mcpAccess };
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

  const { builtIn, extGroups, mcpAccess } = groupPermissions(permissions);
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

  const handleNextPress = useCallback(() => {
    const [approved, deferred] = computeApprovedDeferred();
    onNext(approved, deferred);
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [toggles, permissions, onNext]);

  const handleApprovePress = useCallback(() => {
    const [approved, deferred] = computeApprovedDeferred();
    onApprove(approved, deferred);
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [toggles, permissions, onApprove]);

  const deferredCount = permissions.filter((p) => !toggles[p]).length;

  return (
    <>
      <h3 className="text-[16px] font-bold mb-1">
        {manifest.name}
      </h3>
      <p className="text-[13px] text-default-500 mb-1">
        {t("dialog.requestsPermissions")}
      </p>
      {deferredCount > 0 && (
        <p className="text-[11px] text-warning mb-4">
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
            // eslint-disable-next-line react-perf/jsx-no-new-function-as-prop
            onToggle={() => togglePerm(perm)}
          />
        ))}

        {/* Extension permission groups */}
        {Object.entries(extGroups).map(([extId, extPerms]) => {
          const isMissing = extensionsLoaded && !installedExtensions.has(extId);

          return (
            <div key={extId} className="space-y-1.5">
              <div className="flex items-center gap-2 pt-2 pb-1">
                <Puzzle size={12} strokeWidth={1.5} className="text-default-500" />
                <span className="text-[11px] font-semibold text-default-500 uppercase tracking-wider">
                  {extId}
                </span>
                {isMissing ? (
                  <span className="flex items-center gap-1 text-[10px] font-medium px-1.5 py-0.5 rounded-[6px] bg-warning-50 text-warning">
                    <AlertTriangle size={10} strokeWidth={1.5} />
                    {t("dialog.notInstalled")}
                  </span>
                ) : extensionsLoaded ? (
                  <span className="flex items-center gap-1 text-[10px] font-medium px-1.5 py-0.5 rounded-[6px] bg-success-50 text-success">
                    <Check size={10} strokeWidth={1.5} />
                    {t("common:status.installed")}
                  </span>
                ) : null}
              </div>
              {extPerms.map((perm) => {
                const scopes = getManifestScopes(manifest, perm);
                return (
                  <PermissionToggleRow
                    key={perm}
                    perm={perm}
                    enabled={toggles[perm]}
                    // eslint-disable-next-line react-perf/jsx-no-new-function-as-prop
                    onToggle={() => togglePerm(perm)}
                    scopes={scopes}
                  />
                );
              })}
            </div>
          );
        })}

        {/* MCP access permissions */}
        {mcpAccess.length > 0 && (
          <div className="space-y-1.5">
            <div className="flex items-center gap-2 pt-2 pb-1">
              <Link size={12} strokeWidth={1.5} className="text-default-500" />
              <span className="text-[11px] font-semibold text-default-500 uppercase tracking-wider">
                {t("dialog.mcpAccess")}
              </span>
            </div>
            {mcpAccess.map((perm) => (
              <PermissionToggleRow
                key={perm}
                perm={perm}
                enabled={toggles[perm]}
                // eslint-disable-next-line react-perf/jsx-no-new-function-as-prop
                onToggle={() => togglePerm(perm)}
              />
            ))}
          </div>
        )}
      </div>

      {/* Missing extension warnings */}
      {extensionsLoaded && declaredExtensions.some((e) => !installedExtensions.has(e)) && (
        <div className="mb-4 p-2.5 rounded-[8px] bg-warning-50/50 border border-warning/20">
          <p className="text-[11px] text-warning leading-relaxed">
            {t("dialog.extensionsMissing")}
          </p>
        </div>
      )}

      {!hasSeenAll && (
        <p className="text-[11px] text-default-400 text-center mb-3">
          {t("dialog.scrollToReview")}
        </p>
      )}

      <div className="flex justify-between">
        <Button variant="light" onPress={onBack}>
          <ArrowLeft size={14} strokeWidth={1.5} />
          {t("dialog.pluginInfo")}
        </Button>
        <div className="flex gap-3">
          <Button variant="flat" onPress={onDeny}>
            <ShieldX size={14} strokeWidth={1.5} />
            {t("common:action.deny")}
          </Button>
          {hasMcpTools ? (
            <Button
              color="primary"
              isDisabled={!hasSeenAll}
              onPress={handleNextPress}
            >
              {t("dialog.reviewMcpTools")}
              <ArrowRight size={14} strokeWidth={1.5} />
            </Button>
          ) : (
            <Button
              color="primary"
              isDisabled={!hasSeenAll}
              onPress={handleApprovePress}
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
  scopes,
}: {
  perm: string;
  enabled: boolean;
  onToggle: () => void;
  scopes?: string[] | null;
}) {
  const { t } = useTranslation("permissions");
  const info = getPermissionInfo(perm);

  return (
    <div
      className={`flex items-center justify-between p-3 rounded-[8px] border transition-colors duration-150 ${
        enabled
          ? "bg-background border-default-100"
          : "bg-background/50 border-default-100/50 opacity-60"
      }`}
    >
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-2">
          <p className="text-[12px] font-medium font-mono">
            {perm}
          </p>
          <Chip size="sm" variant="flat" color={riskChipColors[info.risk]}>
            {info.risk}
          </Chip>
        </div>
        <p className="text-[11px] text-default-500 mt-0.5">
          {info.description}
        </p>
        {scopes && scopes.length > 0 && (
          <div className="flex flex-wrap gap-1 mt-1.5">
            {scopes.map((scope) => (
              <Chip key={scope} size="sm" variant="flat">
                {scope}
              </Chip>
            ))}
          </div>
        )}
      </div>
      <Switch
        isSelected={enabled}
        onValueChange={onToggle}
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
        <Cpu size={16} strokeWidth={1.5} className="text-primary" />
        <h3 className="text-[16px] font-bold">
          {t("dialog.mcpTools")}
        </h3>
      </div>
      <p className="text-[13px] text-default-500 mb-5">
        {t("dialog.mcpToolsDesc", { count: mcpTools.length })}
      </p>

      <div className="space-y-2 mb-6 max-h-64 overflow-y-auto">
        {mcpTools.map((tool) => (
          <div
            key={tool.name}
            className="p-3 rounded-[8px] bg-background border border-default-100"
          >
            <div className="flex items-center gap-2 mb-1">
              <Wrench size={11} strokeWidth={1.5} className="text-default-500 flex-shrink-0" />
              <p className="text-[12px] font-medium font-mono">
                {tool.name}
              </p>
            </div>
            <p className="text-[11px] text-default-500 mb-1.5 ml-[19px]">
              {tool.description}
            </p>
            {tool.permissions.length > 0 && (
              <div className="flex flex-wrap gap-1 ml-[19px]">
                {tool.permissions.map((perm) => {
                  const info = getPermissionInfo(perm);
                  return (
                    <Chip key={perm} size="sm" variant="flat" color={riskChipColors[info.risk]}>
                      {perm}
                    </Chip>
                  );
                })}
              </div>
            )}
          </div>
        ))}
      </div>

      <p className="text-[11px] text-default-400 mb-5">
        {t("dialog.mcpToolsCanToggle")}
      </p>

      <div className="flex justify-between">
        <Button variant="light" onPress={onBack}>
          <ArrowLeft size={14} strokeWidth={1.5} />
          {t("common:action.back")}
        </Button>
        <div className="flex gap-3">
          <Button variant="flat" onPress={onDeny}>
            <ShieldX size={14} strokeWidth={1.5} />
            {t("common:action.deny")}
          </Button>
          <Button color="primary" onPress={onApprove}>
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
    <div className="flex items-center justify-between py-2 border-b border-default-100">
      <span className="text-[12px] text-default-500">{label}</span>
      <span
        className={`text-[12px] ${mono ? "font-mono" : ""}`}
      >
        {value}
      </span>
    </div>
  );
}
