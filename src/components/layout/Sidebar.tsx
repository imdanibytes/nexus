import { useAppStore } from "../../stores/appStore";
import type { InstalledPlugin } from "../../types/plugin";
import { Plus, Settings } from "lucide-react";

const statusColor: Record<string, string> = {
  running: "bg-nx-success",
  stopped: "bg-nx-text-muted",
  error: "bg-nx-error",
  installing: "bg-nx-warning",
};

function PluginItem({ plugin }: { plugin: InstalledPlugin }) {
  const { selectedPluginId, selectPlugin, setView } = useAppStore();
  const isSelected = selectedPluginId === plugin.manifest.id;
  const isRunning = plugin.status === "running";

  return (
    <button
      onClick={() => {
        selectPlugin(plugin.manifest.id);
        setView("plugins");
      }}
      className={`w-full flex items-center gap-3 px-3 py-2 rounded-[var(--radius-button)] text-left transition-all duration-150 ${
        isSelected
          ? "bg-nx-accent-muted text-nx-accent"
          : "text-nx-text-secondary hover:bg-nx-overlay hover:text-nx-text"
      }`}
    >
      <span
        className={`w-1.5 h-1.5 rounded-full shrink-0 ${statusColor[plugin.status] ?? "bg-nx-text-muted"}`}
        style={isRunning ? { animation: "pulse-status 2s ease-in-out infinite" } : undefined}
      />
      <span className="truncate text-[12px] font-medium">{plugin.manifest.name}</span>
    </button>
  );
}

export function Sidebar() {
  const { currentView, setView, installedPlugins } = useAppStore();

  return (
    <aside
      className="w-60 border-r border-nx-border flex flex-col h-full"
      style={{
        background: "rgba(34, 38, 49, 0.85)",
        backdropFilter: "blur(12px)",
        WebkitBackdropFilter: "blur(12px)",
      }}
    >
      {/* Logo */}
      <div className="px-4 py-4 border-b border-nx-border-subtle">
        <h1 className="text-[15px] font-bold tracking-tight">
          <span className="text-nx-accent">Nexus</span>
        </h1>
        <p className="text-[10px] text-nx-text-muted font-medium tracking-wide uppercase mt-0.5">
          Plugin Dashboard
        </p>
      </div>

      {/* Installed plugins */}
      <div className="flex-1 overflow-y-auto px-3 py-3">
        <h2 className="text-[10px] font-semibold text-nx-text-muted uppercase tracking-wider px-3 mb-2">
          Installed
        </h2>
        {installedPlugins.length === 0 ? (
          <p className="text-[11px] text-nx-text-ghost px-3 py-2">
            No plugins installed
          </p>
        ) : (
          <div className="space-y-0.5">
            {installedPlugins.map((plugin) => (
              <PluginItem key={plugin.manifest.id} plugin={plugin} />
            ))}
          </div>
        )}
      </div>

      {/* Navigation */}
      <nav className="px-3 py-3 border-t border-nx-border-subtle space-y-0.5">
        <button
          onClick={() => setView("marketplace")}
          className={`w-full flex items-center gap-3 px-3 py-2 rounded-[var(--radius-button)] text-[12px] font-medium transition-all duration-150 ${
            currentView === "marketplace" || currentView === "plugin-detail"
              ? "bg-nx-accent-muted text-nx-accent"
              : "text-nx-text-secondary hover:bg-nx-overlay hover:text-nx-text"
          }`}
        >
          <Plus size={15} strokeWidth={1.5} />
          Add Plugins
        </button>
        <button
          onClick={() => setView("settings")}
          className={`w-full flex items-center gap-3 px-3 py-2 rounded-[var(--radius-button)] text-[12px] font-medium transition-all duration-150 ${
            currentView === "settings"
              ? "bg-nx-accent-muted text-nx-accent"
              : "text-nx-text-secondary hover:bg-nx-overlay hover:text-nx-text"
          }`}
        >
          <Settings size={15} strokeWidth={1.5} />
          Settings
        </button>
      </nav>
    </aside>
  );
}
