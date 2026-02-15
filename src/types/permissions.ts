import i18n from "../i18n";

/** Well-known built-in permissions. */
export type BuiltinPermission =
  | "system:info"
  | "filesystem:read"
  | "filesystem:write"
  | "process:list"
  | "docker:read"
  | "docker:manage"
  | "network:local"
  | "network:internet";

/** A permission string — either a built-in scope or an extension scope (ext:*). */
export type Permission = BuiltinPermission | (string & {});

export type PermissionState = "active" | "revoked" | "deferred";

export interface GrantedPermission {
  plugin_id: string;
  permission: Permission;
  granted_at: string;
  /** Generalized scope whitelist (renamed from approved_paths). */
  approved_scopes: string[] | null;
  /** Three-state lifecycle: active, revoked, or deferred. Source of truth. */
  state: PermissionState;
  /** Legacy timestamp preserved for revoked state. `state` is the source of truth. */
  revoked_at: string | null;
}

export type ApprovalDecision = "approve" | "approve_once" | "deny";

export interface RuntimeApprovalRequest {
  id: string;
  plugin_id: string;
  plugin_name: string;
  category: string;
  permission: string;
  context: Record<string, string>;
}

export interface PermissionMeta {
  description: string;
  risk: "low" | "medium" | "high";
}

/** Look up permission metadata. Supports both built-in and ext:* permissions. */
export function getPermissionInfo(perm: string): PermissionMeta {
  const risk = PERMISSION_RISK[perm as BuiltinPermission];
  if (risk) {
    const key = perm.replace(/:/g, "_");
    return {
      description: i18n.t(`permissions:meta.${key}`, { defaultValue: perm }),
      risk,
    };
  }
  // Extension permissions: ext:{ext_id}:{operation}
  if (perm.startsWith("ext:")) {
    const parts = perm.slice(4).split(":");
    const extId = parts[0] || "unknown";
    const op = parts[1] || "unknown";
    return {
      description: i18n.t("permissions:meta.extensionPerm", { extId, operation: op.replace(/_/g, " ") }),
      risk: "medium",
    };
  }
  return { description: perm, risk: "medium" };
}

/** Compute the full permission list from a manifest (mirrors Rust all_permissions). */
export function allPermissions(manifest: { permissions: string[]; extensions?: Record<string, string[]> }): string[] {
  const perms = [...manifest.permissions];
  if (manifest.extensions) {
    for (const [extId, operations] of Object.entries(manifest.extensions)) {
      for (const op of operations) {
        perms.push(`ext:${extId}:${op}`);
      }
    }
  }
  return perms;
}

/** Risk levels for built-in permissions. Descriptions come from i18n. */
const PERMISSION_RISK: Record<string, "low" | "medium" | "high"> = {
  "system:info": "low",
  "filesystem:read": "medium",
  "filesystem:write": "high",
  "process:list": "medium",
  "docker:read": "medium",
  "docker:manage": "high",
  "network:local": "medium",
  "network:internet": "medium",
};
