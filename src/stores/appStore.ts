import { create } from "zustand";
import { persist } from "zustand/middleware";
import { toast } from "sonner";
import type { InstalledPlugin, RegistryEntry } from "../types/plugin";
import type { ExtensionRegistryEntry, ExtensionStatus } from "../types/extension";
import type { AvailableUpdate } from "../types/updates";

type View = "plugins" | "marketplace" | "settings" | "plugin-detail" | "extension-marketplace" | "extension-detail";
export type PluginAction = "starting" | "stopping" | "removing" | "rebuilding" | "updating";
export type ExtensionAction = "enabling" | "disabling" | "removing" | "updating";

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
  installedExtensions: ExtensionStatus[];
  busyExtensions: Record<string, ExtensionAction>;
  extensionMarketplaceEntries: ExtensionRegistryEntry[];
  selectedExtensionEntry: ExtensionRegistryEntry | null;
  availableUpdates: AvailableUpdate[];
  updateCheckInterval: number;
  installStatus: InstallStatus;
  showLogsPluginId: string | null;
  settingsTab: string;
  focusExtensionId: string | null;
  warmViewports: Record<string, true>;

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
  setExtensions: (extensions: ExtensionStatus[]) => void;
  removeExtension: (extId: string) => void;
  setExtensionBusy: (extId: string, action: ExtensionAction | null) => void;
  setExtensionMarketplace: (entries: ExtensionRegistryEntry[]) => void;
  selectExtensionEntry: (entry: ExtensionRegistryEntry | null) => void;
  setAvailableUpdates: (updates: AvailableUpdate[]) => void;
  setUpdateCheckInterval: (minutes: number) => void;
  setInstallStatus: (message: string | null) => void;
  setShowLogs: (pluginId: string | null) => void;
  setSettingsTab: (tab: string) => void;
  setFocusExtensionId: (id: string | null) => void;
  setWarmViewports: (ids: string[]) => void;
}

export const useAppStore = create<AppState>()(persist((set) => ({
  currentView: "plugins",
  installedPlugins: [],
  selectedPluginId: null,
  busyPlugins: {},
  marketplacePlugins: [],
  selectedRegistryEntry: null,
  searchQuery: "",
  isLoading: false,
  installedExtensions: [],
  busyExtensions: {},
  extensionMarketplaceEntries: [],
  selectedExtensionEntry: null,
  availableUpdates: [],
  updateCheckInterval: 30,
  installStatus: { active: false, message: "" },
  showLogsPluginId: null,
  settingsTab: "general",
  focusExtensionId: null,
  warmViewports: {},

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
  setExtensions: (extensions) => set({ installedExtensions: extensions }),
  removeExtension: (extId) =>
    set((state) => ({
      installedExtensions: state.installedExtensions.filter((e) => e.id !== extId),
    })),
  setExtensionBusy: (extId, action) =>
    set((state) => {
      const next = { ...state.busyExtensions };
      if (action) {
        next[extId] = action;
      } else {
        delete next[extId];
      }
      return { busyExtensions: next };
    }),
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
  setSettingsTab: (tab) => set({ settingsTab: tab }),
  setFocusExtensionId: (id) => set({ focusExtensionId: id }),
  setWarmViewports: (ids) => {
    const next: Record<string, true> = {};
    for (const id of ids) next[id] = true;
    return set({ warmViewports: next });
  },
}), {
  name: "nexus-nav",
  partialize: (state) => ({
    currentView: state.currentView,
    selectedPluginId: state.selectedPluginId,
    settingsTab: state.settingsTab,
  }),
}));
