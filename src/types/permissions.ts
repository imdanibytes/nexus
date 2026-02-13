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

export interface GrantedPermission {
  plugin_id: string;
  permission: Permission;
  granted_at: string;
  /** Generalized scope whitelist (renamed from approved_paths). */
  approved_scopes: string[] | null;
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
  const builtin = PERMISSION_INFO[perm as BuiltinPermission];
  if (builtin) return builtin;
  // Extension permissions: ext:{ext_id}:{operation}
  if (perm.startsWith("ext:")) {
    const parts = perm.slice(4).split(":");
    const extId = parts[0] || "unknown";
    const op = parts[1] || "unknown";
    return {
      description: `Extension ${extId}: ${op.replace(/_/g, " ")}`,
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

export const PERMISSION_INFO: Record<
  string,
  PermissionMeta
> = {
  "system:info": {
    description: "Read OS info, hostname, uptime",
    risk: "low",
  },
  "filesystem:read": {
    description: "Read files on approved paths",
    risk: "medium",
  },
  "filesystem:write": {
    description: "Write files to approved paths",
    risk: "high",
  },
  "process:list": {
    description: "List running processes",
    risk: "medium",
  },
  "docker:read": {
    description: "List containers, read stats",
    risk: "medium",
  },
  "docker:manage": {
    description: "Start/stop/create containers",
    risk: "high",
  },
  "network:local": {
    description: "HTTP requests to LAN",
    risk: "medium",
  },
  "network:internet": {
    description: "HTTP requests to internet",
    risk: "medium",
  },
};
