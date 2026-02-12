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
      <p className="text-xs text-slate-500">No permissions granted</p>
    );
  }

  return (
    <div className="space-y-2">
      {grants.map((grant) => {
        const info = PERMISSION_INFO[grant.permission as Permission];
        return (
          <div
            key={grant.permission}
            className="flex items-center justify-between p-2 rounded-lg bg-slate-900"
          >
            <div>
              <p className="text-sm text-slate-200">{grant.permission}</p>
              <p className="text-xs text-slate-400">
                {info?.description ?? "Unknown permission"}
              </p>
            </div>
            <button
              onClick={() =>
                revoke(pluginId, [grant.permission as Permission])
              }
              className="text-xs px-2 py-1 rounded bg-red-500/20 text-red-400 hover:bg-red-500/30 transition-colors"
            >
              Revoke
            </button>
          </div>
        );
      })}
    </div>
  );
}
