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
import {
  Settings,
  Monitor,
  Puzzle,
  ShieldCheck,
  Cpu,
  Blocks,
  ArrowUpCircle,
  HelpCircle,
} from "lucide-react";
import { ErrorBoundary } from "../ErrorBoundary";
import { cn } from "@imdanibytes/nexus-ui";
import { motion, AnimatePresence } from "framer-motion";

type SettingsTab =
  | "general"
  | "system"
  | "plugins"
  | "security"
  | "mcp"
  | "extensions"
  | "updates"
  | "help";

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
  help: HelpTab,
};

export function SettingsPage() {
  const { t } = useTranslation("settings");
  const { settingsTab, setSettingsTab } = useAppStore();
  const resolved = TAB_REDIRECTS[settingsTab] ?? settingsTab;
  const active = (TAB_IDS.has(resolved) ? resolved : "general") as SettingsTab;
  const ActiveComponent = TAB_COMPONENTS[active];

  return (
    <div className="flex h-full gap-3 p-3">
      {/* Nav surface */}
      <div className="w-[200px] flex-shrink-0 rounded-xl bg-default-50/40 backdrop-blur-xl border border-white/5 p-4">
        <h2 className="text-lg font-bold mb-1 px-3">{t("title")}</h2>
        <p className="text-xs text-default-400 mb-4 px-3">{t("subtitle")}</p>

        <nav className="space-y-0.5">
          {TABS.map((tab) => {
            const Icon = tab.icon;
            const isActive = tab.id === active;
            return (
              <button
                key={tab.id}
                onClick={() => setSettingsTab(tab.id)}
                className={cn(
                  "relative w-full flex items-center gap-3 px-3 py-2 rounded-xl text-sm text-left transition-colors duration-200",
                  isActive
                    ? "text-foreground font-medium"
                    : "text-default-500 hover:text-foreground hover:bg-default-50",
                )}
              >
                {isActive && (
                  <motion.div
                    layoutId="settings-nav"
                    className="absolute inset-0 rounded-xl bg-default-100"
                    transition={{ type: "spring", bounce: 0.15, duration: 0.4 }}
                  />
                )}
                <span className="relative flex items-center gap-3 w-full">
                  <Icon size={16} />
                  <span className="flex-1">{t(tab.labelKey)}</span>
                  <TabDot tabId={tab.id} />
                </span>
              </button>
            );
          })}
        </nav>
      </div>

      {/* Content surface — animated transitions */}
      <div className="flex-1 rounded-xl bg-default-50/40 backdrop-blur-xl border border-white/5 overflow-y-auto p-8">
        <AnimatePresence mode="wait">
          <motion.div
            key={active}
            initial={{ opacity: 0, y: 8 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -8 }}
            transition={{ duration: 0.2, ease: "easeOut" }}
          >
            <ErrorBoundary
              label={
                TABS.find((tab) => tab.id === active)?.labelKey
                  ? t(TABS.find((tab) => tab.id === active)!.labelKey)
                  : undefined
              }
            >
              <ActiveComponent />
            </ErrorBoundary>
          </motion.div>
        </AnimatePresence>
      </div>
    </div>
  );
}
