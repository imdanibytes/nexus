import { useCallback, useState } from "react";
import * as api from "../lib/tauri";
import type { GrantedPermission, Permission } from "../types/permissions";
import { useAppStore } from "../stores/appStore";

export function usePermissions() {
  const [grants, setGrants] = useState<GrantedPermission[]>([]);

  const loadGrants = useCallback(
    async (pluginId: string) => {
      try {
        const result = await api.permissionList(pluginId);
        setGrants(result);
        return result;
      } catch (e) {
        useAppStore.getState().addNotification(`Failed to load permissions: ${e}`, "error");
        return [];
      }
    },
    []
  );

  const grant = useCallback(
    async (pluginId: string, permissions: Permission[]) => {
      try {
        await api.permissionGrant(pluginId, permissions);
        useAppStore.getState().addNotification("Permissions granted", "success");
        await loadGrants(pluginId);
      } catch (e) {
        useAppStore.getState().addNotification(`Failed to grant permissions: ${e}`, "error");
      }
    },
    [loadGrants]
  );

  const revoke = useCallback(
    async (pluginId: string, permissions: Permission[]) => {
      try {
        await api.permissionRevoke(pluginId, permissions);
        useAppStore.getState().addNotification("Permissions revoked", "info");
        await loadGrants(pluginId);
      } catch (e) {
        useAppStore.getState().addNotification(`Failed to revoke permissions: ${e}`, "error");
      }
    },
    [loadGrants]
  );

  const unrevoke = useCallback(
    async (pluginId: string, permissions: Permission[]) => {
      try {
        await api.permissionUnrevoke(pluginId, permissions);
        useAppStore.getState().addNotification("Permission restored", "success");
        await loadGrants(pluginId);
      } catch (e) {
        useAppStore.getState().addNotification(`Failed to restore permission: ${e}`, "error");
      }
    },
    [loadGrants]
  );

  const removePath = useCallback(
    async (pluginId: string, permission: Permission, path: string) => {
      try {
        await api.permissionRemovePath(pluginId, permission, path);
        useAppStore.getState().addNotification(`Revoked access to ${path}`, "info");
        await loadGrants(pluginId);
      } catch (e) {
        useAppStore.getState().addNotification(`Failed to remove path: ${e}`, "error");
      }
    },
    [loadGrants]
  );

  return { grants, loadGrants, grant, revoke, unrevoke, removePath };
}
