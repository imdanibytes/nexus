import type { Permission } from "../../types/permissions";
import { PERMISSION_INFO } from "../../types/permissions";

const riskColors = {
  low: "text-green-400 bg-green-500/10",
  medium: "text-yellow-400 bg-yellow-500/10",
  high: "text-red-400 bg-red-500/10",
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
      <div className="absolute inset-0 bg-black/60" onClick={onDeny} />
      <div className="relative bg-slate-800 border border-slate-700 rounded-xl shadow-2xl max-w-md w-full mx-4 p-6">
        <h3 className="text-lg font-bold text-white mb-1">
          Install {pluginName}?
        </h3>
        <p className="text-sm text-slate-400 mb-4">
          This plugin requests the following permissions:
        </p>

        <div className="space-y-2 mb-6">
          {perms.map((perm) => {
            const info = PERMISSION_INFO[perm];
            return (
              <div
                key={perm}
                className="flex items-center justify-between p-3 rounded-lg bg-slate-900"
              >
                <div>
                  <p className="text-sm text-slate-200">{perm}</p>
                  <p className="text-xs text-slate-400">{info.description}</p>
                </div>
                <span
                  className={`text-xs px-2 py-0.5 rounded-full font-medium capitalize ${riskColors[info.risk]}`}
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
            className="px-4 py-2 text-sm rounded-lg bg-slate-700 hover:bg-slate-600 text-slate-300 transition-colors"
          >
            Deny
          </button>
          <button
            onClick={() => onApprove(perms)}
            className="px-4 py-2 text-sm rounded-lg bg-indigo-500 hover:bg-indigo-600 text-white transition-colors"
          >
            Approve & Install
          </button>
        </div>
      </div>
    </div>
  );
}
