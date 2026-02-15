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
    <div className="flex flex-col h-full">
      {/* Header + tab strip */}
      <div className="flex-shrink-0 border-b border-nx-border bg-nx-deep/60">
        <div className="px-6 pt-5 pb-0">
          <h2 className="text-[15px] font-bold text-nx-text">Settings</h2>
          <p className="text-[11px] text-nx-text-ghost mt-0.5 mb-3">
            Manage your Nexus installation
          </p>
        </div>
        <div className="flex gap-0.5 px-5 overflow-x-auto">
          {TABS.map((tab) => {
            const Icon = tab.icon;
            const isActive = active === tab.id;
            return (
              <button
                key={tab.id}
                onClick={() => setActive(tab.id)}
                className={`flex items-center gap-1.5 px-3 py-2 text-[12px] font-medium rounded-t-[var(--radius-button)] transition-colors whitespace-nowrap border-b-2 ${
                  isActive
                    ? "border-nx-accent text-nx-text bg-nx-surface/50"
                    : "border-transparent text-nx-text-muted hover:text-nx-text-secondary hover:bg-nx-wash/30"
                }`}
              >
                <Icon size={14} strokeWidth={1.5} />
                {tab.label}
              </button>
            );
          })}
        </div>
      </div>

      {/* Content pane */}
      <div className="flex-1 overflow-y-auto p-6">
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
