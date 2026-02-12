import type { Permission } from "../../types/permissions";
import { PERMISSION_INFO } from "../../types/permissions";
import { ShieldCheck, ShieldX } from "lucide-react";

const riskColors = {
  low: "text-nx-success bg-nx-success-muted",
  medium: "text-nx-warning bg-nx-warning-muted",
  high: "text-nx-error bg-nx-error-muted",
};

interface Props {
  pluginName: string;
  requestedPermissions: Permission[];
  onApprove: (permissions: Permission[]) => void;
  onDeny: () => void;
}

export function PermissionDialog({
  pluginName,
  requestedPermissions,
  onApprove,
  onDeny,
}: Props) {
  const perms =
    requestedPermissions.length > 0
      ? requestedPermissions
      : (Object.keys(PERMISSION_INFO) as Permission[]).slice(0, 1);

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      <div className="absolute inset-0 bg-black/50 backdrop-blur-sm" onClick={onDeny} />
      <div
        className="relative bg-nx-surface border border-nx-border rounded-[var(--radius-modal)] shadow-[var(--shadow-modal)] max-w-md w-full mx-4 p-6"
        style={{ animation: "toast-enter 200ms ease-out" }}
      >
        <h3 className="text-[18px] font-bold text-nx-text mb-1">
          Install {pluginName}?
        </h3>
        <p className="text-[13px] text-nx-text-secondary mb-5">
          This plugin requests the following permissions:
        </p>

        <div className="space-y-2 mb-6">
          {perms.map((perm) => {
            const info = PERMISSION_INFO[perm];
            return (
              <div
                key={perm}
                className="flex items-center justify-between p-3 rounded-[var(--radius-button)] bg-nx-deep border border-nx-border-subtle"
              >
                <div>
                  <p className="text-[12px] text-nx-text font-medium font-mono">{perm}</p>
                  <p className="text-[11px] text-nx-text-muted mt-0.5">{info.description}</p>
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

        <div className="flex gap-3 justify-end">
          <button
            onClick={onDeny}
            className="flex items-center gap-1.5 px-4 py-2 text-[13px] font-medium rounded-[var(--radius-button)] bg-nx-overlay hover:bg-nx-wash text-nx-text-secondary transition-all duration-150"
          >
            <ShieldX size={14} strokeWidth={1.5} />
            Deny
          </button>
          <button
            onClick={() => onApprove(perms)}
            className="flex items-center gap-1.5 px-4 py-2 text-[13px] font-medium rounded-[var(--radius-button)] bg-nx-accent hover:bg-nx-accent-hover text-nx-deep transition-all duration-150"
          >
            <ShieldCheck size={14} strokeWidth={1.5} />
            Approve & Install
          </button>
        </div>
      </div>
    </div>
  );
}
