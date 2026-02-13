import type { ExtensionRegistryEntry } from "../../types/extension";

interface Props {
  entry: ExtensionRegistryEntry;
  onSelect: () => void;
}

export function ExtensionRegistryCard({ entry, onSelect }: Props) {
  return (
    <div
      onClick={onSelect}
      className="p-4 rounded-[var(--radius-card)] border border-nx-border bg-nx-surface hover:border-nx-border-strong hover:shadow-[var(--shadow-card-hover)] cursor-pointer transition-all duration-200"
    >
      <div className="flex items-start justify-between mb-2">
        <div>
          <h3 className="text-[13px] font-semibold text-nx-text">
            {entry.name}
          </h3>
          <p className="text-[11px] text-nx-text-muted font-mono">
            v{entry.version}
          </p>
        </div>
      </div>
      <p className="text-[11px] text-nx-text-secondary line-clamp-2">
        {entry.description}
      </p>
      <div className="flex gap-1.5 mt-2.5 flex-wrap">
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
      </div>
    </div>
  );
}
