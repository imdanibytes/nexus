import { useAppStore } from "../../stores/appStore";
import { GeneralTab } from "./GeneralTab";
import { SystemTab } from "./SystemTab";
import { PluginsTab } from "./PluginsTab";
import { McpTab } from "./McpTab";
import { ExtensionsTab } from "./ExtensionsTab";
import { UpdatesTab } from "./UpdatesTab";
import { HelpTab } from "./HelpTab";
import { Settings, Monitor, Puzzle, Cpu, Blocks, ArrowUpCircle, HelpCircle } from "lucide-react";
import { ErrorBoundary } from "../ErrorBoundary";

type SettingsTab = "general" | "system" | "plugins" | "mcp" | "extensions" | "updates" | "help";

const TABS: { id: SettingsTab; label: string; icon: typeof Settings }[] = [
  { id: "general", label: "General", icon: Settings },
  { id: "system", label: "System", icon: Monitor },
  { id: "plugins", label: "Plugins", icon: Puzzle },
  { id: "mcp", label: "MCP", icon: Cpu },
  { id: "extensions", label: "Extensions", icon: Blocks },
  { id: "updates", label: "Updates", icon: ArrowUpCircle },
  { id: "help", label: "Help", icon: HelpCircle },
];

const TAB_IDS = new Set<string>(TABS.map((t) => t.id));

// Map old persisted tab IDs to their new homes so bookmarks/deep links still work
const TAB_REDIRECTS: Record<string, SettingsTab> = {
  runtime: "system",
  resources: "system",
  permissions: "plugins",
  notifications: "general",
};

export function SettingsPage() {
  const { settingsTab, setSettingsTab } = useAppStore();
  const resolved = TAB_REDIRECTS[settingsTab] ?? settingsTab;
  const active = (TAB_IDS.has(resolved) ? resolved : "general") as SettingsTab;

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
        <div className="flex gap-0.5 px-5 overflow-x-auto [scrollbar-width:none] [&::-webkit-scrollbar]:hidden">
          {TABS.map((tab) => {
            const Icon = tab.icon;
            const isActive = active === tab.id;
            return (
              <button
                key={tab.id}
                onClick={() => setSettingsTab(tab.id)}
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
          {active === "system" && <SystemTab />}
          {active === "plugins" && <PluginsTab />}
          {active === "mcp" && <McpTab />}
          {active === "extensions" && <ExtensionsTab />}
          {active === "updates" && <UpdatesTab />}
          {active === "help" && <HelpTab />}
        </ErrorBoundary>
      </div>
    </div>
  );
}
