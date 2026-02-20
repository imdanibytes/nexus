import { useCallback, useEffect, type FC } from "react";
import { XIcon } from "lucide-react";
import { LazyMotion, domAnimation, m, AnimatePresence } from "framer-motion";
import { cn } from "../lib/utils";

export interface SettingsTab {
  id: string;
  label: string;
  icon: React.ComponentType<{ size?: number; strokeWidth?: number }>;
}

export interface SettingsShellProps {
  tabs: SettingsTab[];
  activeTab: string;
  onTabChange: (tabId: string) => void;
  children: React.ReactNode;
  variant?: "modal" | "panel";
  onClose?: () => void;
  navHeader?: React.ReactNode;
  navFooter?: React.ReactNode;
  tabBadge?: (tabId: string) => React.ReactNode;
  className?: string;
}

const NavContent: FC<{
  tabs: SettingsTab[];
  activeTab: string;
  onTabChange: (tabId: string) => void;
  onClose?: () => void;
  navHeader?: React.ReactNode;
  navFooter?: React.ReactNode;
  tabBadge?: (tabId: string) => React.ReactNode;
  variant: "modal" | "panel";
}> = ({ tabs, activeTab, onTabChange, onClose, navHeader, navFooter, tabBadge, variant }) => (
  <nav className="w-[200px] shrink-0 nx-glass p-4 flex flex-col gap-1">
    {navHeader}
    {variant === "modal" && onClose && !navHeader && (
      <div className="flex items-center justify-between mb-3">
        <h2 className="text-sm font-semibold text-foreground">Settings</h2>
        <button
          onClick={onClose}
          className="p-1 rounded hover:bg-default-200/40 transition-colors text-default-400 hover:text-default-900"
        >
          <XIcon className="size-4" />
        </button>
      </div>
    )}
    {tabs.map((tab) => {
      const Icon = tab.icon;
      const isActive = activeTab === tab.id;
      return (
        <button
          key={tab.id}
          onClick={() => onTabChange(tab.id)}
          className={cn(
            "relative w-full flex items-center gap-3 px-3 py-2 rounded-xl text-sm text-left transition-colors duration-200",
            isActive
              ? "text-foreground font-medium"
              : "text-default-500 hover:text-foreground hover:bg-default-200/40",
          )}
        >
          {isActive && (
            <m.div
              layoutId="settings-nav"
              className="absolute inset-0 rounded-xl bg-default-100"
              transition={{ type: "spring", bounce: 0.15, duration: 0.4 }}
            />
          )}
          <span className="relative flex items-center gap-3 w-full">
            <Icon size={15} strokeWidth={1.5} />
            <span className="flex-1">{tab.label}</span>
            {tabBadge?.(tab.id)}
          </span>
        </button>
      );
    })}
    {navFooter && <div className="mt-auto pt-2">{navFooter}</div>}
  </nav>
);

const ContentPanel: FC<{ activeTab: string; children: React.ReactNode }> = ({
  activeTab,
  children,
}) => (
  <div className="flex-1 min-h-0 nx-glass overflow-y-auto p-8">
    <AnimatePresence mode="wait">
      <m.div
        key={activeTab}
        initial={{ opacity: 0, y: 8 }}
        animate={{ opacity: 1, y: 0 }}
        exit={{ opacity: 0, y: -8 }}
        transition={{ duration: 0.2, ease: "easeOut" }}
      >
        {children}
      </m.div>
    </AnimatePresence>
  </div>
);

export const SettingsShell: FC<SettingsShellProps> = ({
  tabs,
  activeTab,
  onTabChange,
  children,
  variant = "panel",
  onClose,
  navHeader,
  navFooter,
  tabBadge,
  className,
}) => {
  // Escape key → close (modal only)
  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if (variant === "modal" && e.key === "Escape") onClose?.();
    },
    [variant, onClose],
  );

  useEffect(() => {
    if (variant !== "modal") return;
    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [variant, handleKeyDown]);

  const navContent = (
    <NavContent
      tabs={tabs}
      activeTab={activeTab}
      onTabChange={onTabChange}
      onClose={variant === "modal" ? onClose : undefined}
      navHeader={navHeader}
      navFooter={navFooter}
      tabBadge={tabBadge}
      variant={variant}
    />
  );

  const contentPanel = (
    <ContentPanel activeTab={activeTab}>{children}</ContentPanel>
  );

  // ── Panel variant ──
  if (variant === "panel") {
    return (
      <LazyMotion features={domAnimation}>
        <div className={cn("flex h-full gap-3 p-3", className)}>
          {navContent}
          {contentPanel}
        </div>
      </LazyMotion>
    );
  }

  // ── Modal variant ──
  return (
    <LazyMotion features={domAnimation}>
      <AnimatePresence>
        <m.div
          className={cn("absolute inset-0 z-50 flex items-center justify-center", className)}
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          transition={{ duration: 0.2 }}
        >
          {/* Backdrop */}
          <div
            className="absolute inset-0 bg-black/30 dark:bg-black/40 backdrop-blur-sm"
            onClick={onClose}
          />

          {/* Modal */}
          <m.div
            className="relative z-10 flex h-[85vh] w-[min(90vw,56rem)] gap-2 p-2"
            initial={{ opacity: 0, scale: 0.96, y: 12 }}
            animate={{ opacity: 1, scale: 1, y: 0 }}
            exit={{ opacity: 0, scale: 0.96, y: 12 }}
            transition={{ duration: 0.25, ease: "easeOut" }}
          >
            {navContent}
            {contentPanel}
          </m.div>
        </m.div>
      </AnimatePresence>
    </LazyMotion>
  );
};
