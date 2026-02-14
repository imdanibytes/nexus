import { useEffect, useState } from "react";
import { usePermissions } from "../../hooks/usePermissions";
import { getPermissionInfo } from "../../types/permissions";
import type { Permission, GrantedPermission } from "../../types/permissions";
import { ChevronDown, FolderOpen, RotateCcw, X, ShieldCheck, Clock } from "lucide-react";

interface Props {
  pluginId: string;
}

export function PermissionList({ pluginId }: Props) {
  const { grants, loadGrants, revoke, unrevoke, removePath } = usePermissions();
  const [expandedPerms, setExpandedPerms] = useState<Set<string>>(new Set());
  const [confirmRestore, setConfirmRestore] = useState<string | null>(null);

  useEffect(() => {
    loadGrants(pluginId);
  }, [pluginId, loadGrants]);

  function togglePerm(perm: string) {
    setExpandedPerms((prev) => {
      const next = new Set(prev);
      if (next.has(perm)) next.delete(perm);
      else next.add(perm);
      return next;
    });
  }

  if (grants.length === 0) {
    return (
      <p className="text-[11px] text-nx-text-ghost">No permissions granted</p>
    );
  }

  const activeGrants = grants.filter((g) => g.state === "active");
  const deferredGrants = grants.filter((g) => g.state === "deferred");
  const revokedGrants = grants.filter((g) => g.state === "revoked");

  const FS_PERMISSIONS = ["filesystem:read", "filesystem:write"];

  return (
    <div className="space-y-1.5">
      {/* Active permissions */}
      {activeGrants.map((grant) => (
        <ActivePermissionRow
          key={grant.permission}
          grant={grant}
          pluginId={pluginId}
          fsPermissions={FS_PERMISSIONS}
          expandedPerms={expandedPerms}
          onToggle={togglePerm}
          onRevoke={revoke}
          onRemovePath={removePath}
        />
      ))}

      {/* Deferred permissions */}
      {deferredGrants.length > 0 && (
        <>
          {activeGrants.length > 0 && (
            <div className="flex items-center gap-2 pt-2 pb-0.5">
              <div className="flex-1 h-px bg-nx-border-subtle" />
              <span className="text-[10px] text-nx-warning font-medium uppercase tracking-wide">
                Deferred
              </span>
              <div className="flex-1 h-px bg-nx-border-subtle" />
            </div>
          )}

          {deferredGrants.map((grant) => {
            const info = getPermissionInfo(grant.permission);

            return (
              <div
                key={`deferred-${grant.permission}`}
                className="rounded-[var(--radius-button)] border border-nx-warning/20 bg-nx-deep overflow-hidden"
              >
                <div className="flex items-center justify-between p-2.5">
                  <div className="min-w-0">
                    <div className="flex items-center gap-2">
                      <p className="text-[12px] text-nx-text font-medium font-mono">
                        {grant.permission}
                      </p>
                      <span className="flex items-center gap-1 text-[10px] text-nx-warning font-medium px-1.5 py-0.5 rounded-[var(--radius-tag)] bg-nx-warning-muted flex-shrink-0">
                        <Clock size={10} strokeWidth={1.5} />
                        DEFERRED
                      </span>
                    </div>
                    <p className="text-[11px] text-nx-text-muted mt-0.5">
                      {info?.description ?? "Unknown permission"}
                    </p>
                    <p className="text-[10px] text-nx-text-ghost mt-0.5">
                      Will prompt for approval on first use
                    </p>
                  </div>
                  <div className="flex gap-1.5 flex-shrink-0 ml-2">
                    <button
                      onClick={() => unrevoke(pluginId, [grant.permission as Permission])}
                      className="flex items-center gap-1 text-[11px] font-medium px-2 py-1 rounded-[var(--radius-tag)] bg-nx-accent-muted text-nx-accent hover:bg-nx-accent/20 transition-colors duration-150"
                      title="Activate this permission now"
                    >
                      <ShieldCheck size={11} strokeWidth={1.5} />
                      Activate
                    </button>
                    <button
                      onClick={() => revoke(pluginId, [grant.permission as Permission])}
                      className="text-[11px] font-medium px-2 py-1 rounded-[var(--radius-tag)] bg-nx-error-muted text-nx-error hover:bg-nx-error/20 transition-colors duration-150"
                    >
                      Revoke
                    </button>
                  </div>
                </div>
              </div>
            );
          })}
        </>
      )}

      {/* Revoked permissions */}
      {revokedGrants.length > 0 && (
        <>
          {(activeGrants.length > 0 || deferredGrants.length > 0) && (
            <div className="flex items-center gap-2 pt-2 pb-0.5">
              <div className="flex-1 h-px bg-nx-border-subtle" />
              <span className="text-[10px] text-nx-text-ghost font-medium uppercase tracking-wide">
                Revoked
              </span>
              <div className="flex-1 h-px bg-nx-border-subtle" />
            </div>
          )}

          {revokedGrants.map((grant) => {
            const info = getPermissionInfo(grant.permission);
            const scopeCount = grant.approved_scopes?.length ?? 0;

            return (
              <div
                key={`revoked-${grant.permission}`}
                className="rounded-[var(--radius-button)] border border-nx-border-subtle bg-nx-deep overflow-hidden opacity-60"
              >
                <div className="flex items-center justify-between p-2.5">
                  <div className="min-w-0">
                    <div className="flex items-center gap-2">
                      <p className="text-[12px] text-nx-text-muted font-medium font-mono line-through">
                        {grant.permission}
                      </p>
                      <span className="text-[10px] text-nx-error font-medium px-1.5 py-0.5 rounded-[var(--radius-tag)] bg-nx-error-muted flex-shrink-0">
                        REVOKED
                      </span>
                      {scopeCount > 0 && (
                        <span className="text-[10px] text-nx-text-ghost font-mono flex-shrink-0">
                          {scopeCount} saved scope{scopeCount !== 1 ? "s" : ""}
                        </span>
                      )}
                    </div>
                    <p className="text-[11px] text-nx-text-ghost mt-0.5">
                      {info?.description ?? "Unknown permission"}
                    </p>
                  </div>
                  <button
                    onClick={() => setConfirmRestore(grant.permission)}
                    className="flex items-center gap-1 text-[11px] font-medium px-2 py-1 rounded-[var(--radius-tag)] bg-nx-accent-muted text-nx-accent hover:bg-nx-accent/20 transition-colors duration-150 flex-shrink-0 ml-2"
                  >
                    <RotateCcw size={11} strokeWidth={1.5} />
                    Restore
                  </button>
                </div>
              </div>
            );
          })}
        </>
      )}

      {/* Confirm restore modal */}
      {confirmRestore && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm">
          <div className="bg-nx-surface border border-nx-border rounded-[var(--radius-card)] p-5 max-w-sm w-full mx-4 shadow-xl">
            <h3 className="text-[14px] font-semibold text-nx-text mb-2">
              Restore Permission
            </h3>
            <p className="text-[12px] text-nx-text-muted mb-1">
              Restore <span className="font-mono font-medium text-nx-text">{confirmRestore}</span> for this plugin?
            </p>
            <p className="text-[11px] text-nx-text-ghost mb-4">
              Previously approved scopes will be preserved. The plugin will regain access immediately.
            </p>
            <div className="flex gap-2 justify-end">
              <button
                onClick={() => setConfirmRestore(null)}
                className="px-3 py-1.5 text-[12px] font-medium text-nx-text-muted bg-nx-wash border border-nx-border-subtle rounded-[var(--radius-button)] hover:bg-nx-base transition-colors duration-150"
              >
                Cancel
              </button>
              <button
                onClick={() => {
                  unrevoke(pluginId, [confirmRestore as Permission]);
                  setConfirmRestore(null);
                }}
                className="px-3 py-1.5 text-[12px] font-medium text-nx-text bg-nx-accent-muted border border-nx-accent/30 rounded-[var(--radius-button)] hover:bg-nx-accent/20 transition-colors duration-150"
              >
                Restore
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

/** Active permission row with scope expansion support. */
function ActivePermissionRow({
  grant,
  pluginId,
  fsPermissions,
  expandedPerms,
  onToggle,
  onRevoke,
  onRemovePath,
}: {
  grant: GrantedPermission;
  pluginId: string;
  fsPermissions: string[];
  expandedPerms: Set<string>;
  onToggle: (perm: string) => void;
  onRevoke: (pluginId: string, permissions: Permission[]) => void;
  onRemovePath: (pluginId: string, permission: Permission, path: string) => void;
}) {
  const info = getPermissionInfo(grant.permission);
  const isFs = fsPermissions.includes(grant.permission);
  const hasPaths =
    isFs &&
    grant.approved_scopes !== null &&
    grant.approved_scopes !== undefined;
  const paths = grant.approved_scopes ?? [];
  const isExpanded = expandedPerms.has(grant.permission);

  return (
    <div className="rounded-[var(--radius-button)] border border-nx-border-subtle bg-nx-deep overflow-hidden">
      {/* Permission row */}
      <div
        onClick={hasPaths ? () => onToggle(grant.permission) : undefined}
        className={`flex items-center justify-between p-2.5 ${hasPaths ? "cursor-pointer hover:bg-nx-wash/30 transition-colors duration-150" : ""}`}
      >
        <div className="flex items-center gap-2 min-w-0">
          {hasPaths && (
            <ChevronDown
              size={14}
              strokeWidth={1.5}
              className={`text-nx-text-muted flex-shrink-0 transition-transform duration-200 ${
                isExpanded ? "rotate-0" : "-rotate-90"
              }`}
            />
          )}
          <div className="min-w-0">
            <div className="flex items-center gap-2">
              <p className="text-[12px] text-nx-text font-medium font-mono">
                {grant.permission}
              </p>
              {hasPaths && (
                <span className="text-[10px] text-nx-text-ghost font-mono flex-shrink-0">
                  {paths.length === 0
                    ? "no paths approved"
                    : `${paths.length} path${paths.length !== 1 ? "s" : ""}`}
                </span>
              )}
              {isFs && grant.approved_scopes === null && (
                <span className="text-[10px] text-nx-warning font-medium px-1.5 py-0.5 rounded-[var(--radius-tag)] bg-nx-warning-muted flex-shrink-0">
                  UNRESTRICTED
                </span>
              )}
            </div>
            <p className="text-[11px] text-nx-text-muted mt-0.5">
              {info?.description ?? "Unknown permission"}
            </p>
          </div>
        </div>
        <button
          onClick={(e) => {
            e.stopPropagation();
            onRevoke(pluginId, [grant.permission as Permission]);
          }}
          className="text-[11px] font-medium px-2 py-1 rounded-[var(--radius-tag)] bg-nx-error-muted text-nx-error hover:bg-nx-error/20 transition-colors duration-150 flex-shrink-0 ml-2"
        >
          Revoke
        </button>
      </div>

      {/* Approved paths (expanded) */}
      {hasPaths && isExpanded && (
        <div className="px-2.5 pb-2.5 border-t border-nx-border-subtle">
          {paths.length === 0 ? (
            <p className="text-[11px] text-nx-text-ghost pt-2">
              No directories approved yet. Access will be prompted at
              runtime.
            </p>
          ) : (
            <div className="pt-2 space-y-1">
              {paths.map((p) => (
                <div
                  key={p}
                  className="flex items-center justify-between gap-2 px-2 py-1.5 rounded-[var(--radius-tag)] bg-nx-base"
                >
                  <div className="flex items-center gap-2 min-w-0">
                    <FolderOpen
                      size={12}
                      strokeWidth={1.5}
                      className="text-nx-accent flex-shrink-0"
                    />
                    <span className="text-[11px] font-mono text-nx-text truncate">
                      {p}
                    </span>
                  </div>
                  <button
                    onClick={() =>
                      onRemovePath(
                        pluginId,
                        grant.permission as Permission,
                        p
                      )
                    }
                    title={`Revoke access to ${p}`}
                    className="text-nx-text-ghost hover:text-nx-error transition-colors duration-150 flex-shrink-0 p-1.5 -m-1.5"
                  >
                    <X size={12} strokeWidth={1.5} />
                  </button>
                </div>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
