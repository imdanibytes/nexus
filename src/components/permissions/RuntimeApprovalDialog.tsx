import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { ShieldCheck, ShieldAlert, ShieldX, FolderOpen } from "lucide-react";
import { runtimeApprovalRespond } from "../../lib/tauri";
import type { ApprovalDecision, RuntimeApprovalRequest } from "../../types/permissions";

const categoryLabels: Record<string, string> = {
  filesystem: "File Access",
  network: "Network Access",
};

const categoryIcons: Record<string, typeof FolderOpen> = {
  filesystem: FolderOpen,
};

export function RuntimeApprovalDialog() {
  const [queue, setQueue] = useState<RuntimeApprovalRequest[]>([]);

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

  if (queue.length === 0) return null;

  const current = queue[0];

  async function respond(decision: ApprovalDecision) {
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
    setQueue((prev) => prev.slice(1));
  }

  const CategoryIcon = categoryIcons[current.category] ?? ShieldAlert;
  const label = categoryLabels[current.category] ?? current.category;

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
          <div className="w-10 h-10 rounded-[var(--radius-card)] bg-nx-warning-muted border border-nx-border-subtle flex items-center justify-center flex-shrink-0">
            <CategoryIcon size={20} strokeWidth={1.5} className="text-nx-warning" />
          </div>
          <div className="min-w-0">
            <h3 className="text-[16px] font-bold text-nx-text">{label} Request</h3>
            <p className="text-[12px] text-nx-text-muted truncate">
              {current.plugin_name}
              <span className="text-nx-text-ghost"> wants {current.permission.replace(":", " ")}</span>
            </p>
          </div>
        </div>

        {/* Category-specific content */}
        <div className="px-6 pb-4">
          {current.category === "filesystem" ? (
            <FilesystemDetail context={current.context} />
          ) : (
            <GenericDetail context={current.context} />
          )}
        </div>

        {/* Queue indicator */}
        {queue.length > 1 && (
          <div className="px-6 pb-3">
            <p className="text-[11px] text-nx-text-ghost">
              +{queue.length - 1} more {queue.length - 1 === 1 ? "request" : "requests"} pending
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
          <button
            onClick={() => respond("approve_once")}
            className="flex items-center gap-1.5 px-4 py-2 text-[13px] font-medium rounded-[var(--radius-button)] bg-nx-overlay hover:bg-nx-wash text-nx-text transition-all duration-150"
          >
            <ShieldCheck size={14} strokeWidth={1.5} />
            Allow Once
          </button>
          <button
            onClick={() => respond("approve")}
            className="flex items-center gap-1.5 px-4 py-2 text-[13px] font-medium rounded-[var(--radius-button)] bg-nx-accent hover:bg-nx-accent-hover text-nx-deep transition-all duration-150"
          >
            <ShieldCheck size={14} strokeWidth={1.5} />
            Allow
          </button>
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

function GenericDetail({ context }: { context: Record<string, string> }) {
  const entries = Object.entries(context).filter(([k]) => k !== "permission");

  if (entries.length === 0) return null;

  return (
    <div className="p-3 rounded-[var(--radius-button)] bg-nx-deep border border-nx-border-subtle space-y-1.5">
      {entries.map(([key, value]) => (
        <div key={key}>
          <p className="text-[11px] text-nx-text-muted">{key}</p>
          <p className="text-[12px] text-nx-text font-mono break-all">{value}</p>
        </div>
      ))}
    </div>
  );
}
