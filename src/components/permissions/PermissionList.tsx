import { useEffect, useState } from "react";
import { usePermissions } from "../../hooks/usePermissions";
import { PERMISSION_INFO } from "../../types/permissions";
import type { Permission } from "../../types/permissions";
import { ChevronDown, FolderOpen, X } from "lucide-react";

interface Props {
  pluginId: string;
}

export function PermissionList({ pluginId }: Props) {
  const { grants, loadGrants, revoke, removePath } = usePermissions();
  const [expandedPerms, setExpandedPerms] = useState<Set<string>>(new Set());

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

  const FS_PERMISSIONS = ["filesystem:read", "filesystem:write"];

  return (
    <div className="space-y-1.5">
      {grants.map((grant) => {
        const info = PERMISSION_INFO[grant.permission as Permission];
        const isFs = FS_PERMISSIONS.includes(grant.permission);
        const hasPaths =
          isFs &&
          grant.approved_paths !== null &&
          grant.approved_paths !== undefined;
        const paths = grant.approved_paths ?? [];
        const isExpanded = expandedPerms.has(grant.permission);

        return (
          <div
            key={grant.permission}
            className="rounded-[var(--radius-button)] border border-nx-border-subtle bg-nx-deep overflow-hidden"
          >
            {/* Permission row */}
            <div
              onClick={hasPaths ? () => togglePerm(grant.permission) : undefined}
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
                    {isFs && grant.approved_paths === null && (
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
                  revoke(pluginId, [grant.permission as Permission]);
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
                            removePath(
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
      })}
    </div>
  );
}
