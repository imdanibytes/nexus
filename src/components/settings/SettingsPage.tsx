import { useTranslation } from "react-i18next";
import { useAppStore } from "../../stores/appStore";
import { useNotificationCount } from "../../stores/appStore";
import { GeneralTab } from "./GeneralTab";
import { SystemTab } from "./SystemTab";
import { PluginsTab } from "./PluginsTab";
import { SecurityTab } from "./SecurityTab";
import { McpTab } from "./McpTab";
import { ExtensionsTab } from "./ExtensionsTab";
import { UpdatesTab } from "./UpdatesTab";
import { HelpTab } from "./HelpTab";
import { AuditTab } from "./AuditTab";
import {
  Settings,
  Monitor,
  Puzzle,
  ShieldCheck,
  Cpu,
  Blocks,
  ArrowUpCircle,
  HelpCircle,
  ScrollText,
} from "lucide-react";
import { ErrorBoundary } from "../ErrorBoundary";
import { SettingsShell } from "@imdanibytes/nexus-ui";
import type { SettingsTab as SettingsTabDef } from "@imdanibytes/nexus-ui";

type SettingsTab =
  | "general"
  | "system"
  | "plugins"
  | "security"
  | "mcp"
  | "extensions"
  | "updates"
  | "audit"
  | "help";

const TAB_DEFS: { id: SettingsTab; labelKey: string; icon: typeof Settings }[] = [
  { id: "general", labelKey: "tabs.general", icon: Settings },
  { id: "system", labelKey: "tabs.system", icon: Monitor },
  { id: "plugins", labelKey: "tabs.plugins", icon: Puzzle },
  { id: "security", labelKey: "tabs.security", icon: ShieldCheck },
  { id: "mcp", labelKey: "tabs.mcp", icon: Cpu },
  { id: "extensions", labelKey: "tabs.extensions", icon: Blocks },
  { id: "updates", labelKey: "tabs.updates", icon: ArrowUpCircle },
  { id: "audit", labelKey: "tabs.audit", icon: ScrollText },
  { id: "help", labelKey: "tabs.help", icon: HelpCircle },
];

const TAB_IDS = new Set<string>(TAB_DEFS.map((t) => t.id));

const TAB_NOTIFICATION_PREFIX: Partial<Record<SettingsTab, string>> = {
  updates: "updates",
  system: "system",
  general: "updates.app",
};

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
  return <span className="h-2 w-2 rounded-full bg-primary" />;
}

const TAB_COMPONENTS: Record<SettingsTab, React.FC> = {
  general: GeneralTab,
  system: SystemTab,
  plugins: PluginsTab,
  security: SecurityTab,
  mcp: McpTab,
  extensions: ExtensionsTab,
  updates: UpdatesTab,
  audit: AuditTab,
  help: HelpTab,
};

export function SettingsPage() {
  const { t } = useTranslation("settings");
  const settingsTab = useAppStore((s) => s.settingsTab);
  const setSettingsTab = useAppStore((s) => s.setSettingsTab);
  const resolved = TAB_REDIRECTS[settingsTab] ?? settingsTab;
  const active = (TAB_IDS.has(resolved) ? resolved : "general") as SettingsTab;
  const ActiveComponent = TAB_COMPONENTS[active];

  // Translate tab labels for SettingsShell
  const tabs: SettingsTabDef[] = TAB_DEFS.map((tab) => ({
    id: tab.id,
    label: t(tab.labelKey),
    icon: tab.icon,
  }));

  return (
    <SettingsShell
      tabs={tabs}
      activeTab={active}
      onTabChange={(id) => setSettingsTab(id)}
      variant="panel"
      navHeader={
        <div className="px-3 mb-4">
          <h2 className="text-lg font-bold mb-1">{t("title")}</h2>
          <p className="text-xs text-default-400">{t("subtitle")}</p>
        </div>
      }
      tabBadge={(tabId) => <TabDot tabId={tabId as SettingsTab} />}
    >
      <ErrorBoundary
        label={
          TAB_DEFS.find((tab) => tab.id === active)?.labelKey
            ? t(TAB_DEFS.find((tab) => tab.id === active)!.labelKey)
            : undefined
        }
      >
        <ActiveComponent />
      </ErrorBoundary>
    </SettingsShell>
  );
}
