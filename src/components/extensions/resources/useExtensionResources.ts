import { useState, useCallback } from "react";
import * as api from "../../../lib/tauri";

export function useExtensionResources(extId: string, resourceType: string) {
  const [items, setItems] = useState<Record<string, unknown>[]>([]);
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const result = await api.extensionResourceList(extId, resourceType);
      setItems(result.items);
      setTotal(result.total);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, [extId, resourceType]);

  const create = useCallback(async (data: Record<string, unknown>) => {
    const result = await api.extensionResourceCreate(extId, resourceType, data);
    await refresh();
    return result;
  }, [extId, resourceType, refresh]);

  const update = useCallback(async (id: string, data: Record<string, unknown>) => {
    const result = await api.extensionResourceUpdate(extId, resourceType, id, data);
    await refresh();
    return result;
  }, [extId, resourceType, refresh]);

  const remove = useCallback(async (id: string) => {
    await api.extensionResourceDelete(extId, resourceType, id);
    await refresh();
  }, [extId, resourceType, refresh]);

  return { items, total, loading, error, refresh, create, update, remove };
}
