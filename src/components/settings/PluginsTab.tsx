import { useEffect, useState } from "react";
import { usePlugins } from "../../hooks/usePlugins";
import { pluginGetSettings, pluginSaveSettings } from "../../lib/tauri";
import type { InstalledPlugin, SettingDef } from "../../types/plugin";
import { Puzzle, Save, Check, Square, Trash2 } from "lucide-react";
import { ErrorBoundary } from "../ErrorBoundary";

function SettingField({
  def,
  value,
  onChange,
}: {
  def: SettingDef;
  value: unknown;
  onChange: (key: string, value: unknown) => void;
}) {
  const baseInput =
    "w-full px-3 py-2 text-[13px] bg-nx-wash border border-nx-border-strong rounded-[var(--radius-input)] text-nx-text focus:outline-none focus:shadow-[var(--shadow-focus)] transition-shadow duration-150";

  switch (def.type) {
    case "boolean":
      return (
        <label className="flex items-center gap-3 cursor-pointer">
          <button
            type="button"
            role="switch"
            aria-checked={!!value}
            onClick={() => onChange(def.key, !value)}
            className={`relative w-9 h-5 rounded-full transition-colors duration-200 flex-shrink-0 ${
              value ? "bg-nx-accent" : "bg-nx-border-strong"
            }`}
          >
            <span
              className={`absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white transition-transform duration-200 ${
                value ? "translate-x-4" : "translate-x-0"
              }`}
            />
          </button>
          <span className="text-[13px] text-nx-text">{def.label}</span>
        </label>
      );

    case "number":
      return (
        <div>
          <label className="block text-[12px] text-nx-text-muted mb-1.5">
            {def.label}
          </label>
          <input
            type="number"
            value={value as number ?? ""}
            onChange={(e) =>
              onChange(
                def.key,
                e.target.value === "" ? null : Number(e.target.value)
              )
            }
            className={baseInput}
          />
        </div>
      );

    case "select":
      return (
        <div>
          <label className="block text-[12px] text-nx-text-muted mb-1.5">
            {def.label}
          </label>
          <select
            value={(value as string) ?? ""}
            onChange={(e) => onChange(def.key, e.target.value)}
            className={baseInput + " appearance-none"}
          >
            {def.options?.map((opt) => (
              <option key={opt} value={opt}>
                {opt}
              </option>
            ))}
          </select>
        </div>
      );

    // string and fallback
    default:
      return (
        <div>
          <label className="block text-[12px] text-nx-text-muted mb-1.5">
            {def.label}
          </label>
          <input
            type="text"
            value={(value as string) ?? ""}
            onChange={(e) => onChange(def.key, e.target.value)}
            className={baseInput}
          />
        </div>
      );
  }
}

function PluginSettingsCard({
  plugin,
  busy,
  onStop,
  onRemove,
}: {
  plugin: InstalledPlugin;
  busy: string | null;
  onStop: () => void;
  onRemove: () => void;
}) {
  const defs = plugin.manifest.settings ?? [];
  const [values, setValues] = useState<Record<string, unknown>>({});
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    pluginGetSettings(plugin.manifest.id)
      .then(setValues)
      .catch(() => {});
  }, [plugin.manifest.id]);

  function handleChange(key: string, value: unknown) {
    setValues((prev) => ({ ...prev, [key]: value }));
    setSaved(false);
  }

  async function handleSave() {
    setSaving(true);
    setError(null);
    try {
      await pluginSaveSettings(plugin.manifest.id, values);
      setSaved(true);
      setTimeout(() => setSaved(false), 2000);
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  }

  const actionButtons = (
    <div className="flex items-center gap-1.5 flex-shrink-0 ml-auto">
      {plugin.status === "running" && (
        <button
          onClick={onStop}
          disabled={busy !== null}
          className="flex items-center gap-1 px-2 py-1 text-[11px] font-medium rounded-[var(--radius-tag)] bg-nx-warning-muted text-nx-warning hover:bg-nx-warning/20 transition-colors duration-150 disabled:opacity-50"
        >
          <Square size={10} strokeWidth={2} />
          {busy === "stopping" ? "Stopping..." : "Stop"}
        </button>
      )}
      <button
        onClick={onRemove}
        disabled={busy !== null}
        className="flex items-center gap-1 px-2 py-1 text-[11px] font-medium rounded-[var(--radius-tag)] bg-nx-error-muted text-nx-error hover:bg-nx-error/20 transition-colors duration-150 disabled:opacity-50"
      >
        <Trash2 size={10} strokeWidth={2} />
        {busy === "removing" ? "Removing..." : "Remove"}
      </button>
    </div>
  );

  if (defs.length === 0) {
    return (
      <div className="rounded-[var(--radius-card)] border border-nx-border-subtle bg-nx-deep p-4">
        <div className="flex items-center gap-3 mb-2">
          <span className="text-[13px] text-nx-text font-medium">
            {plugin.manifest.name}
          </span>
          <span className="text-[11px] text-nx-text-ghost font-mono">
            v{plugin.manifest.version}
          </span>
          <span
            className={`text-[10px] font-semibold px-1.5 py-0.5 rounded-[var(--radius-tag)] flex-shrink-0 ${
              plugin.status === "running"
                ? "bg-nx-success-muted text-nx-success"
                : "bg-nx-overlay text-nx-text-ghost"
            }`}
          >
            {plugin.status.toUpperCase()}
          </span>
          {actionButtons}
        </div>
        <p className="text-[11px] text-nx-text-ghost">
          No configurable settings
        </p>
      </div>
    );
  }

  return (
    <div className="rounded-[var(--radius-card)] border border-nx-border-subtle bg-nx-deep p-4">
      <div className="flex items-center gap-3 mb-4">
        <span className="text-[13px] text-nx-text font-medium">
          {plugin.manifest.name}
        </span>
        <span className="text-[11px] text-nx-text-ghost font-mono">
          v{plugin.manifest.version}
        </span>
        <span
          className={`text-[10px] font-semibold px-1.5 py-0.5 rounded-[var(--radius-tag)] flex-shrink-0 ${
            plugin.status === "running"
              ? "bg-nx-success-muted text-nx-success"
              : "bg-nx-overlay text-nx-text-ghost"
          }`}
        >
          {plugin.status.toUpperCase()}
        </span>
        {actionButtons}
      </div>

      <div className="space-y-4">
        {defs.map((def) => (
          <SettingField
            key={def.key}
            def={def}
            value={values[def.key] ?? def.default}
            onChange={handleChange}
          />
        ))}
      </div>

      {error && (
        <p className="mt-3 text-[11px] text-nx-error">{error}</p>
      )}

      <div className="mt-4 flex items-center gap-2">
        <button
          onClick={handleSave}
          disabled={saving}
          className="flex items-center gap-1.5 px-3 py-1.5 text-[12px] font-medium bg-nx-accent text-white rounded-[var(--radius-button)] hover:opacity-90 transition-opacity duration-150 disabled:opacity-50"
        >
          {saved ? (
            <Check size={13} strokeWidth={2} />
          ) : (
            <Save size={13} strokeWidth={1.5} />
          )}
          {saving ? "Saving..." : saved ? "Saved" : "Save"}
        </button>
      </div>
    </div>
  );
}

export function PluginsTab() {
  const { plugins, busyPlugins, stop, remove } = usePlugins();

  return (
    <div className="space-y-6">
      <section className="bg-nx-surface rounded-[var(--radius-card)] border border-nx-border p-5">
        <div className="flex items-center gap-2 mb-4">
          <Puzzle size={15} strokeWidth={1.5} className="text-nx-text-muted" />
          <h3 className="text-[14px] font-semibold text-nx-text">
            Plugin Settings
          </h3>
        </div>

        {plugins.length === 0 ? (
          <p className="text-[11px] text-nx-text-ghost">
            No plugins installed
          </p>
        ) : (
          <div className="space-y-3">
            {plugins.map((plugin) => (
              <ErrorBoundary key={plugin.manifest.id} inline label={plugin.manifest.name}>
                <PluginSettingsCard
                  plugin={plugin}
                  busy={busyPlugins[plugin.manifest.id] ?? null}
                  onStop={() => stop(plugin.manifest.id)}
                  onRemove={() => remove(plugin.manifest.id)}
                />
              </ErrorBoundary>
            ))}
          </div>
        )}
      </section>
    </div>
  );
}
