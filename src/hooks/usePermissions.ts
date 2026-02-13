import { useCallback, useState } from "react";
import * as api from "../lib/tauri";
import type { GrantedPermission, Permission } from "../types/permissions";
import { useAppStore } from "../stores/appStore";

export function usePermissions() {
  const [grants, setGrants] = useState<GrantedPermission[]>([]);
  const { addNotification } = useAppStore();

  const loadGrants = useCallback(
    async (pluginId: string) => {
      try {
        const result = await api.permissionList(pluginId);
        setGrants(result);
        return result;
      } catch (e) {
        addNotification(`Failed to load permissions: ${e}`, "error");
        return [];
      }
    },
    [addNotification]
  );

  const grant = useCallback(
    async (pluginId: string, permissions: Permission[]) => {
      try {
        await api.permissionGrant(pluginId, permissions);
        addNotification("Permissions granted", "success");
        await loadGrants(pluginId);
      } catch (e) {
        addNotification(`Failed to grant permissions: ${e}`, "error");
      }
    },
    [loadGrants, addNotification]
  );

  const revoke = useCallback(
    async (pluginId: string, permissions: Permission[]) => {
      try {
        await api.permissionRevoke(pluginId, permissions);
        addNotification("Permissions revoked", "info");
        await loadGrants(pluginId);
      } catch (e) {
        addNotification(`Failed to revoke permissions: ${e}`, "error");
      }
    },
    [loadGrants, addNotification]
  );

  const unrevoke = useCallback(
    async (pluginId: string, permissions: Permission[]) => {
      try {
        await api.permissionUnrevoke(pluginId, permissions);
        addNotification("Permission restored", "success");
        await loadGrants(pluginId);
      } catch (e) {
        addNotification(`Failed to restore permission: ${e}`, "error");
      }
    },
    [loadGrants, addNotification]
  );

  const removePath = useCallback(
    async (pluginId: string, permission: Permission, path: string) => {
      try {
        await api.permissionRemovePath(pluginId, permission, path);
        addNotification(`Revoked access to ${path}`, "info");
        await loadGrants(pluginId);
      } catch (e) {
        addNotification(`Failed to remove path: ${e}`, "error");
      }
    },
    [loadGrants, addNotification]
  );

  return { grants, loadGrants, grant, revoke, unrevoke, removePath };
}
