import { useState } from "react";
import { GeneralTab } from "./GeneralTab";
import { RuntimeTab } from "./RuntimeTab";
import { ResourcesTab } from "./ResourcesTab";
import { PluginsTab } from "./PluginsTab";
import { PermissionsTab } from "./PermissionsTab";
import { McpTab } from "./McpTab";
import { ExtensionsTab } from "./ExtensionsTab";
import { NotificationsTab } from "./NotificationsTab";
import { UpdatesTab } from "./UpdatesTab";
import { Settings, Container, Gauge, Puzzle, Shield, Cpu, Blocks, Bell, ArrowUpCircle } from "lucide-react";
import { ErrorBoundary } from "../ErrorBoundary";

type SettingsTab = "general" | "runtime" | "resources" | "plugins" | "permissions" | "mcp" | "extensions" | "notifications" | "updates";

const TABS: { id: SettingsTab; label: string; icon: typeof Settings }[] = [
  { id: "general", label: "General", icon: Settings },
  { id: "runtime", label: "Runtime", icon: Container },
  { id: "resources", label: "Resources", icon: Gauge },
  { id: "plugins", label: "Plugins", icon: Puzzle },
  { id: "permissions", label: "Permissions", icon: Shield },
  { id: "notifications", label: "Notifications", icon: Bell },
  { id: "mcp", label: "MCP", icon: Cpu },
  { id: "extensions", label: "Extensions", icon: Blocks },
  { id: "updates", label: "Updates", icon: ArrowUpCircle },
];

export function SettingsPage() {
  const [active, setActive] = useState<SettingsTab>("general");

  return (
    <div className="flex h-full">
      {/* Tab rail */}
      <nav className="w-44 flex-shrink-0 border-r border-nx-border bg-nx-deep p-3 space-y-1">
        <div className="mb-4 px-2">
          <h2 className="text-[15px] font-bold text-nx-text">Settings</h2>
          <p className="text-[11px] text-nx-text-ghost mt-0.5">
            Manage your Nexus installation
          </p>
        </div>

        {TABS.map((tab) => {
          const Icon = tab.icon;
          const isActive = active === tab.id;
          return (
            <button
              key={tab.id}
              onClick={() => setActive(tab.id)}
              className={`w-full flex items-center gap-2.5 px-3 py-2 text-[13px] font-medium rounded-[var(--radius-button)] transition-all duration-150 ${
                isActive
                  ? "bg-nx-surface text-nx-text shadow-sm border border-nx-border"
                  : "text-nx-text-muted hover:text-nx-text-secondary hover:bg-nx-wash/40"
              }`}
            >
              <Icon size={15} strokeWidth={1.5} />
              {tab.label}
            </button>
          );
        })}
      </nav>

      {/* Content pane */}
      <div className="flex-1 overflow-y-auto p-6 max-w-2xl">
        <ErrorBoundary label={TABS.find((t) => t.id === active)?.label}>
          {active === "general" && <GeneralTab />}
          {active === "runtime" && <RuntimeTab />}
          {active === "resources" && <ResourcesTab />}
          {active === "plugins" && <PluginsTab />}
          {active === "permissions" && <PermissionsTab />}
          {active === "notifications" && <NotificationsTab />}
          {active === "mcp" && <McpTab />}
          {active === "extensions" && <ExtensionsTab />}
          {active === "updates" && <UpdatesTab />}
        </ErrorBoundary>
      </div>
    </div>
  );
}
