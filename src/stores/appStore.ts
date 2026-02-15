import { create } from "zustand";
import { toast } from "sonner";
import type { InstalledPlugin, RegistryEntry } from "../types/plugin";
import type { ExtensionRegistryEntry } from "../types/extension";
import type { AvailableUpdate } from "../types/updates";

type View = "plugins" | "marketplace" | "settings" | "plugin-detail" | "extension-marketplace" | "extension-detail";
export type PluginAction = "starting" | "stopping" | "removing" | "rebuilding";

interface InstallStatus {
  active: boolean;
  message: string;
}

interface AppState {
  currentView: View;
  installedPlugins: InstalledPlugin[];
  selectedPluginId: string | null;
  busyPlugins: Record<string, PluginAction>;
  marketplacePlugins: RegistryEntry[];
  selectedRegistryEntry: RegistryEntry | null;
  searchQuery: string;
  isLoading: boolean;
  extensionMarketplaceEntries: ExtensionRegistryEntry[];
  selectedExtensionEntry: ExtensionRegistryEntry | null;
  availableUpdates: AvailableUpdate[];
  updateCheckInterval: number;
  installStatus: InstallStatus;
  showLogsPluginId: string | null;

  setView: (view: View) => void;
  setPlugins: (plugins: InstalledPlugin[]) => void;
  updatePlugin: (plugin: InstalledPlugin) => void;
  removePlugin: (pluginId: string) => void;
  selectPlugin: (pluginId: string | null) => void;
  setBusy: (pluginId: string, action: PluginAction | null) => void;
  setMarketplace: (plugins: RegistryEntry[]) => void;
  selectRegistryEntry: (entry: RegistryEntry | null) => void;
  setSearchQuery: (query: string) => void;
  setLoading: (loading: boolean) => void;
  addNotification: (message: string, type: "info" | "success" | "error") => void;
  setExtensionMarketplace: (entries: ExtensionRegistryEntry[]) => void;
  selectExtensionEntry: (entry: ExtensionRegistryEntry | null) => void;
  setAvailableUpdates: (updates: AvailableUpdate[]) => void;
  setUpdateCheckInterval: (minutes: number) => void;
  setInstallStatus: (message: string | null) => void;
  setShowLogs: (pluginId: string | null) => void;
}

export const useAppStore = create<AppState>((set) => ({
  currentView: "plugins",
  installedPlugins: [],
  selectedPluginId: null,
  busyPlugins: {},
  marketplacePlugins: [],
  selectedRegistryEntry: null,
  searchQuery: "",
  isLoading: false,
  extensionMarketplaceEntries: [],
  selectedExtensionEntry: null,
  availableUpdates: [],
  updateCheckInterval: 30,
  installStatus: { active: false, message: "" },
  showLogsPluginId: null,

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
  setBusy: (pluginId, action) =>
    set((state) => {
      const next = { ...state.busyPlugins };
      if (action) {
        next[pluginId] = action;
      } else {
        delete next[pluginId];
      }
      return { busyPlugins: next };
    }),
  setMarketplace: (plugins) => set({ marketplacePlugins: plugins }),
  selectRegistryEntry: (entry) => set({ selectedRegistryEntry: entry }),
  setExtensionMarketplace: (entries) => set({ extensionMarketplaceEntries: entries }),
  selectExtensionEntry: (entry) => set({ selectedExtensionEntry: entry }),
  setAvailableUpdates: (updates) => set({ availableUpdates: updates }),
  setUpdateCheckInterval: (minutes) => set({ updateCheckInterval: minutes }),
  setSearchQuery: (query) => set({ searchQuery: query }),
  setLoading: (loading) => set({ isLoading: loading }),
  addNotification: (message, type) => {
    if (type === "success") toast.success(message);
    else if (type === "error") toast.error(message);
    else toast.info(message);
  },
  setInstallStatus: (message) =>
    set({
      installStatus: message
        ? { active: true, message }
        : { active: false, message: "" },
    }),
  setShowLogs: (pluginId) => set({ showLogsPluginId: pluginId }),
}));
