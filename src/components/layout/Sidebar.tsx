import { useAppStore } from "../../stores/appStore";
import type { InstalledPlugin } from "../../types/plugin";

const statusColor: Record<string, string> = {
  running: "bg-green-500",
  stopped: "bg-slate-400",
  error: "bg-red-500",
  installing: "bg-yellow-500",
};

function PluginItem({ plugin }: { plugin: InstalledPlugin }) {
  const { selectedPluginId, selectPlugin, setView } = useAppStore();
  const isSelected = selectedPluginId === plugin.manifest.id;

  return (
    <button
      onClick={() => {
        selectPlugin(plugin.manifest.id);
        setView("plugins");
      }}
      className={`w-full flex items-center gap-3 px-3 py-2 rounded-lg text-left transition-colors ${
        isSelected
          ? "bg-slate-700 text-white"
          : "text-slate-300 hover:bg-slate-700/50 hover:text-white"
      }`}
    >
      <span
        className={`w-2 h-2 rounded-full shrink-0 ${statusColor[plugin.status] ?? "bg-slate-400"}`}
      />
      <span className="truncate text-sm">{plugin.manifest.name}</span>
    </button>
  );
}

export function Sidebar() {
  const { currentView, setView, installedPlugins } = useAppStore();

  return (
    <aside className="w-64 bg-slate-800 border-r border-slate-700 flex flex-col h-full">
      {/* Logo */}
      <div className="px-5 py-4 border-b border-slate-700">
        <h1 className="text-lg font-bold text-white tracking-tight">Nexus</h1>
        <p className="text-xs text-slate-400">Plugin Dashboard</p>
      </div>

      {/* Installed plugins */}
      <div className="flex-1 overflow-y-auto px-3 py-3">
        <h2 className="text-xs font-semibold text-slate-400 uppercase tracking-wider px-3 mb-2">
          Installed
        </h2>
        {installedPlugins.length === 0 ? (
          <p className="text-xs text-slate-500 px-3 py-2">
            No plugins installed
          </p>
        ) : (
          <div className="space-y-1">
            {installedPlugins.map((plugin) => (
              <PluginItem key={plugin.manifest.id} plugin={plugin} />
            ))}
          </div>
        )}
      </div>

      {/* Navigation */}
      <nav className="px-3 py-3 border-t border-slate-700 space-y-1">
        <button
          onClick={() => setView("marketplace")}
          className={`w-full flex items-center gap-3 px-3 py-2 rounded-lg text-sm transition-colors ${
            currentView === "marketplace" || currentView === "plugin-detail"
              ? "bg-indigo-500/20 text-indigo-400"
              : "text-slate-300 hover:bg-slate-700/50 hover:text-white"
          }`}
        >
          <svg
            className="w-4 h-4"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M12 4v16m8-8H4"
            />
          </svg>
          Add Plugins
        </button>
        <button
          onClick={() => setView("settings")}
          className={`w-full flex items-center gap-3 px-3 py-2 rounded-lg text-sm transition-colors ${
            currentView === "settings"
              ? "bg-indigo-500/20 text-indigo-400"
              : "text-slate-300 hover:bg-slate-700/50 hover:text-white"
          }`}
        >
          <svg
            className="w-4 h-4"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.066 2.573c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.573 1.066c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.066-2.573c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"
            />
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"
            />
          </svg>
          Settings
        </button>
      </nav>
    </aside>
  );
}
