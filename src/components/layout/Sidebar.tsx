import { useAppStore } from "../../stores/appStore";
import type { InstalledPlugin } from "../../types/plugin";
import { Plus, Settings, ArrowUp } from "lucide-react";
import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarGroup,
  SidebarGroupLabel,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuBadge,
  SidebarMenuButton,
  SidebarMenuItem,
} from "@/components/ui/sidebar";

const statusColor: Record<string, string> = {
  running: "bg-nx-success",
  stopped: "bg-nx-text-muted",
  error: "bg-nx-error",
  installing: "bg-nx-warning",
};

function PluginItem({ plugin }: { plugin: InstalledPlugin }) {
  const { selectedPluginId, selectPlugin, setView, availableUpdates } = useAppStore();
  const isSelected = selectedPluginId === plugin.manifest.id;
  const isRunning = plugin.status === "running";
  const hasUpdate = availableUpdates.some((u) => u.item_id === plugin.manifest.id);

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
          style={isRunning ? { animation: "pulse-status 2s ease-in-out infinite" } : undefined}
        />
        <span className="truncate">{plugin.manifest.name}</span>
      </SidebarMenuButton>
      {hasUpdate && (
        <SidebarMenuBadge>
          <ArrowUp size={12} strokeWidth={1.5} className="text-nx-accent" />
        </SidebarMenuBadge>
      )}
    </SidebarMenuItem>
  );
}

export function AppSidebar() {
  const { currentView, setView, installedPlugins, availableUpdates } = useAppStore();

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
        <SidebarGroup>
          <SidebarGroupLabel className="text-[10px] font-semibold text-nx-text-muted uppercase tracking-wider">
            Installed
          </SidebarGroupLabel>
          <SidebarMenu>
            {installedPlugins.length === 0 ? (
              <p className="text-[11px] text-nx-text-ghost px-2 py-2">
                No plugins installed
              </p>
            ) : (
              installedPlugins.map((plugin) => (
                <PluginItem key={plugin.manifest.id} plugin={plugin} />
              ))
            )}
          </SidebarMenu>
        </SidebarGroup>
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
