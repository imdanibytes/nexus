import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import {
  ShieldCheck,
  ShieldAlert,
  ShieldX,
  FolderOpen,
  Puzzle,
  AlertTriangle,
} from "lucide-react";
import { runtimeApprovalRespond } from "../../lib/tauri";
import type {
  ApprovalDecision,
  RuntimeApprovalRequest,
} from "../../types/permissions";

/** Derive a human-readable header from the approval category. */
function resolveHeader(req: RuntimeApprovalRequest): {
  icon: typeof FolderOpen;
  title: string;
  subtitle: string;
  iconBg: string;
  iconColor: string;
} {
  const ctx = req.context;

  // Extension high-risk operation
  if (req.category.startsWith("extension:")) {
    const extName = ctx.extension_display_name ?? ctx.extension ?? req.category;
    const opDesc = ctx.operation_description ?? ctx.operation ?? "an operation";
    return {
      icon: AlertTriangle,
      title: "High-Risk Operation",
      subtitle: `${req.plugin_name} wants to run "${opDesc}" via ${extName}`,
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
      title: `${extName} Access`,
      subtitle: scopeVal
        ? `${req.plugin_name} wants ${opName} access to ${scopeDesc}: ${scopeVal}`
        : `${req.plugin_name} wants ${opName} access`,
      iconBg: "bg-nx-warning-muted",
      iconColor: "text-nx-warning",
    };
  }

  // Filesystem
  if (req.category === "filesystem") {
    return {
      icon: FolderOpen,
      title: "File Access",
      subtitle: `${req.plugin_name} wants ${req.permission.replace(":", " ")}`,
      iconBg: "bg-nx-warning-muted",
      iconColor: "text-nx-warning",
    };
  }

  // Network
  if (req.category === "network") {
    return {
      icon: ShieldAlert,
      title: "Network Access",
      subtitle: `${req.plugin_name} wants ${req.permission.replace(":", " ")}`,
      iconBg: "bg-nx-warning-muted",
      iconColor: "text-nx-warning",
    };
  }

  // Fallback
  return {
    icon: ShieldAlert,
    title: `${req.category} Request`,
    subtitle: `${req.plugin_name} wants ${req.permission.replace(":", " ")}`,
    iconBg: "bg-nx-warning-muted",
    iconColor: "text-nx-warning",
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

  useEffect(() => {
    const unlisten = listen<RuntimeApprovalRequest>(
      "nexus://runtime-approval",
      (event) => {
        setQueue((prev) => [...prev, event.payload]);
      }
    );
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const current = queue.length > 0 ? queue[0] : null;

  // Reset cooldown whenever the active request changes
  useEffect(() => {
    if (!current) return;
    const duration = cooldownFor(current.context.risk_level);
    setCooldown(duration);
    if (duration === 0) return;

    const interval = setInterval(() => {
      setCooldown((prev) => {
        if (prev <= 1) {
          clearInterval(interval);
          return 0;
        }
        return prev - 1;
      });
    }, 1000);

    return () => clearInterval(interval);
  }, [current?.id]);

  if (!current) return null;

  async function respond(decision: ApprovalDecision) {
    await runtimeApprovalRespond(
      current!.id,
      decision,
      current!.plugin_id,
      current!.category,
      {
        ...current!.context,
        permission: current!.permission,
      }
    );
    setQueue((prev) => prev.slice(1));
  }

  const header = resolveHeader(current);
  const HeaderIcon = header.icon;
  const isHighRisk = current.context.risk_level === "high";
  const isExtension =
    current.category.startsWith("extension:") ||
    current.category.startsWith("extension_scope:");
  const approveDisabled = cooldown > 0;

  return (
    <div className="fixed inset-0 z-[60] flex items-center justify-center">
      <div
        className="absolute inset-0 bg-black/50 backdrop-blur-sm"
        onClick={() => respond("deny")}
      />
      <div
        className="relative bg-nx-surface border border-nx-border rounded-[var(--radius-modal)] shadow-[var(--shadow-modal)] max-w-md w-full mx-4 overflow-hidden"
        style={{ animation: "toast-enter 200ms ease-out" }}
      >
        {/* Header */}
        <div className="flex items-center gap-3 px-6 pt-6 pb-4">
          <div
            className={`w-10 h-10 rounded-[var(--radius-card)] ${header.iconBg} border border-nx-border-subtle flex items-center justify-center flex-shrink-0`}
          >
            <HeaderIcon
              size={20}
              strokeWidth={1.5}
              className={header.iconColor}
            />
          </div>
          <div className="min-w-0">
            <h3 className="text-[16px] font-bold text-nx-text">
              {header.title}
            </h3>
            <p className="text-[12px] text-nx-text-muted leading-snug">
              {header.subtitle}
            </p>
          </div>
        </div>

        {/* Category-specific content */}
        <div className="px-6 pb-4">
          {current.category === "filesystem" ? (
            <FilesystemDetail context={current.context} />
          ) : isExtension ? (
            <ExtensionDetail
              context={current.context}
              isHighRisk={isHighRisk}
            />
          ) : (
            <GenericDetail context={current.context} />
          )}
        </div>

        {/* Queue indicator */}
        {queue.length > 1 && (
          <div className="px-6 pb-3">
            <p className="text-[11px] text-nx-text-ghost">
              +{queue.length - 1} more{" "}
              {queue.length - 1 === 1 ? "request" : "requests"} pending
            </p>
          </div>
        )}

        {/* Actions */}
        <div className="flex gap-3 justify-end px-6 pb-6">
          <button
            onClick={() => respond("deny")}
            className="flex items-center gap-1.5 px-4 py-2 text-[13px] font-medium rounded-[var(--radius-button)] bg-nx-overlay hover:bg-nx-wash text-nx-text-secondary transition-all duration-150"
          >
            <ShieldX size={14} strokeWidth={1.5} />
            Deny
          </button>
          {isHighRisk ? (
            <button
              disabled={approveDisabled}
              onClick={() => respond("approve_once")}
              className={`flex items-center gap-1.5 px-4 py-2 text-[13px] font-medium rounded-[var(--radius-button)] transition-all duration-150 ${
                approveDisabled
                  ? "bg-nx-overlay text-nx-text-ghost cursor-not-allowed"
                  : "bg-nx-accent hover:bg-nx-accent-hover text-nx-deep"
              }`}
            >
              <ShieldCheck size={14} strokeWidth={1.5} />
              {approveDisabled ? `Allow Once (${cooldown}s)` : "Allow Once"}
            </button>
          ) : (
            <>
              <button
                disabled={approveDisabled}
                onClick={() => respond("approve_once")}
                className={`flex items-center gap-1.5 px-4 py-2 text-[13px] font-medium rounded-[var(--radius-button)] transition-all duration-150 ${
                  approveDisabled
                    ? "bg-nx-overlay text-nx-text-ghost cursor-not-allowed"
                    : "bg-nx-overlay hover:bg-nx-wash text-nx-text"
                }`}
              >
                <ShieldCheck size={14} strokeWidth={1.5} />
                {approveDisabled ? `Allow Once (${cooldown}s)` : "Allow Once"}
              </button>
              <button
                disabled={approveDisabled}
                onClick={() => respond("approve")}
                className={`flex items-center gap-1.5 px-4 py-2 text-[13px] font-medium rounded-[var(--radius-button)] transition-all duration-150 ${
                  approveDisabled
                    ? "bg-nx-overlay text-nx-text-ghost cursor-not-allowed"
                    : "bg-nx-accent hover:bg-nx-accent-hover text-nx-deep"
                }`}
              >
                <ShieldCheck size={14} strokeWidth={1.5} />
                {approveDisabled ? `Allow (${cooldown}s)` : "Allow"}
              </button>
            </>
          )}
        </div>
      </div>
    </div>
  );
}

function FilesystemDetail({ context }: { context: Record<string, string> }) {
  return (
    <div className="space-y-2">
      <div className="p-3 rounded-[var(--radius-button)] bg-nx-deep border border-nx-border-subtle">
        <p className="text-[11px] text-nx-text-muted mb-1">Requested path</p>
        <p className="text-[12px] text-nx-text font-mono break-all leading-relaxed">
          {context.path ?? "unknown"}
        </p>
      </div>
      {context.parent_dir && (
        <div className="p-3 rounded-[var(--radius-button)] bg-nx-deep border border-nx-border-subtle">
          <p className="text-[11px] text-nx-text-muted mb-1">
            "Allow" grants access to this directory
          </p>
          <p className="text-[12px] text-nx-accent font-mono break-all leading-relaxed">
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
  const extName =
    context.extension_display_name ?? context.extension ?? "Unknown extension";
  const operation = context.operation ?? "unknown";
  const opDesc = context.operation_description;

  // Collect input.* fields for display
  const inputEntries = Object.entries(context)
    .filter(([k]) => k.startsWith("input."))
    .map(([k, v]) => [k.replace("input.", ""), v] as const);

  return (
    <div className="space-y-2">
      {/* Extension + operation info */}
      <div className="p-3 rounded-[var(--radius-button)] bg-nx-deep border border-nx-border-subtle">
        <div className="flex items-center gap-2 mb-1.5">
          <Puzzle size={13} strokeWidth={1.5} className="text-nx-text-muted" />
          <p className="text-[11px] text-nx-text-muted">{extName}</p>
        </div>
        <p className="text-[13px] text-nx-text font-medium">{operation}</p>
        {opDesc && (
          <p className="text-[11px] text-nx-text-secondary mt-0.5">
            {opDesc}
          </p>
        )}
      </div>

      {/* Scope value (for scope approvals) */}
      {context.scope_value && (
        <div className="p-3 rounded-[var(--radius-button)] bg-nx-deep border border-nx-border-subtle">
          <p className="text-[11px] text-nx-text-muted mb-1">
            {context.scope_description ?? context.scope_key ?? "Scope"}
          </p>
          <p className="text-[12px] text-nx-accent font-mono break-all leading-relaxed">
            {context.scope_value}
          </p>
        </div>
      )}

      {/* Input parameters (for high-risk, shows what's being passed) */}
      {isHighRisk && inputEntries.length > 0 && (
        <div className="p-3 rounded-[var(--radius-button)] bg-nx-deep border border-nx-border-subtle space-y-1.5">
          <p className="text-[11px] text-nx-text-muted">Parameters</p>
          {inputEntries.map(([key, value]) => (
            <div key={key} className="flex gap-2">
              <span className="text-[11px] text-nx-text-ghost font-mono whitespace-nowrap">
                {key}:
              </span>
              <span className="text-[12px] text-nx-text font-mono break-all">
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
            This operation requires approval every time it runs
          </p>
        </div>
      )}
    </div>
  );
}

function GenericDetail({ context }: { context: Record<string, string> }) {
  const entries = Object.entries(context).filter(([k]) => k !== "permission");

  if (entries.length === 0) return null;

  return (
    <div className="p-3 rounded-[var(--radius-button)] bg-nx-deep border border-nx-border-subtle space-y-1.5">
      {entries.map(([key, value]) => (
        <div key={key}>
          <p className="text-[11px] text-nx-text-muted">{key}</p>
          <p className="text-[12px] text-nx-text font-mono break-all">
            {value}
          </p>
        </div>
      ))}
    </div>
  );
}
