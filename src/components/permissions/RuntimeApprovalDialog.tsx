import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { listen } from "@tauri-apps/api/event";
import {
  ShieldCheck,
  ShieldAlert,
  ShieldX,
  FolderOpen,
  Puzzle,
  AlertTriangle,
  Link,
} from "lucide-react";
import { runtimeApprovalRespond } from "../../lib/tauri";
import { useOsNotification } from "../../hooks/useOsNotification";
import { getPermissionInfo } from "../../types/permissions";
import type {
  ApprovalDecision,
  RuntimeApprovalRequest,
} from "../../types/permissions";
import { Modal, ModalContent, Button, Chip } from "@heroui/react";
import i18n from "../../i18n";

/** Derive a human-readable header from the approval category. */
function resolveHeader(req: RuntimeApprovalRequest): {
  icon: typeof FolderOpen;
  title: string;
  subtitle: string;
  iconBg: string;
  iconColor: string;
} {
  const t = i18n.t.bind(i18n);
  const ctx = req.context;

  // Deferred permission — JIT approval on first use
  if (req.category === "deferred_permission") {
    const desc = ctx.description ?? ctx.operation_description ?? req.permission;
    return {
      icon: ShieldAlert,
      title: t("permissions:runtime.permissionRequired"),
      subtitle: t("permissions:runtime.deferredSubtitle", { pluginName: req.plugin_name, description: desc }),
      iconBg: "bg-warning-50",
      iconColor: "text-warning",
    };
  }

  // Extension high-risk operation
  if (req.category.startsWith("extension:")) {
    const extName = ctx.extension_display_name ?? ctx.extension ?? req.category;
    const opDesc = ctx.operation_description ?? ctx.operation ?? "an operation";
    return {
      icon: AlertTriangle,
      title: t("permissions:runtime.highRiskOperation"),
      subtitle: t("permissions:runtime.extensionSubtitle", { pluginName: req.plugin_name, operation: opDesc, extensionName: extName }),
      iconBg: "bg-red-500/10",
      iconColor: "text-red-400",
    };
  }

  // Extension scope approval
  if (req.category.startsWith("extension_scope:")) {
    const extName = ctx.extension_display_name ?? ctx.extension ?? req.category;
    const opName = ctx.operation ?? "an operation";
    const scopeDesc = ctx.scope_description ?? ctx.scope_key ?? "a resource";
    const scopeVal = ctx.scope_value ?? "";
    return {
      icon: Puzzle,
      title: t("permissions:runtime.accessTitle", { extName }),
      subtitle: scopeVal
        ? t("permissions:runtime.extensionAccessSubtitle", { pluginName: req.plugin_name, operation: opName, scope: scopeDesc, value: scopeVal })
        : t("permissions:runtime.extensionAccessSubtitleShort", { pluginName: req.plugin_name, operation: opName }),
      iconBg: "bg-warning-50",
      iconColor: "text-warning",
    };
  }

  // OAuth authorization — AI client wants to connect
  if (req.category === "oauth_authorize") {
    const clientName = ctx.client_name ?? "Unknown client";
    return {
      icon: Link,
      title: t("permissions:runtime.oauthConnect"),
      subtitle: t("permissions:runtime.oauthSubtitle", { clientName }),
      iconBg: "bg-primary/10",
      iconColor: "text-primary",
    };
  }

  // Filesystem
  if (req.category === "filesystem") {
    return {
      icon: FolderOpen,
      title: t("permissions:runtime.fileAccess"),
      subtitle: t("permissions:runtime.filesystemSubtitle", { pluginName: req.plugin_name, permission: req.permission.replace(":", " ") }),
      iconBg: "bg-warning-50",
      iconColor: "text-warning",
    };
  }

  // Network
  if (req.category === "network") {
    return {
      icon: ShieldAlert,
      title: t("permissions:runtime.networkAccess"),
      subtitle: t("permissions:runtime.networkSubtitle", { pluginName: req.plugin_name, permission: req.permission.replace(":", " ") }),
      iconBg: "bg-warning-50",
      iconColor: "text-warning",
    };
  }

  // MCP tool invocation
  if (req.category === "mcp_tool") {
    const toolName = ctx.tool_name ?? "a tool";
    return {
      icon: ShieldAlert,
      title: t("permissions:runtime.mcpToolCall"),
      subtitle: t("permissions:runtime.mcpToolSubtitle", { pluginName: req.plugin_name, toolName }),
      iconBg: "bg-warning-50",
      iconColor: "text-warning",
    };
  }

  // Fallback
  return {
    icon: ShieldAlert,
    title: t("permissions:runtime.categoryRequest", { category: req.category }),
    subtitle: t("permissions:runtime.fallbackSubtitle", { pluginName: req.plugin_name, permission: req.permission.replace(":", " ") }),
    iconBg: "bg-warning-50",
    iconColor: "text-warning",
  };
}

/** Cooldown duration in seconds based on risk level. */
function cooldownFor(riskLevel: string | undefined): number {
  switch (riskLevel) {
    case "high":
      return 4;
    case "medium":
      return 2;
    default:
      return 0;
  }
}

export function RuntimeApprovalDialog() {
  const [queue, setQueue] = useState<RuntimeApprovalRequest[]>([]);
  const [cooldown, setCooldown] = useState(0);
  const { notify } = useOsNotification();
  // Dedup set survives StrictMode mount/unmount/remount cycle and the
  // async unlisten race that can leave two listeners briefly active.
  const seenIds = useRef(new Set<string>());

  useEffect(() => {
    const unlisten = listen<RuntimeApprovalRequest>(
      "nexus://runtime-approval",
      (event) => {
        const id = event.payload.id;
        if (seenIds.current.has(id)) return;
        seenIds.current.add(id);

        setQueue((prev) => [...prev, event.payload]);
        const header = resolveHeader(event.payload);
        notify(i18n.t("permissions:runtime.notificationTitle"), header.subtitle, 1);
      }
    );
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [notify]);

  const current = queue.length > 0 ? queue[0] : null;

  // Reset cooldown when the active request changes.
  // Derive during render (React-recommended pattern for adjusting state when
  // derived values change) instead of inside useEffect.
  const prevRequestId = useRef<string | undefined>(undefined);
  if (current?.id !== prevRequestId.current) {
    prevRequestId.current = current?.id;
    const duration = current ? cooldownFor(current.context.risk_level) : 0;
    setCooldown(duration);
  }

  // Countdown timer — recursive setTimeout avoids setInterval dance
  useEffect(() => {
    if (cooldown <= 0) return;
    const timer = setTimeout(() => setCooldown((c) => c - 1), 1000);
    return () => clearTimeout(timer);
  }, [cooldown]);

  async function respond(decision: ApprovalDecision) {
    if (!current) return;
    try {
      await runtimeApprovalRespond(
        current.id,
        decision,
        current.plugin_id,
        current.category,
        {
          ...current.context,
          permission: current.permission,
        }
      );
    } catch (err) {
      console.error("[RuntimeApproval] respond failed:", err);
    }
    seenIds.current.delete(current.id);
    setQueue((prev) => prev.slice(1));
  }

  const handleOpenChange = useCallback(
    (open: boolean) => { if (!open) respond("deny"); },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [current]
  );

  const modalClassNames = useMemo(
    () => ({ backdrop: "z-[60]", wrapper: "z-[60]" }),
    []
  );

  return (
    <Modal
      isOpen={current !== null}
      onOpenChange={handleOpenChange}
      hideCloseButton
      classNames={modalClassNames}
    >
      <ModalContent>
        {current && (
          <RuntimeApprovalContent
            current={current}
            queue={queue}
            cooldown={cooldown}
            respond={respond}
          />
        )}
      </ModalContent>
    </Modal>
  );
}

/** Inner content extracted to avoid deriving from a potentially-null `current`. */
function RuntimeApprovalContent({
  current,
  queue,
  cooldown,
  respond,
}: {
  current: RuntimeApprovalRequest;
  queue: RuntimeApprovalRequest[];
  cooldown: number;
  respond: (decision: ApprovalDecision) => void;
}) {
  const { t } = useTranslation("permissions");
  const header = resolveHeader(current);
  const HeaderIcon = header.icon;
  const isHighRisk = current.context.risk_level === "high";
  const isDeferred = current.category === "deferred_permission";
  const isExtension =
    current.category.startsWith("extension:") ||
    current.category.startsWith("extension_scope:");
  const approveDisabled = cooldown > 0;

  const handleDeny = useCallback(() => respond("deny"), [respond]);
  const handleApproveOnce = useCallback(() => respond("approve_once"), [respond]);
  const handleApprove = useCallback(() => respond("approve"), [respond]);

  return (
    <>
      {/* Header */}
      <div className="flex items-center gap-3 px-6 pt-6 pb-4">
        <div
          className={`w-10 h-10 rounded-[14px] ${header.iconBg} border border-default-100 flex items-center justify-center flex-shrink-0`}
        >
          <HeaderIcon
            size={20}
            strokeWidth={1.5}
            className={header.iconColor}
          />
        </div>
        <div className="min-w-0">
          <h3 className="text-[16px] font-bold">
            {header.title}
          </h3>
          <p className="text-[12px] text-default-500 leading-snug">
            {header.subtitle}
          </p>
        </div>
      </div>

      {/* Category-specific content */}
      <div className="px-6 pb-4">
        {isDeferred ? (
          <DeferredPermissionDetail context={current.context} permission={current.permission} />
        ) : current.category === "oauth_authorize" ? (
          <OAuthConsentDetail context={current.context} />
        ) : current.category === "filesystem" ? (
          <FilesystemDetail context={current.context} />
        ) : isExtension ? (
          <ExtensionDetail
            context={current.context}
            isHighRisk={isHighRisk}
          />
        ) : current.category === "mcp_tool" ? (
          <McpToolDetail context={current.context} />
        ) : (
          <GenericDetail context={current.context} />
        )}
      </div>

      {/* Queue indicator */}
      {queue.length > 1 && (
        <div className="px-6 pb-3">
          <p className="text-[11px] text-default-400">
            {t("runtime.requestsPending", { count: queue.length - 1 })}
          </p>
        </div>
      )}

      {/* Actions */}
      <div className="flex gap-3 justify-end px-6 pb-6">
        <Button onPress={handleDeny}>
          <ShieldX size={14} strokeWidth={1.5} />
          {t("common:action.deny")}
        </Button>
        {isHighRisk ? (
          <Button
            color="primary"
            isDisabled={approveDisabled}
            onPress={handleApproveOnce}
          >
            <ShieldCheck size={14} strokeWidth={1.5} />
            {approveDisabled ? t("runtime.allowOnceCountdown", { seconds: cooldown }) : t("runtime.allowOnce")}
          </Button>
        ) : (
          <>
            <Button
              isDisabled={approveDisabled}
              onPress={handleApproveOnce}
            >
              <ShieldCheck size={14} strokeWidth={1.5} />
              {approveDisabled ? t("runtime.allowOnceCountdown", { seconds: cooldown }) : t("runtime.allowOnce")}
            </Button>
            <Button
              color="primary"
              isDisabled={approveDisabled}
              onPress={handleApprove}
            >
              <ShieldCheck size={14} strokeWidth={1.5} />
              {approveDisabled ? t("runtime.allowCountdown", { seconds: cooldown }) : t("runtime.allow")}
            </Button>
          </>
        )}
      </div>
    </>
  );
}

function DeferredPermissionDetail({
  context,
  permission,
}: {
  context: Record<string, string>;
  permission: string;
}) {
  const { t } = useTranslation("permissions");
  const info = getPermissionInfo(permission);
  const riskChipColors: Record<string, "success" | "warning" | "danger"> = {
    low: "success",
    medium: "warning",
    high: "danger",
  };

  return (
    <div className="space-y-2">
      <div className="p-3 rounded-[8px] bg-background border border-default-100">
        <p className="text-[11px] text-default-500 mb-1.5">{t("runtime.permission")}</p>
        <div className="flex items-center gap-2">
          <p className="text-[12px] font-medium font-mono">
            {permission}
          </p>
          <Chip size="sm" variant="flat" color={riskChipColors[info.risk] ?? "warning"}>
            {info.risk}
          </Chip>
        </div>
        <p className="text-[11px] text-default-500 mt-1">
          {info.description}
        </p>
      </div>
      {context.extension && (
        <div className="p-3 rounded-[8px] bg-background border border-default-100">
          <div className="flex items-center gap-2 mb-1">
            <Puzzle size={12} strokeWidth={1.5} className="text-default-500" />
            <p className="text-[11px] text-default-500">
              {context.extension_display_name ?? context.extension}
            </p>
          </div>
          {context.operation && (
            <p className="text-[12px] font-medium">
              {context.operation}
            </p>
          )}
          {context.operation_description && (
            <p className="text-[11px] text-default-500 mt-0.5">
              {context.operation_description}
            </p>
          )}
        </div>
      )}
    </div>
  );
}

function FilesystemDetail({ context }: { context: Record<string, string> }) {
  const { t } = useTranslation("permissions");
  return (
    <div className="space-y-2">
      <div className="p-3 rounded-[8px] bg-background border border-default-100">
        <p className="text-[11px] text-default-500 mb-1">{t("runtime.requestedPath")}</p>
        <p className="text-[12px] font-mono break-all leading-relaxed">
          {context.path ?? "unknown"}
        </p>
      </div>
      {context.parent_dir && (
        <div className="p-3 rounded-[8px] bg-background border border-default-100">
          <p className="text-[11px] text-default-500 mb-1">
            {t("runtime.allowGrantsAccess")}
          </p>
          <p className="text-[12px] text-primary font-mono break-all leading-relaxed">
            {context.parent_dir}
          </p>
        </div>
      )}
    </div>
  );
}

function ExtensionDetail({
  context,
  isHighRisk,
}: {
  context: Record<string, string>;
  isHighRisk: boolean;
}) {
  const { t } = useTranslation("permissions");
  const extName =
    context.extension_display_name ?? context.extension ?? t("runtime.unknownExtension");
  const operation = context.operation ?? "unknown";
  const opDesc = context.operation_description;

  // Collect input.* fields for display
  const inputEntries = Object.entries(context)
    .filter(([k]) => k.startsWith("input."))
    .map(([k, v]) => [k.replace("input.", ""), v] as const);

  return (
    <div className="space-y-2">
      {/* Extension + operation info */}
      <div className="p-3 rounded-[8px] bg-background border border-default-100">
        <div className="flex items-center gap-2 mb-1.5">
          <Puzzle size={13} strokeWidth={1.5} className="text-default-500" />
          <p className="text-[11px] text-default-500">{extName}</p>
        </div>
        <p className="text-[13px] font-medium">{operation}</p>
        {opDesc && (
          <p className="text-[11px] text-default-500 mt-0.5">
            {opDesc}
          </p>
        )}
      </div>

      {/* Scope value (for scope approvals) */}
      {context.scope_value && (
        <div className="p-3 rounded-[8px] bg-background border border-default-100">
          <p className="text-[11px] text-default-500 mb-1">
            {context.scope_description ?? context.scope_key ?? t("runtime.scope")}
          </p>
          <p className="text-[12px] text-primary font-mono break-all leading-relaxed">
            {context.scope_value}
          </p>
        </div>
      )}

      {/* Input parameters (for high-risk, shows what's being passed) */}
      {isHighRisk && inputEntries.length > 0 && (
        <div className="p-3 rounded-[8px] bg-background border border-default-100 space-y-1.5">
          <p className="text-[11px] text-default-500">{t("runtime.parameters")}</p>
          {inputEntries.map(([key, value]) => (
            <div key={key} className="flex gap-2">
              <span className="text-[11px] text-default-400 font-mono whitespace-nowrap">
                {key}:
              </span>
              <span className="text-[12px] font-mono break-all">
                {value}
              </span>
            </div>
          ))}
        </div>
      )}

      {/* Risk badge */}
      {isHighRisk && (
        <div className="flex items-center gap-1.5 pt-1">
          <AlertTriangle size={12} strokeWidth={1.5} className="text-red-400" />
          <p className="text-[11px] text-red-400 font-medium">
            {t("runtime.approvalRequired")}
          </p>
        </div>
      )}
    </div>
  );
}

function McpToolDetail({ context }: { context: Record<string, string> }) {
  const { t } = useTranslation("permissions");
  const [showFullDesc, setShowFullDesc] = useState(false);
  const toolName = context.tool_name ?? "unknown";
  const pluginName = context.plugin_name;
  const description = context.description;

  // Collect arg.* fields
  const argEntries = Object.entries(context)
    .filter(([k]) => k.startsWith("arg."))
    .map(([k, v]) => [k.replace("arg.", ""), v] as const);

  const handleShowFullDesc = useCallback(() => setShowFullDesc(true), []);

  return (
    <div className="space-y-2 max-h-[40vh] overflow-y-auto">
      {/* Tool name + plugin */}
      <div className="p-3 rounded-[8px] bg-background border border-default-100">
        <p className="text-[11px] text-default-500 mb-1">{t("runtime.tool")}</p>
        <p className="text-[13px] font-mono font-medium">
          {toolName}
        </p>
        {pluginName && (
          <p className="text-[11px] text-default-400 mt-0.5">
            {t("runtime.via", { name: pluginName })}
          </p>
        )}
      </div>

      {/* Arguments */}
      {argEntries.length > 0 && (
        <div className="p-3 rounded-[8px] bg-background border border-default-100 space-y-1.5">
          <p className="text-[11px] text-default-500">{t("runtime.arguments")}</p>
          {argEntries.map(([key, value]) => (
            <div key={key}>
              <span className="text-[11px] text-default-400 font-mono">
                {key}
              </span>
              <p className="text-[12px] font-mono break-all leading-relaxed">
                {value}
              </p>
            </div>
          ))}
        </div>
      )}

      {/* Description — truncated to 3 lines */}
      {description && (
        <div className="p-3 rounded-[8px] bg-background border border-default-100">
          <p className="text-[11px] text-default-500 mb-1">{t("runtime.description")}</p>
          <p
            className={`text-[11px] text-default-500 leading-relaxed break-words ${
              showFullDesc ? "" : "line-clamp-3"
            }`}
          >
            {description}
          </p>
          {!showFullDesc && description.length > 150 && (
            <Button
              onPress={handleShowFullDesc}
              className="mt-1"
            >
              {t("common:action.showMore")}
            </Button>
          )}
        </div>
      )}
    </div>
  );
}

function OAuthConsentDetail({ context }: { context: Record<string, string> }) {
  const { t } = useTranslation("permissions");
  const clientName = context.client_name ?? "Unknown client";
  const clientId = context.client_id ?? "";
  const scopes = context.scopes ?? "mcp";

  return (
    <div className="space-y-2">
      <div className="p-3 rounded-[8px] bg-background border border-default-100">
        <p className="text-[11px] text-default-500 mb-1">{t("runtime.oauthClient")}</p>
        <p className="text-[13px] font-medium">{clientName}</p>
        {clientId && (
          <p className="text-[11px] text-default-400 font-mono mt-0.5">
            {clientId.slice(0, 8)}...
          </p>
        )}
      </div>
      <div className="p-3 rounded-[8px] bg-background border border-default-100">
        <p className="text-[11px] text-default-500 mb-1">{t("runtime.oauthAccess")}</p>
        <p className="text-[12px]">{scopes}</p>
      </div>
    </div>
  );
}

function GenericDetail({ context }: { context: Record<string, string> }) {
  const entries = Object.entries(context).filter(([k]) => k !== "permission");

  if (entries.length === 0) return null;

  return (
    <div className="p-3 rounded-[8px] bg-background border border-default-100 space-y-1.5 max-h-[40vh] overflow-y-auto">
      {entries.map(([key, value]) => (
        <div key={key}>
          <p className="text-[11px] text-default-500">{key}</p>
          <p className="text-[12px] font-mono break-all">
            {value}
          </p>
        </div>
      ))}
    </div>
  );
}
