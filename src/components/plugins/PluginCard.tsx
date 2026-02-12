import type { InstalledPlugin, RegistryEntry } from "../../types/plugin";
import type { PluginStatus } from "../../types/plugin";

const statusBadge: Record<
  PluginStatus,
  { label: string; className: string }
> = {
  running: { label: "Running", className: "bg-green-500/20 text-green-400" },
  stopped: { label: "Stopped", className: "bg-slate-500/20 text-slate-400" },
  error: { label: "Error", className: "bg-red-500/20 text-red-400" },
  installing: {
    label: "Installing",
    className: "bg-yellow-500/20 text-yellow-400",
  },
};

interface InstalledPluginCardProps {
  plugin: InstalledPlugin;
  onSelect: () => void;
  isSelected: boolean;
}

export function InstalledPluginCard({
  plugin,
  onSelect,
  isSelected,
}: InstalledPluginCardProps) {
  const badge = statusBadge[plugin.status];

  return (
    <div
      onClick={onSelect}
      className={`p-4 rounded-xl border cursor-pointer transition-all ${
        isSelected
          ? "border-indigo-500 bg-slate-700/50"
          : "border-slate-700 bg-slate-800 hover:border-slate-600"
      }`}
    >
      <div className="flex items-start justify-between mb-2">
        <div>
          <h3 className="text-sm font-semibold text-white">
            {plugin.manifest.name}
          </h3>
          <p className="text-xs text-slate-400">v{plugin.manifest.version}</p>
        </div>
        <span
          className={`text-xs px-2 py-0.5 rounded-full font-medium ${badge.className}`}
        >
          {badge.label}
        </span>
      </div>
      <p className="text-xs text-slate-400 line-clamp-2">
        {plugin.manifest.description}
      </p>
    </div>
  );
}

interface RegistryPluginCardProps {
  entry: RegistryEntry;
  onSelect: () => void;
  isInstalled: boolean;
}

export function RegistryPluginCard({
  entry,
  onSelect,
  isInstalled,
}: RegistryPluginCardProps) {
  return (
    <div
      onClick={onSelect}
      className="p-4 rounded-xl border border-slate-700 bg-slate-800 hover:border-slate-600 cursor-pointer transition-all"
    >
      <div className="flex items-start justify-between mb-2">
        <div>
          <h3 className="text-sm font-semibold text-white">{entry.name}</h3>
          <p className="text-xs text-slate-400">v{entry.version}</p>
        </div>
        {isInstalled && (
          <span className="text-xs px-2 py-0.5 rounded-full font-medium bg-indigo-500/20 text-indigo-400">
            Installed
          </span>
        )}
      </div>
      <p className="text-xs text-slate-400 line-clamp-2">
        {entry.description}
      </p>
      {entry.categories.length > 0 && (
        <div className="flex gap-1.5 mt-2">
          {entry.categories.map((cat) => (
            <span
              key={cat}
              className="text-xs px-1.5 py-0.5 rounded bg-slate-700 text-slate-300"
            >
              {cat}
            </span>
          ))}
        </div>
      )}
    </div>
  );
}
