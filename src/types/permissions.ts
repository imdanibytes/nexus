import i18n from "../i18n";

/** Well-known built-in permissions. */
export type BuiltinPermission =
  | "system:info"
  | "filesystem:read"
  | "filesystem:write"
  | "process:list"
  | "container:read"
  | "container:manage"
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

/** Look up permission metadata. Supports built-in, ext:*, mcp:call, and mcp:* permissions. */
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
  // MCP blanket access
  if (perm === "mcp:call") {
    return {
      description: i18n.t("permissions:meta.mcp_call", { defaultValue: "Call any MCP tool from any plugin" }),
      risk: "high",
    };
  }
  // Per-plugin MCP access: mcp:{target_plugin_id}
  if (perm.startsWith("mcp:")) {
    const target = perm.slice(4);
    return {
      description: i18n.t("permissions:meta.mcpAccess", { target, defaultValue: `Access MCP tools from ${target}` }),
      risk: "medium",
    };
  }
  return { description: perm, risk: "medium" };
}

/** Compute the full permission list from a manifest (mirrors Rust all_permissions). */
export function allPermissions(manifest: {
  permissions: string[];
  extensions?: Record<string, string[] | Record<string, { scopes?: string[] }>>;
  mcp_access?: string[];
}): string[] {
  const perms = [...manifest.permissions];
  if (manifest.extensions) {
    for (const [extId, deps] of Object.entries(manifest.extensions)) {
      if (Array.isArray(deps)) {
        // Flat format: ["op1", "op2"]
        for (const op of deps) {
          perms.push(`ext:${extId}:${op}`);
        }
      } else {
        // Rich format: { "op1": { scopes: [...] }, "op2": {} }
        for (const op of Object.keys(deps)) {
          perms.push(`ext:${extId}:${op}`);
        }
      }
    }
  }
  if (manifest.mcp_access) {
    for (const target of manifest.mcp_access) {
      perms.push(`mcp:${target}`);
    }
  }
  return perms;
}

/** Extract pre-declared scopes for an extension permission from a rich manifest declaration. */
export function getManifestScopes(
  manifest: { extensions?: Record<string, string[] | Record<string, { scopes?: string[] }>> },
  perm: string,
): string[] | null {
  if (!perm.startsWith("ext:") || !manifest.extensions) return null;
  const parts = perm.slice(4).split(":");
  const extId = parts[0];
  const op = parts[1];
  if (!extId || !op) return null;
  const deps = manifest.extensions[extId];
  if (!deps || Array.isArray(deps)) return null;
  const decl = deps[op];
  if (!decl?.scopes?.length) return null;
  return decl.scopes;
}

/** Risk levels for built-in permissions. Descriptions come from i18n. */
const PERMISSION_RISK: Record<string, "low" | "medium" | "high"> = {
  "system:info": "low",
  "filesystem:read": "medium",
  "filesystem:write": "high",
  "process:list": "medium",
  "container:read": "medium",
  "container:manage": "high",
  "network:local": "medium",
  "network:internet": "medium",
  "mcp:call": "high",
};
