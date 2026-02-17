import { create } from "zustand";

export interface NotificationMeta {
  /** Dot-delimited category for hierarchical querying.
   *  e.g. "updates.plugins", "system.engine", "plugin.com.nexus.agent" */
  category: string;
  /** Optional source plugin ID — enables per-plugin sidebar badges later */
  pluginId?: string;
}

export interface Notification {
  id: string;
  message: string;
  meta: NotificationMeta;
  createdAt: number;
  data?: unknown;
}

interface NotificationState {
  notifications: Notification[];
  notify: (
    category: string,
    message: string,
    opts?: { pluginId?: string; data?: unknown },
  ) => string;
  dismiss: (id: string) => void;
  dismissByCategory: (prefix: string) => void;
  clearAll: () => void;
}

export const useNotificationStore = create<NotificationState>()((set) => ({
  notifications: [],

  notify: (category, message, opts) => {
    const id = crypto.randomUUID();
    set((state) => ({
      notifications: [
        ...state.notifications,
        {
          id,
          message,
          meta: { category, pluginId: opts?.pluginId },
          createdAt: Date.now(),
          data: opts?.data,
        },
      ],
    }));
    return id;
  },

  dismiss: (id) =>
    set((state) => ({
      notifications: state.notifications.filter((n) => n.id !== id),
    })),

  dismissByCategory: (prefix) =>
    set((state) => ({
      notifications: state.notifications.filter(
        (n) => !n.meta.category.startsWith(prefix),
      ),
    })),

  clearAll: () => set({ notifications: [] }),
}));

/** Total count, or count matching a category prefix */
export function useNotificationCount(prefix?: string): number {
  return useNotificationStore((state) =>
    prefix
      ? state.notifications.filter((n) =>
          n.meta.category.startsWith(prefix),
        ).length
      : state.notifications.length,
  );
}

/** All notifications matching a category prefix */
export function useNotifications(prefix: string): Notification[] {
  return useNotificationStore((state) =>
    state.notifications.filter((n) => n.meta.category.startsWith(prefix)),
  );
}
