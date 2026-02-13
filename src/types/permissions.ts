export type Permission =
  | "system:info"
  | "filesystem:read"
  | "filesystem:write"
  | "process:list"
  | "docker:read"
  | "docker:manage"
  | "network:local"
  | "network:internet";

export interface GrantedPermission {
  plugin_id: string;
  permission: Permission;
  granted_at: string;
  approved_paths: string[] | null;
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

export const PERMISSION_INFO: Record<
  Permission,
  { description: string; risk: "low" | "medium" | "high" }
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
