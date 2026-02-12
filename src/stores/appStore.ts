import { create } from "zustand";
import type { InstalledPlugin, RegistryEntry } from "../types/plugin";

type View = "plugins" | "marketplace" | "settings" | "plugin-detail";

interface Notification {
  id: string;
  message: string;
  type: "info" | "success" | "error";
}

interface AppState {
  currentView: View;
  installedPlugins: InstalledPlugin[];
  selectedPluginId: string | null;
  marketplacePlugins: RegistryEntry[];
  selectedRegistryEntry: RegistryEntry | null;
  searchQuery: string;
  isLoading: boolean;
  notifications: Notification[];

  setView: (view: View) => void;
  setPlugins: (plugins: InstalledPlugin[]) => void;
  updatePlugin: (plugin: InstalledPlugin) => void;
  removePlugin: (pluginId: string) => void;
  selectPlugin: (pluginId: string | null) => void;
  setMarketplace: (plugins: RegistryEntry[]) => void;
  selectRegistryEntry: (entry: RegistryEntry | null) => void;
  setSearchQuery: (query: string) => void;
  setLoading: (loading: boolean) => void;
  addNotification: (message: string, type: Notification["type"]) => void;
  removeNotification: (id: string) => void;
}

let notifCounter = 0;

export const useAppStore = create<AppState>((set) => ({
  currentView: "plugins",
  installedPlugins: [],
  selectedPluginId: null,
  marketplacePlugins: [],
  selectedRegistryEntry: null,
  searchQuery: "",
  isLoading: false,
  notifications: [],

  setView: (view) => set({ currentView: view }),
  setPlugins: (plugins) => set({ installedPlugins: plugins }),
  updatePlugin: (plugin) =>
    set((state) => ({
      installedPlugins: state.installedPlugins.map((p) =>
        p.manifest.id === plugin.manifest.id ? plugin : p
      ),
    })),
  removePlugin: (pluginId) =>
    set((state) => ({
      installedPlugins: state.installedPlugins.filter(
        (p) => p.manifest.id !== pluginId
      ),
      selectedPluginId:
        state.selectedPluginId === pluginId ? null : state.selectedPluginId,
    })),
  selectPlugin: (pluginId) => set({ selectedPluginId: pluginId }),
  setMarketplace: (plugins) => set({ marketplacePlugins: plugins }),
  selectRegistryEntry: (entry) => set({ selectedRegistryEntry: entry }),
  setSearchQuery: (query) => set({ searchQuery: query }),
  setLoading: (loading) => set({ isLoading: loading }),
  addNotification: (message, type) => {
    const id = `notif-${++notifCounter}`;
    set((state) => ({
      notifications: [...state.notifications, { id, message, type }],
    }));
    setTimeout(() => {
      set((state) => ({
        notifications: state.notifications.filter((n) => n.id !== id),
      }));
    }, 5000);
  },
  removeNotification: (id) =>
    set((state) => ({
      notifications: state.notifications.filter((n) => n.id !== id),
    })),
}));
