/**
 * Single Zustand store for ALL app state. Do not create additional stores.
 *
 * State updates flow through lifecycle events from the backend:
 *   Component fires Tauri command -> Backend processes -> Backend emits lifecycle event
 *   -> useLifecycleEvents listener -> store mutation -> React re-render
 *
 * Components MUST NOT call setBusy/setExtensionBusy/setPlugins/etc directly.
 * They only fire commands via lib/tauri.ts wrappers.
 */
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
  updateChannel: "stable" | "nightly";
  notifications: Notification[];

  setView: (view: View) => void;
  setPlugins: (plugins: InstalledPlugin[]) => void;
  updatePlugin: (plugin: InstalledPlugin) => void;
  updateExtension: (extension: ExtensionStatus) => void;
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
  setUpdateChannel: (channel: "stable" | "nightly") => void;
  setWarmViewports: (ids: string[]) => void;
  notify: (
    category: string,
    message: string,
    opts?: { pluginId?: string; data?: unknown },
  ) => string;
  dismiss: (id: string) => void;
  dismissByCategory: (prefix: string) => void;
  clearAll: () => void;
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
  updateCheckInterval: 1440,
  installStatus: { active: false, message: "" },
  showLogsPluginId: null,
  settingsTab: "general",
  focusExtensionId: null,
  warmViewports: {},
  updateChannel: "stable",

  setView: (view) => set({ currentView: view }),
  notifications: [],

  setPlugins: (plugins) => set({ installedPlugins: plugins }),
  updatePlugin: (plugin) =>
    set((state) => {
      const idx = state.installedPlugins.findIndex(
        (p) => p.manifest.id === plugin.manifest.id
      );
      if (idx >= 0) {
        const next = [...state.installedPlugins];
        next[idx] = plugin;
        return { installedPlugins: next };
      }
      return { installedPlugins: [...state.installedPlugins, plugin] };
    }),
  updateExtension: (extension) =>
    set((state) => {
      const idx = state.installedExtensions.findIndex((e) => e.id === extension.id);
      if (idx >= 0) {
        const next = [...state.installedExtensions];
        next[idx] = extension;
        return { installedExtensions: next };
      }
      return { installedExtensions: [...state.installedExtensions, extension] };
    }),
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
  setUpdateChannel: (channel) => set({ updateChannel: channel }),
  setWarmViewports: (ids) => {
    const next: Record<string, true> = {};
    for (const id of ids) next[id] = true;
    return set({ warmViewports: next });
  },

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
}), {
  name: "nexus-nav",
  partialize: (state) => ({
    currentView: state.currentView,
    selectedPluginId: state.selectedPluginId,
    settingsTab: state.settingsTab,
  }),
}));

/** Total count, or count matching a category prefix */
export function useNotificationCount(prefix?: string): number {
  return useAppStore((state) =>
    prefix
      ? state.notifications.filter((n) =>
          n.meta.category.startsWith(prefix),
        ).length
      : state.notifications.length,
  );
}

/** All notifications matching a category prefix */
export function useNotifications(prefix: string): Notification[] {
  return useAppStore((state) =>
    state.notifications.filter((n) => n.meta.category.startsWith(prefix)),
  );
}
