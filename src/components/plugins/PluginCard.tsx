import type { InstalledPlugin, RegistryEntry } from "../../types/plugin";
import type { PluginStatus } from "../../types/plugin";
import { timeAgo } from "../../lib/timeAgo";

const statusBadge: Record<
  PluginStatus,
  { label: string; className: string }
> = {
  running: { label: "Running", className: "bg-nx-success-muted text-nx-success" },
  stopped: { label: "Stopped", className: "bg-nx-overlay text-nx-text-muted" },
  error: { label: "Error", className: "bg-nx-error-muted text-nx-error" },
  installing: {
    label: "Installing",
    className: "bg-nx-warning-muted text-nx-warning",
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
      className={`p-4 rounded-[var(--radius-card)] border cursor-pointer transition-all duration-200 ${
        isSelected
          ? "border-nx-border-accent bg-nx-accent-subtle"
          : "border-nx-border bg-nx-surface hover:border-nx-border-strong hover:shadow-[var(--shadow-card-hover)]"
      }`}
    >
      <div className="flex items-start justify-between mb-2">
        <div>
          <h3 className="text-[13px] font-semibold text-nx-text">
            {plugin.manifest.name}
          </h3>
          <p className="text-[11px] text-nx-text-muted font-mono">v{plugin.manifest.version}</p>
        </div>
        <span
          className={`text-[10px] px-2 py-0.5 rounded-[var(--radius-tag)] font-medium ${badge.className}`}
        >
          {badge.label}
        </span>
      </div>
      <p className="text-[11px] text-nx-text-secondary line-clamp-2">
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
      className="p-4 rounded-[var(--radius-card)] border border-nx-border bg-nx-surface hover:border-nx-border-strong hover:shadow-[var(--shadow-card-hover)] cursor-pointer transition-all duration-200"
    >
      <div className="flex items-start justify-between mb-2">
        <div className="flex items-center gap-2.5">
          {entry.icon ? (
            <img
              src={entry.icon}
              alt=""
              className="w-8 h-8 rounded-[var(--radius-button)] object-cover flex-shrink-0"
            />
          ) : (
            <div className="w-8 h-8 rounded-[var(--radius-button)] bg-nx-overlay flex items-center justify-center flex-shrink-0">
              <span className="text-[13px] font-semibold text-nx-text-muted">
                {entry.name.charAt(0)}
              </span>
            </div>
          )}
          <div>
            <h3 className="text-[13px] font-semibold text-nx-text">{entry.name}</h3>
            <p className="text-[11px] text-nx-text-muted font-mono">
              v{entry.version}
              {entry.author_url ? (
                <a
                  href={entry.author_url}
                  target="_blank"
                  rel="noopener noreferrer"
                  onClick={(e) => e.stopPropagation()}
                  className="font-sans ml-1.5 text-nx-accent hover:underline"
                >
                  {entry.author}
                </a>
              ) : (
                <span className="font-sans ml-1.5">by {entry.author}</span>
              )}
            </p>
          </div>
        </div>
        <div className="flex gap-1.5 flex-shrink-0">
          {entry.status === "deprecated" && (
            <span className="text-[10px] px-2 py-0.5 rounded-[var(--radius-tag)] font-medium bg-nx-warning-muted text-nx-warning">
              Deprecated
            </span>
          )}
          {isInstalled && (
            <span className="text-[10px] px-2 py-0.5 rounded-[var(--radius-tag)] font-medium bg-nx-accent-muted text-nx-accent">
              Installed
            </span>
          )}
        </div>
      </div>
      <p className="text-[11px] text-nx-text-secondary line-clamp-2">
        {entry.description}
      </p>
      <div className="flex items-center gap-1.5 mt-2.5 flex-wrap">
        {entry.source && (
          <span className="text-[10px] px-1.5 py-0.5 rounded-[var(--radius-tag)] bg-nx-accent-muted text-nx-accent font-medium">
            {entry.source}
          </span>
        )}
        {entry.categories.map((cat) => (
          <span
            key={cat}
            className="text-[10px] px-1.5 py-0.5 rounded-[var(--radius-tag)] bg-nx-overlay text-nx-text-secondary"
          >
            {cat}
          </span>
        ))}
        {entry.created_at && (
          <span className="text-[10px] text-nx-text-ghost ml-auto">
            {timeAgo(entry.created_at)}
          </span>
        )}
      </div>
    </div>
  );
}
