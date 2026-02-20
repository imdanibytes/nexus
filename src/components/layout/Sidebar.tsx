import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useAppStore } from "../../stores/appStore";
import { useNotificationCount } from "../../stores/appStore";
import { usePluginActions } from "../../hooks/usePlugins";
import type { InstalledPlugin } from "../../types/plugin";
import type { ExtensionStatus } from "../../types/extension";
import * as api from "../../lib/tauri";
import {
  Settings,
  ArrowUp,
  Play,
  Square,
  ScrollText,
  Trash2,
  Hammer,
  Wrench,
  MoreHorizontal,
  TriangleAlert,
  Power,
  Puzzle,
  Store,
  Blocks,
  PanelLeftClose,
  PanelLeftOpen,
} from "lucide-react";
import {
  Button,
  Dropdown,
  DropdownTrigger,
  DropdownMenu,
  DropdownItem,
  DropdownSection,
  Modal,
  ModalContent,
  ModalHeader,
  ModalBody,
  ModalFooter,
  Tooltip,
  useDisclosure,
} from "@heroui/react";
import { cn } from "@imdanibytes/nexus-ui";
import { NexusLogo } from "../brand/NexusLogo";
import { LazyMotion, domAnimation, m } from "framer-motion";

/* ─── Status colors ─── */
const statusColor: Record<string, string> = {
  running: "bg-success",
  stopped: "bg-default-400",
  error: "bg-danger",
  installing: "bg-warning",
};

/* ─── Nav item ─── */
function NavItem({
  isActive,
  onClick,
  children,
  className,
  collapsed,
  tooltip,
}: {
  isActive?: boolean;
  onClick?: () => void;
  children: React.ReactNode;
  className?: string;
  collapsed?: boolean;
  tooltip?: string;
}) {
  const button = (
    <button
      onClick={onClick}
      className={cn(
        "relative flex items-center w-full rounded-xl text-sm transition-all duration-300",
        collapsed ? "px-0 py-2 gap-0 justify-center" : "px-3 py-2 gap-3",
        isActive
          ? "text-foreground font-medium"
          : "text-default-500 hover:text-foreground hover:bg-default-50",
        className,
      )}
    >
      {isActive && (
        <div className={cn("absolute inset-0 rounded-xl", collapsed ? "bg-primary/15" : "bg-default-100")} />
      )}
      <span className={cn("relative flex items-center w-full transition-all duration-300", collapsed ? "gap-0 justify-center" : "gap-3")}>
        {children}
      </span>
    </button>
  );

  if (tooltip) {
    return (
      <Tooltip content={tooltip} placement="right" delay={300} closeDelay={0} isDisabled={!collapsed}>
        {button}
      </Tooltip>
    );
  }

  return button;
}

/* ─── Surface layer — each section is its own card ─── */
function Surface({ children, className }: { children: React.ReactNode; className?: string }) {
  return (
    <div className={cn("rounded-xl bg-default-50/40 backdrop-blur-xl border border-default-200/50 p-2", className)}>
      {children}
    </div>
  );
}

/* ─── Plugin row ─── */
function PluginItem({ plugin, collapsed }: { plugin: InstalledPlugin; collapsed?: boolean }) {
  const { t } = useTranslation(["common", "plugins"]);
  const id = plugin.manifest.id;

  // Surgical selectors — only re-render when THIS data changes
  const selectedPluginId = useAppStore((s) => s.selectedPluginId);
  const isBusy = useAppStore((s) => !!s.busyPlugins[id]);
  const isWarm = useAppStore((s) => !!s.warmViewports[id]);
  const availableUpdates = useAppStore((s) => s.availableUpdates);

  const { start, stop, remove, rebuild, toggleDevMode } = usePluginActions();
  const isSelected = selectedPluginId === id;
  const isRunning = plugin.status === "running";
  const update = availableUpdates.find((u) => u.item_id === id);
  const hasUpdate = !!update;
  const isLocal = !!plugin.local_manifest_path;
  const [menuOpen, setMenuOpen] = useState(false);
  const removeModal = useDisclosure();

  const handleStart = () => start(id);
  const handleStop = () => stop(id);
  const handleRebuild = () => rebuild(id);
  const handleToggleDevMode = () => toggleDevMode(id, !plugin.dev_mode);
  const handleRemove = () => {
    removeModal.onClose();
    remove(id);
  };

  async function handleUpdate() {
    if (!update) return;
    if (update.security.includes("key_changed")) {
      const s = useAppStore.getState();
      s.setSettingsTab("updates");
      s.setView("settings");
      return;
    }
    try {
      await api.updatePlugin(
        update.manifest_url,
        update.new_image_digest,
        update.build_context,
      );
      const { availableUpdates: current, setAvailableUpdates } = useAppStore.getState();
      setAvailableUpdates(current.filter((u) => u.item_id !== id));
    } catch {
      // Lifecycle events handle busy state and error toasts
    }
  }

  const statusDot = (
    <span className="flex items-center justify-center w-4 shrink-0">
      <span className="relative flex h-2 w-2">
        <span
          className={cn(
            "absolute inline-flex h-full w-full rounded-full",
            statusColor[plugin.status] ?? "bg-default-400",
            isRunning && !isWarm && "animate-ping opacity-75",
          )}
        />
        <span
          className={cn(
            "relative inline-flex rounded-full h-2 w-2",
            statusColor[plugin.status] ?? "bg-default-400",
          )}
        />
      </span>
    </span>
  );

  if (collapsed) {
    return (
      <NavItem
        isActive={isSelected}
        onClick={() => {
          const s = useAppStore.getState();
          s.selectPlugin(id);
          s.setView("plugins");
        }}
        collapsed
        tooltip={plugin.manifest.name}
      >
        <span className="relative">
          {plugin.manifest.name.charAt(0).toUpperCase()}
          <span
            className={cn(
              "absolute -bottom-1 -right-2.5 h-2 w-2 rounded-full",
              statusColor[plugin.status] ?? "bg-default-400",
            )}
          />
        </span>
      </NavItem>
    );
  }

  return (
    <div className="group/item relative">
      <NavItem
        isActive={isSelected}
        onClick={() => {
          const s = useAppStore.getState();
          s.selectPlugin(id);
          s.setView("plugins");
        }}
      >
        {statusDot}
        <span className="truncate whitespace-nowrap">{plugin.manifest.name}</span>
        {hasUpdate && (
          <ArrowUp
            size={14}
            strokeWidth={2}
            className="text-primary shrink-0 group-hover/item:opacity-0 transition-opacity"
          />
        )}
      </NavItem>

      {/* Context menu — lazy: Dropdown only mounts when opened */}
      <button
        className="absolute right-2 top-1/2 -translate-y-1/2 opacity-0 group-hover/item:opacity-100 focus:opacity-100 p-1 rounded-lg text-default-400 hover:text-foreground transition-all"
        onClick={(e) => { e.stopPropagation(); setMenuOpen(true); }}
      >
        <MoreHorizontal size={14} />
      </button>
      {menuOpen && (
        <Dropdown isOpen onOpenChange={(open) => { if (!open) setMenuOpen(false); }}>
          <DropdownTrigger>
            <span className="sr-only">menu</span>
          </DropdownTrigger>
          <DropdownMenu aria-label="Plugin actions">
            {hasUpdate ? (
              <DropdownSection showDivider>
                <DropdownItem
                  key="update"
                  onPress={handleUpdate}
                  isDisabled={isBusy}
                  startContent={<ArrowUp size={14} className="text-primary" />}
                >
                  {t("plugins:menu.update")}
                </DropdownItem>
              </DropdownSection>
            ) : (
              <DropdownSection className="hidden">
                <DropdownItem key="noop">-</DropdownItem>
              </DropdownSection>
            )}
            <DropdownSection showDivider>
              {isRunning ? (
                <DropdownItem
                  key="stop"
                  onPress={handleStop}
                  isDisabled={isBusy}
                  startContent={<Square size={14} className="text-warning" />}
                >
                  {t("common:action.stop")}
                </DropdownItem>
              ) : (
                <DropdownItem
                  key="start"
                  onPress={handleStart}
                  isDisabled={isBusy}
                  startContent={<Play size={14} className="text-success" />}
                >
                  {t("common:action.start")}
                </DropdownItem>
              )}
              <DropdownItem
                key="logs"
                onPress={() => useAppStore.getState().setShowLogs(id)}
                startContent={<ScrollText size={14} />}
              >
                {t("plugins:menu.logs")}
              </DropdownItem>
            </DropdownSection>
            {isLocal ? (
              <DropdownSection showDivider>
                <DropdownItem
                  key="rebuild"
                  onPress={handleRebuild}
                  isDisabled={isBusy}
                  startContent={<Hammer size={14} className="text-primary" />}
                >
                  {t("plugins:menu.rebuild")}
                </DropdownItem>
                <DropdownItem
                  key="devmode"
                  onPress={handleToggleDevMode}
                  isDisabled={isBusy}
                  startContent={<Wrench size={14} />}
                >
                  {plugin.dev_mode
                    ? t("plugins:menu.disableDevMode")
                    : t("plugins:menu.enableDevMode")}
                </DropdownItem>
              </DropdownSection>
            ) : (
              <DropdownSection className="hidden">
                <DropdownItem key="noop2">-</DropdownItem>
              </DropdownSection>
            )}
            <DropdownSection>
              <DropdownItem
                key="remove"
                onPress={removeModal.onOpen}
                isDisabled={isBusy}
                className="text-danger"
                color="danger"
                startContent={<Trash2 size={14} />}
              >
                {t("common:action.remove")}
              </DropdownItem>
            </DropdownSection>
          </DropdownMenu>
        </Dropdown>
      )}

      {/* Remove confirmation modal */}
      <Modal isOpen={removeModal.isOpen} onOpenChange={removeModal.onOpenChange}>
        <ModalContent>
          {(onClose) => (
            <>
              <ModalHeader>
                <div className="flex items-center gap-2">
                  <TriangleAlert size={18} className="text-warning" />
                  {t("common:confirm.removePlugin", {
                    name: plugin.manifest.name,
                  })}
                </div>
              </ModalHeader>
              <ModalBody>
                <p className="text-default-500">
                  {t("common:confirm.removePluginDesc")}
                </p>
              </ModalBody>
              <ModalFooter>
                <Button variant="flat" onPress={onClose}>
                  {t("common:action.cancel")}
                </Button>
                <Button color="danger" onPress={handleRemove}>
                  {t("common:confirm.removeAndDeleteData")}
                </Button>
              </ModalFooter>
            </>
          )}
        </ModalContent>
      </Modal>
    </div>
  );
}

/* ─── Extension row ─── */
function ExtensionItem({ ext, collapsed }: { ext: ExtensionStatus; collapsed?: boolean }) {
  const { t } = useTranslation(["common", "plugins"]);
  const isBusy = useAppStore((s) => !!s.busyExtensions[ext.id]);
  const availableUpdates = useAppStore((s) => s.availableUpdates);
  const update = availableUpdates.find((u) => u.item_id === ext.id);
  const hasUpdate = !!update;
  const [menuOpen, setMenuOpen] = useState(false);
  const removeModal = useDisclosure();

  async function handleExtUpdate() {
    if (!update) return;
    if (update.security.includes("key_changed")) {
      const s = useAppStore.getState();
      s.setSettingsTab("updates");
      s.setView("settings");
      return;
    }
    try {
      await api.updateExtension(update.manifest_url);
      const { availableUpdates: current, setAvailableUpdates } = useAppStore.getState();
      setAvailableUpdates(current.filter((u) => u.item_id !== ext.id));
    } catch {
      // Lifecycle events handle errors
    }
  }

  async function handleToggle() {
    if (ext.enabled) {
      await api.extensionDisable(ext.id);
    } else {
      await api.extensionEnable(ext.id);
    }
  }

  async function handleRemove() {
    removeModal.onClose();
    await api.extensionRemove(ext.id);
  }

  const statusDot = (
    <span className="flex items-center justify-center w-4 shrink-0">
      <span
        className={cn(
          "h-2 w-2 rounded-full",
          ext.enabled ? "bg-success" : "bg-default-400",
        )}
      />
    </span>
  );

  if (collapsed) {
    return (
      <Tooltip content={ext.display_name} placement="right" delay={300} closeDelay={0}>
        <div className="group/item relative">
          <NavItem
            onClick={() => {
              const s = useAppStore.getState();
              s.setSettingsTab("extensions");
              s.setFocusExtensionId(ext.id);
              s.setView("settings");
            }}
            collapsed
          >
            <span className="relative flex items-center justify-center h-7 w-7 rounded-lg bg-default-100 text-xs font-semibold text-default-500 shrink-0">
              {ext.display_name.charAt(0).toUpperCase()}
              <span
                className={cn(
                  "absolute -bottom-0.5 -right-0.5 h-2.5 w-2.5 rounded-full border-2 border-background",
                  ext.enabled ? "bg-success" : "bg-default-400",
                )}
              />
            </span>
          </NavItem>
        </div>
      </Tooltip>
    );
  }

  return (
    <div className="group/item relative">
      <NavItem
        onClick={() => {
          const s = useAppStore.getState();
          s.setSettingsTab("extensions");
          s.setFocusExtensionId(ext.id);
          s.setView("settings");
        }}
      >
        {statusDot}
        <span className="truncate whitespace-nowrap">{ext.display_name}</span>
        {hasUpdate && (
          <ArrowUp
            size={14}
            strokeWidth={2}
            className="text-primary shrink-0 group-hover/item:opacity-0 transition-opacity"
          />
        )}
      </NavItem>

      {/* Context menu — lazy */}
      <button
        className="absolute right-2 top-1/2 -translate-y-1/2 opacity-0 group-hover/item:opacity-100 focus:opacity-100 p-1 rounded-lg text-default-400 hover:text-foreground transition-all"
        onClick={(e) => { e.stopPropagation(); setMenuOpen(true); }}
      >
        <MoreHorizontal size={14} />
      </button>
      {menuOpen && (
        <Dropdown isOpen onOpenChange={(open) => { if (!open) setMenuOpen(false); }}>
          <DropdownTrigger>
            <span className="sr-only">menu</span>
          </DropdownTrigger>
          <DropdownMenu aria-label="Extension actions">
            {hasUpdate ? (
              <DropdownSection showDivider>
                <DropdownItem
                  key="update"
                  onPress={handleExtUpdate}
                  isDisabled={isBusy}
                  startContent={<ArrowUp size={14} className="text-primary" />}
                >
                  {t("plugins:menu.update")}
                </DropdownItem>
              </DropdownSection>
            ) : (
              <DropdownSection className="hidden">
                <DropdownItem key="noop">-</DropdownItem>
              </DropdownSection>
            )}
            <DropdownSection showDivider>
              <DropdownItem
                key="toggle"
                onPress={handleToggle}
                isDisabled={isBusy}
                startContent={
                  <Power
                    size={14}
                    className={ext.enabled ? "text-warning" : "text-success"}
                  />
                }
              >
                {ext.enabled
                  ? t("common:action.disable")
                  : t("common:action.enable")}
              </DropdownItem>
              <DropdownItem
                key="manage"
                onPress={() => {
                  const s = useAppStore.getState();
                  s.setSettingsTab("extensions");
                  s.setView("settings");
                }}
                startContent={<Settings size={14} />}
              >
                {t("plugins:menu.manageExtensions")}
              </DropdownItem>
            </DropdownSection>
            <DropdownSection>
              <DropdownItem
                key="remove"
                onPress={removeModal.onOpen}
                isDisabled={isBusy}
                className="text-danger"
                color="danger"
                startContent={<Trash2 size={14} />}
              >
                {t("common:action.remove")}
            </DropdownItem>
          </DropdownSection>
          </DropdownMenu>
        </Dropdown>
      )}

      <Modal isOpen={removeModal.isOpen} onOpenChange={removeModal.onOpenChange}>
        <ModalContent>
          {(onClose) => (
            <>
              <ModalHeader>
                <div className="flex items-center gap-2">
                  <TriangleAlert size={18} className="text-warning" />
                  {t("common:confirm.removeExtension", {
                    name: ext.display_name,
                  })}
                </div>
              </ModalHeader>
              <ModalBody>
                {ext.consumers.length > 0 ? (
                  <>
                    <p className="text-default-500">
                      {t("common:confirm.removeExtensionConsumers", {
                        count: ext.consumers.length,
                      })}
                    </p>
                    <ul className="mt-3 space-y-2">
                      {ext.consumers.map((c) => (
                        <li
                          key={c.plugin_id}
                          className="flex items-center gap-2 px-3 py-2 rounded-xl bg-default-100"
                        >
                          <Puzzle size={14} className="text-default-400" />
                          <span className="text-sm font-medium truncate">
                            {c.plugin_name}
                          </span>
                        </li>
                      ))}
                    </ul>
                  </>
                ) : (
                  <p className="text-default-500">
                    {t("common:confirm.removeExtensionNoConsumers")}
                  </p>
                )}
              </ModalBody>
              <ModalFooter>
                <Button variant="flat" onPress={onClose}>
                  {t("common:action.cancel")}
                </Button>
                <Button color="danger" onPress={handleRemove}>
                  {t("common:confirm.removeExtensionAction")}
                </Button>
              </ModalFooter>
            </>
          )}
        </ModalContent>
      </Modal>
    </div>
  );
}

/* ─── Main sidebar ─── */
export function AppSidebar() {
  const { t } = useTranslation(["common", "plugins"]);
  const currentView = useAppStore((s) => s.currentView);
  const installedPlugins = useAppStore((s) => s.installedPlugins);
  const installedExtensions = useAppStore((s) => s.installedExtensions);
  const badgeCount = useNotificationCount();

  const [collapsed, setCollapsed] = useState(() => {
    return localStorage.getItem("nexus-sidebar-collapsed") === "true";
  });

  const toggleCollapsed = () => {
    const next = !collapsed;
    setCollapsed(next);
    localStorage.setItem("nexus-sidebar-collapsed", String(next));
  };

  const plugins = installedPlugins.filter((p) => p.manifest.ui !== null);
  const integrations = installedPlugins.filter((p) => p.manifest.ui === null);

  return (
    <LazyMotion features={domAnimation}>
    <m.aside
      animate={{ width: collapsed ? 68 : 240 }}
      transition={{ type: "spring", bounce: 0, duration: 0.3 }}
      className="flex-shrink-0 flex flex-col h-full backdrop-blur-2xl bg-background/40 p-3 gap-2 overflow-hidden"
    >
      <div className="flex flex-col flex-1 gap-2 overflow-hidden">
        {/* Layer 0 — Brand */}
        <Surface className={cn("py-3 flex items-center transition-all duration-300", collapsed ? "px-0 justify-center" : "px-4")}>
          <NexusLogo className="text-foreground" collapsed={collapsed} />
        </Surface>

        {/* Layer 1 — Installed items (scrollable, sectioned) */}
        <Surface className="flex-1 overflow-y-auto">
          {installedPlugins.length === 0 && installedExtensions.length === 0 ? (
            collapsed ? (
              <div className="flex justify-center py-3">
                <span className="h-2 w-2 rounded-full bg-default-300" />
              </div>
            ) : (
              <p className="text-sm text-default-400 px-2 py-3">
                {t("common:empty.noPlugins")}
              </p>
            )
          ) : (
            <div className="space-y-3">
              {/* Plugins (with UI) */}
              {plugins.length > 0 && (
                <div>
                  {!collapsed && (
                    <p className="text-[11px] font-medium text-default-400 uppercase tracking-wider px-3 pb-1">
                      {t("common:nav.plugins")}
                    </p>
                  )}
                  <div className="space-y-0.5">
                    {plugins.map((plugin) => (
                      <PluginItem key={plugin.manifest.id} plugin={plugin} collapsed={collapsed} />
                    ))}
                  </div>
                </div>
              )}

              {/* Headless MCP servers (no UI) */}
              {integrations.length > 0 && (
                <div>
                  {!collapsed && (
                    <p className="text-[11px] font-medium text-default-400 uppercase tracking-wider px-3 pb-1">
                      {t("common:nav.integrations")}
                    </p>
                  )}
                  <div className="space-y-0.5">
                    {integrations.map((plugin) => (
                      <PluginItem key={plugin.manifest.id} plugin={plugin} collapsed={collapsed} />
                    ))}
                  </div>
                </div>
              )}

              {/* Extensions */}
              {installedExtensions.length > 0 && (
                <div>
                  {!collapsed && (
                    <p className="text-[11px] font-medium text-default-400 uppercase tracking-wider px-3 pb-1">
                      {t("common:nav.extensions")}
                    </p>
                  )}
                  <div className="space-y-0.5">
                    {installedExtensions.map((ext) => (
                      <ExtensionItem key={ext.id} ext={ext} collapsed={collapsed} />
                    ))}
                  </div>
                </div>
              )}
            </div>
          )}
        </Surface>

        {/* Layer 2 — Navigation */}
        <Surface className="space-y-0.5">
          <NavItem
            isActive={currentView === "marketplace" || currentView === "plugin-detail"}
            onClick={() => useAppStore.getState().setView("marketplace")}
            collapsed={collapsed}
            tooltip={t("common:nav.addPlugins")}
          >
            <Store size={16} className="shrink-0" />
            <span className={cn("truncate whitespace-nowrap transition-all duration-300", collapsed ? "w-0 opacity-0" : "w-auto opacity-100")}>{t("common:nav.addPlugins")}</span>
          </NavItem>

          <NavItem
            isActive={currentView === "extension-marketplace" || currentView === "extension-detail"}
            onClick={() => useAppStore.getState().setView("extension-marketplace")}
            collapsed={collapsed}
            tooltip={t("common:nav.extensions")}
          >
            <Blocks size={16} className="shrink-0" />
            <span className={cn("truncate whitespace-nowrap transition-all duration-300", collapsed ? "w-0 opacity-0" : "w-auto opacity-100")}>{t("common:nav.extensions")}</span>
          </NavItem>

          <NavItem
            isActive={currentView === "settings"}
            onClick={() => useAppStore.getState().setView("settings")}
            collapsed={collapsed}
            tooltip={t("common:nav.settings")}
          >
            <span className="relative shrink-0">
              <Settings size={16} />
              {badgeCount > 0 && collapsed && (
                <span className="absolute -top-1 -right-1 h-2 w-2 rounded-full bg-primary" />
              )}
            </span>
            <span className={cn("truncate whitespace-nowrap transition-all duration-300", collapsed ? "w-0 opacity-0" : "w-auto opacity-100")}>{t("common:nav.settings")}</span>
            {badgeCount > 0 && !collapsed && (
              <span className="flex h-4 min-w-4 items-center justify-center rounded-full bg-primary text-primary-foreground text-[10px] font-medium px-1 shrink-0">
                {badgeCount}
              </span>
            )}
          </NavItem>
        </Surface>
      </div>

      {/* Collapse toggle */}
      <button
        onClick={toggleCollapsed}
        className="flex items-center justify-center py-1.5 rounded-xl text-default-400 hover:text-foreground hover:bg-default-50 transition-colors"
      >
        {collapsed ? (
          <PanelLeftOpen size={16} />
        ) : (
          <PanelLeftClose size={16} />
        )}
      </button>
    </m.aside>
    </LazyMotion>
  );
}
