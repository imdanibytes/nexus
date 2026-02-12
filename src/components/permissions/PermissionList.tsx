import { useEffect } from "react";
import { usePermissions } from "../../hooks/usePermissions";
import { PERMISSION_INFO } from "../../types/permissions";
import type { Permission } from "../../types/permissions";

interface Props {
  pluginId: string;
}

export function PermissionList({ pluginId }: Props) {
  const { grants, loadGrants, revoke } = usePermissions();

  useEffect(() => {
    loadGrants(pluginId);
  }, [pluginId, loadGrants]);

  if (grants.length === 0) {
    return (
      <p className="text-[11px] text-nx-text-ghost">No permissions granted</p>
    );
  }

  return (
    <div className="space-y-1.5">
      {grants.map((grant) => {
        const info = PERMISSION_INFO[grant.permission as Permission];
        return (
          <div
            key={grant.permission}
            className="flex items-center justify-between p-2.5 rounded-[var(--radius-button)] bg-nx-deep border border-nx-border-subtle"
          >
            <div>
              <p className="text-[12px] text-nx-text font-medium font-mono">
                {grant.permission}
              </p>
              <p className="text-[11px] text-nx-text-muted mt-0.5">
                {info?.description ?? "Unknown permission"}
              </p>
            </div>
            <button
              onClick={() =>
                revoke(pluginId, [grant.permission as Permission])
              }
              className="text-[11px] font-medium px-2 py-1 rounded-[var(--radius-tag)] bg-nx-error-muted text-nx-error hover:bg-nx-error/20 transition-colors duration-150"
            >
              Revoke
            </button>
          </div>
        );
      })}
    </div>
  );
}
