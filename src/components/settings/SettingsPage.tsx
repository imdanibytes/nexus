import { useTranslation } from "react-i18next";
import { useAppStore } from "../../stores/appStore";
import { useNotificationCount } from "../../stores/notificationStore";
import { GeneralTab } from "./GeneralTab";
import { SystemTab } from "./SystemTab";
import { PluginsTab } from "./PluginsTab";
import { SecurityTab } from "./SecurityTab";
import { McpTab } from "./McpTab";
import { ExtensionsTab } from "./ExtensionsTab";
import { UpdatesTab } from "./UpdatesTab";
import { HelpTab } from "./HelpTab";
import { Settings, Monitor, Puzzle, ShieldCheck, Cpu, Blocks, ArrowUpCircle, HelpCircle } from "lucide-react";
import { ErrorBoundary } from "../ErrorBoundary";

type SettingsTab = "general" | "system" | "plugins" | "security" | "mcp" | "extensions" | "updates" | "help";

const TABS: { id: SettingsTab; labelKey: string; icon: typeof Settings }[] = [
  { id: "general", labelKey: "tabs.general", icon: Settings },
  { id: "system", labelKey: "tabs.system", icon: Monitor },
  { id: "plugins", labelKey: "tabs.plugins", icon: Puzzle },
  { id: "security", labelKey: "tabs.security", icon: ShieldCheck },
  { id: "mcp", labelKey: "tabs.mcp", icon: Cpu },
  { id: "extensions", labelKey: "tabs.extensions", icon: Blocks },
  { id: "updates", labelKey: "tabs.updates", icon: ArrowUpCircle },
  { id: "help", labelKey: "tabs.help", icon: HelpCircle },
];

const TAB_IDS = new Set<string>(TABS.map((t) => t.id));

/** Map tab IDs to notification category prefixes */
const TAB_NOTIFICATION_PREFIX: Partial<Record<SettingsTab, string>> = {
  updates: "updates",
  system: "system",
  general: "updates.app",
};

// Map old persisted tab IDs to their new homes so bookmarks/deep links still work
const TAB_REDIRECTS: Record<string, SettingsTab> = {
  runtime: "system",
  resources: "system",
  permissions: "security",
  notifications: "general",
};

function TabDot({ tabId }: { tabId: SettingsTab }) {
  const prefix = TAB_NOTIFICATION_PREFIX[tabId];
  const count = useNotificationCount(prefix);
  if (!prefix || count === 0) return null;
  return <span className="w-1.5 h-1.5 rounded-full bg-nx-accent" />;
}

export function SettingsPage() {
  const { t } = useTranslation("settings");
  const { settingsTab, setSettingsTab } = useAppStore();
  const resolved = TAB_REDIRECTS[settingsTab] ?? settingsTab;
  const active = (TAB_IDS.has(resolved) ? resolved : "general") as SettingsTab;

  return (
    <div className="flex flex-col h-full">
      {/* Header + tab strip */}
      <div className="flex-shrink-0 border-b border-nx-border bg-nx-deep/60">
        <div className="px-6 pt-5 pb-0">
          <h2 className="text-[15px] font-bold text-nx-text">{t("title")}</h2>
          <p className="text-[11px] text-nx-text-ghost mt-0.5 mb-3">
            {t("subtitle")}
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
                {t(tab.labelKey)}
                <TabDot tabId={tab.id} />
              </button>
            );
          })}
        </div>
      </div>

      {/* Content pane */}
      <div className="flex-1 overflow-y-auto p-6">
        <ErrorBoundary label={TABS.find((tab) => tab.id === active)?.labelKey ? t(TABS.find((tab) => tab.id === active)!.labelKey) : undefined}>
          {active === "general" && <GeneralTab />}
          {active === "system" && <SystemTab />}
          {active === "plugins" && <PluginsTab />}
          {active === "security" && <SecurityTab />}
          {active === "mcp" && <McpTab />}
          {active === "extensions" && <ExtensionsTab />}
          {active === "updates" && <UpdatesTab />}
          {active === "help" && <HelpTab />}
        </ErrorBoundary>
      </div>
    </div>
  );
}
