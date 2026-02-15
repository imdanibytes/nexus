import { useState } from "react";
import { useAppStore } from "../../stores/appStore";
import type { InstalledPlugin } from "../../types/plugin";
import type { ExtensionStatus } from "../../types/extension";
import * as api from "../../lib/tauri";
import { Plus, Settings, ArrowUp, Play, Square, ScrollText, Trash2, Hammer, Wrench, MoreHorizontal, TriangleAlert, Power, Puzzle } from "lucide-react";
import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarGroup,
  SidebarGroupLabel,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuAction,
  SidebarMenuBadge,
  SidebarMenuButton,
  SidebarMenuItem,
} from "@/components/ui/sidebar";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";

const statusColor: Record<string, string> = {
  running: "bg-nx-success",
  stopped: "bg-nx-text-muted",
  error: "bg-nx-error",
  installing: "bg-nx-warning",
};

function PluginItem({ plugin }: { plugin: InstalledPlugin }) {
  const { selectedPluginId, selectPlugin, setView, availableUpdates, busyPlugins, setBusy, removePlugin, addNotification, setShowLogs, warmViewports } = useAppStore();
  const isSelected = selectedPluginId === plugin.manifest.id;
  const isRunning = plugin.status === "running";
  const isWarm = !!warmViewports[plugin.manifest.id];
  const hasUpdate = availableUpdates.some((u) => u.item_id === plugin.manifest.id);
  const isBusy = !!busyPlugins[plugin.manifest.id];
  const isLocal = !!plugin.local_manifest_path;
  const [removeDialogOpen, setRemoveDialogOpen] = useState(false);

  async function handleStart() {
    const id = plugin.manifest.id;
    setBusy(id, "starting");
    try {
      await api.pluginStart(id);
      addNotification("Plugin started", "success");
      const plugins = await api.pluginList();
      useAppStore.getState().setPlugins(plugins);
    } catch (e) {
      addNotification(`Start failed: ${e}`, "error");
    } finally {
      setBusy(id, null);
    }
  }

  async function handleStop() {
    const id = plugin.manifest.id;
    setBusy(id, "stopping");
    try {
      await api.pluginStop(id);
      addNotification("Plugin stopped", "info");
      const plugins = await api.pluginList();
      useAppStore.getState().setPlugins(plugins);
    } catch (e) {
      addNotification(`Stop failed: ${e}`, "error");
    } finally {
      setBusy(id, null);
    }
  }

  async function handleRemove() {
    const id = plugin.manifest.id;
    setRemoveDialogOpen(false);
    setBusy(id, "removing");
    try {
      await api.pluginRemove(id);
      removePlugin(id);
      addNotification("Plugin removed", "info");
    } catch (e) {
      addNotification(`Remove failed: ${e}`, "error");
    } finally {
      setBusy(id, null);
    }
  }

  async function handleRebuild() {
    const id = plugin.manifest.id;
    setBusy(id, "rebuilding");
    try {
      await api.pluginRebuild(id);
      addNotification("Plugin rebuilt", "success");
      const plugins = await api.pluginList();
      useAppStore.getState().setPlugins(plugins);
    } catch (e) {
      addNotification(`Rebuild failed: ${e}`, "error");
    } finally {
      setBusy(id, null);
    }
  }

  async function handleToggleDevMode() {
    const id = plugin.manifest.id;
    const next = !plugin.dev_mode;
    try {
      await api.pluginDevModeToggle(id, next);
      addNotification(next ? "Dev mode enabled" : "Dev mode disabled", "info");
      const plugins = await api.pluginList();
      useAppStore.getState().setPlugins(plugins);
    } catch (e) {
      addNotification(`Dev mode toggle failed: ${e}`, "error");
    }
  }

  return (
    <SidebarMenuItem>
      <SidebarMenuButton
        size="sm"
        isActive={isSelected}
        onClick={() => {
          selectPlugin(plugin.manifest.id);
          setView("plugins");
        }}
        className="text-[12px]"
      >
        <span
          className={`w-1.5 h-1.5 rounded-full shrink-0 ${statusColor[plugin.status] ?? "bg-nx-text-muted"}`}
          style={isRunning && !isWarm ? { animation: "pulse-status 2s ease-in-out infinite" } : undefined}
        />
        <span className="truncate">{plugin.manifest.name}</span>
      </SidebarMenuButton>

      {hasUpdate && (
        <SidebarMenuBadge>
          <ArrowUp size={12} strokeWidth={1.5} className="text-nx-accent" />
        </SidebarMenuBadge>
      )}

      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <SidebarMenuAction showOnHover className="text-nx-text-ghost hover:text-nx-text">
            <MoreHorizontal size={14} strokeWidth={1.5} />
          </SidebarMenuAction>
        </DropdownMenuTrigger>
        <DropdownMenuContent side="right" align="start" className="w-48">
          {isRunning ? (
            <DropdownMenuItem onClick={handleStop} disabled={isBusy}>
              <Square size={14} strokeWidth={1.5} className="text-nx-warning" />
              Stop
            </DropdownMenuItem>
          ) : (
            <DropdownMenuItem onClick={handleStart} disabled={isBusy}>
              <Play size={14} strokeWidth={1.5} className="text-nx-success" />
              Start
            </DropdownMenuItem>
          )}

          <DropdownMenuItem onClick={() => setShowLogs(plugin.manifest.id)}>
            <ScrollText size={14} strokeWidth={1.5} />
            Logs
          </DropdownMenuItem>

          {isLocal && (
            <>
              <DropdownMenuSeparator />
              <DropdownMenuItem onClick={handleRebuild} disabled={isBusy}>
                <Hammer size={14} strokeWidth={1.5} className="text-nx-accent" />
                Rebuild
              </DropdownMenuItem>
              <DropdownMenuItem onClick={handleToggleDevMode} disabled={isBusy}>
                <Wrench size={14} strokeWidth={1.5} />
                {plugin.dev_mode ? "Disable Dev Mode" : "Enable Dev Mode"}
              </DropdownMenuItem>
            </>
          )}

          <DropdownMenuSeparator />
          <DropdownMenuItem
            variant="destructive"
            onClick={() => setRemoveDialogOpen(true)}
            disabled={isBusy}
          >
            <Trash2 size={14} strokeWidth={1.5} />
            Remove
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>

      <Dialog open={removeDialogOpen} onOpenChange={setRemoveDialogOpen}>
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2 text-base">
              <TriangleAlert size={18} className="text-nx-warning" />
              Remove {plugin.manifest.name}?
            </DialogTitle>
            <DialogDescription className="text-[13px] leading-relaxed pt-1">
              This will permanently delete all plugin data, including stored files and settings.
              This action cannot be undone.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter className="pt-2">
            <Button
              variant="secondary"
              size="sm"
              onClick={() => setRemoveDialogOpen(false)}
            >
              Cancel
            </Button>
            <Button
              variant="destructive"
              size="sm"
              onClick={handleRemove}
              className="bg-nx-error text-white hover:bg-nx-error/80"
            >
              Remove & Delete Data
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </SidebarMenuItem>
  );
}

function ExtensionItem({ ext }: { ext: ExtensionStatus }) {
  const { busyExtensions, setExtensionBusy, setExtensions, addNotification, setView, setSettingsTab, setFocusExtensionId } = useAppStore();
  const isBusy = !!busyExtensions[ext.id];
  const [removeDialogOpen, setRemoveDialogOpen] = useState(false);

  async function handleToggle() {
    const action = ext.enabled ? "disabling" : "enabling";
    setExtensionBusy(ext.id, action);
    try {
      if (ext.enabled) {
        await api.extensionDisable(ext.id);
        addNotification(`Extension "${ext.display_name}" disabled`, "info");
      } else {
        await api.extensionEnable(ext.id);
        addNotification(`Extension "${ext.display_name}" enabled`, "success");
      }
      const exts = await api.extensionList();
      setExtensions(exts);
    } catch (e) {
      addNotification(`Failed to ${ext.enabled ? "disable" : "enable"} extension: ${e}`, "error");
    } finally {
      setExtensionBusy(ext.id, null);
    }
  }

  async function handleRemove() {
    setRemoveDialogOpen(false);
    setExtensionBusy(ext.id, "removing");
    try {
      await api.extensionRemove(ext.id);
      useAppStore.getState().removeExtension(ext.id);
      addNotification(`Extension "${ext.display_name}" removed`, "info");
    } catch (e) {
      addNotification(`Failed to remove extension: ${e}`, "error");
    } finally {
      setExtensionBusy(ext.id, null);
    }
  }

  return (
    <SidebarMenuItem>
      <SidebarMenuButton
        size="sm"
        className="text-[12px]"
        onClick={() => {
          setSettingsTab("extensions");
          setFocusExtensionId(ext.id);
          setView("settings");
        }}
      >
        <span
          className={`w-1.5 h-1.5 rounded-full shrink-0 ${ext.enabled ? "bg-nx-success" : "bg-nx-text-muted"}`}
        />
        <span className="truncate">{ext.display_name}</span>
      </SidebarMenuButton>

      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <SidebarMenuAction showOnHover className="text-nx-text-ghost hover:text-nx-text">
            <MoreHorizontal size={14} strokeWidth={1.5} />
          </SidebarMenuAction>
        </DropdownMenuTrigger>
        <DropdownMenuContent side="right" align="start" className="w-48">
          <DropdownMenuItem onClick={handleToggle} disabled={isBusy}>
            <Power size={14} strokeWidth={1.5} className={ext.enabled ? "text-nx-warning" : "text-nx-success"} />
            {ext.enabled ? "Disable" : "Enable"}
          </DropdownMenuItem>

          <DropdownMenuItem onClick={() => {
            setSettingsTab("extensions");
            setView("settings");
          }}>
            <Settings size={14} strokeWidth={1.5} />
            Manage Extensions
          </DropdownMenuItem>

          <DropdownMenuSeparator />
          <DropdownMenuItem
            variant="destructive"
            onClick={() => setRemoveDialogOpen(true)}
            disabled={isBusy}
          >
            <Trash2 size={14} strokeWidth={1.5} />
            Remove
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>

      <Dialog open={removeDialogOpen} onOpenChange={setRemoveDialogOpen}>
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2 text-base">
              <TriangleAlert size={18} className="text-nx-warning" />
              Remove {ext.display_name}?
            </DialogTitle>
            <DialogDescription className="text-[13px] leading-relaxed pt-1" asChild>
              <div>
                {ext.consumers.length > 0 ? (
                  <>
                    <p>
                      The following plugin{ext.consumers.length !== 1 ? "s" : ""} will
                      lose access to this extension's operations:
                    </p>
                    <ul className="mt-2 space-y-1.5">
                      {ext.consumers.map((c) => (
                        <li
                          key={c.plugin_id}
                          className="flex items-center gap-2 px-3 py-2 rounded-[var(--radius-button)] bg-nx-deep border border-nx-border-subtle"
                        >
                          <Puzzle size={12} strokeWidth={1.5} className="text-nx-text-ghost flex-shrink-0" />
                          <span className="text-[12px] text-nx-text font-medium truncate">
                            {c.plugin_name}
                          </span>
                        </li>
                      ))}
                    </ul>
                  </>
                ) : (
                  <p>
                    No plugins currently use this extension.
                    You can reinstall it later from the marketplace.
                  </p>
                )}
              </div>
            </DialogDescription>
          </DialogHeader>
          <DialogFooter className="pt-2">
            <Button
              variant="secondary"
              size="sm"
              onClick={() => setRemoveDialogOpen(false)}
            >
              Cancel
            </Button>
            <Button
              variant="destructive"
              size="sm"
              onClick={handleRemove}
              className="bg-nx-error text-white hover:bg-nx-error/80"
            >
              Remove Extension
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </SidebarMenuItem>
  );
}

export function AppSidebar() {
  const { currentView, setView, installedPlugins, installedExtensions, availableUpdates } = useAppStore();

  const plugins = installedPlugins.filter((p) => p.manifest.ui !== null);
  const integrations = installedPlugins.filter((p) => p.manifest.ui === null);

  return (
    <Sidebar
      collapsible="none"
      className="border-r border-nx-border"
      style={{
        background: "rgba(34, 38, 49, 0.85)",
        backdropFilter: "blur(12px)",
        WebkitBackdropFilter: "blur(12px)",
      }}
    >
      <SidebarHeader className="px-4 py-4 border-b border-nx-border-subtle">
        <h1 className="text-[15px] font-bold tracking-tight">
          <span className="text-nx-accent">Nexus</span>
        </h1>
        <p className="text-[10px] text-nx-text-muted font-medium tracking-wide uppercase mt-0.5">
          Plugin Dashboard
        </p>
      </SidebarHeader>

      <SidebarContent>
        {installedPlugins.length === 0 ? (
          <SidebarGroup>
            <SidebarGroupLabel className="text-[10px] font-semibold text-nx-text-muted uppercase tracking-wider">
              Installed
            </SidebarGroupLabel>
            <SidebarMenu>
              <p className="text-[11px] text-nx-text-ghost px-2 py-2">
                No plugins installed
              </p>
            </SidebarMenu>
          </SidebarGroup>
        ) : (
          <>
            {plugins.length > 0 && (
              <SidebarGroup>
                <SidebarGroupLabel className="text-[10px] font-semibold text-nx-text-muted uppercase tracking-wider">
                  Plugins
                </SidebarGroupLabel>
                <SidebarMenu>
                  {plugins.map((plugin) => (
                    <PluginItem key={plugin.manifest.id} plugin={plugin} />
                  ))}
                </SidebarMenu>
              </SidebarGroup>
            )}
            {integrations.length > 0 && (
              <SidebarGroup>
                <SidebarGroupLabel className="text-[10px] font-semibold text-nx-text-muted uppercase tracking-wider">
                  Integrations
                </SidebarGroupLabel>
                <SidebarMenu>
                  {integrations.map((plugin) => (
                    <PluginItem key={plugin.manifest.id} plugin={plugin} />
                  ))}
                </SidebarMenu>
              </SidebarGroup>
            )}
            {installedExtensions.length > 0 && (
              <SidebarGroup>
                <SidebarGroupLabel className="text-[10px] font-semibold text-nx-text-muted uppercase tracking-wider">
                  Extensions
                </SidebarGroupLabel>
                <SidebarMenu>
                  {installedExtensions.map((ext) => (
                    <ExtensionItem key={ext.id} ext={ext} />
                  ))}
                </SidebarMenu>
              </SidebarGroup>
            )}
          </>
        )}
      </SidebarContent>

      <SidebarFooter className="border-t border-nx-border-subtle">
        <SidebarMenu>
          <SidebarMenuItem>
            <SidebarMenuButton
              size="sm"
              isActive={currentView === "marketplace" || currentView === "plugin-detail"}
              onClick={() => setView("marketplace")}
              className="text-[12px]"
            >
              <Plus size={15} strokeWidth={1.5} />
              Add Plugins
            </SidebarMenuButton>
          </SidebarMenuItem>
          <SidebarMenuItem>
            <SidebarMenuButton
              size="sm"
              isActive={currentView === "settings"}
              onClick={() => setView("settings")}
              className="text-[12px]"
            >
              <Settings size={15} strokeWidth={1.5} />
              Settings
            </SidebarMenuButton>
            {availableUpdates.length > 0 && (
              <SidebarMenuBadge className="min-w-[16px] h-4 px-1 text-[9px] font-bold rounded-full bg-nx-accent text-nx-deep">
                {availableUpdates.length}
              </SidebarMenuBadge>
            )}
          </SidebarMenuItem>
        </SidebarMenu>
      </SidebarFooter>
    </Sidebar>
  );
}
