import { useCallback, useEffect, useState } from "react";
import { usePlugins } from "../../hooks/usePlugins";
import { useAppStore } from "../../stores/appStore";
import { pluginGetSettings, pluginSaveSettings, pluginStorageInfo, pluginClearStorage } from "../../lib/tauri";
import type { InstalledPlugin, SettingDef } from "../../types/plugin";
import { Puzzle, Save, Check, Square, Trash2, Database, HardDrive, Cloud, Shield, Search, ChevronDown } from "lucide-react";
import { Switch } from "@/components/ui/switch";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Badge } from "@/components/ui/badge";
import { Collapsible, CollapsibleTrigger, CollapsibleContent } from "@/components/ui/collapsible";
import { PermissionList } from "../permissions/PermissionList";
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
  switch (def.type) {
    case "boolean":
      return (
        <label className="flex items-center gap-3 cursor-pointer">
          <Switch
            checked={!!value}
            onCheckedChange={(checked) => onChange(def.key, checked)}
          />
          <span className="text-[13px] text-nx-text">{def.label}</span>
        </label>
      );

    case "number":
      return (
        <div>
          <label className="block text-[12px] text-nx-text-muted mb-1.5">
            {def.label}
          </label>
          <Input
            type="number"
            value={value as number ?? ""}
            onChange={(e) =>
              onChange(
                def.key,
                e.target.value === "" ? null : Number(e.target.value)
              )
            }
          />
        </div>
      );

    case "select":
      return (
        <div>
          <label className="block text-[12px] text-nx-text-muted mb-1.5">
            {def.label}
          </label>
          <Select
            value={(value as string) ?? ""}
            onValueChange={(v) => onChange(def.key, v)}
          >
            <SelectTrigger className="w-full">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {def.options?.map((opt) => (
                <SelectItem key={opt} value={opt}>
                  {opt}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
      );

    // string and fallback
    default:
      return (
        <div>
          <label className="block text-[12px] text-nx-text-muted mb-1.5">
            {def.label}
          </label>
          <Input
            type="text"
            value={(value as string) ?? ""}
            onChange={(e) => onChange(def.key, e.target.value)}
          />
        </div>
      );
  }
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B";
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function StorageInfo({ pluginId }: { pluginId: string }) {
  const [bytes, setBytes] = useState<number | null>(null);
  const [clearing, setClearing] = useState(false);

  const load = useCallback(() => {
    pluginStorageInfo(pluginId)
      .then(setBytes)
      .catch(() => {});
  }, [pluginId]);

  useEffect(() => { load(); }, [load]);

  if (bytes === null) return null;

  return (
    <div className="flex items-center justify-between pt-3 mt-3 border-t border-nx-border-subtle">
      <div className="flex items-center gap-1.5">
        <Database size={11} strokeWidth={1.5} className="text-nx-text-ghost" />
        <span className="text-[11px] text-nx-text-ghost">
          Storage: {formatBytes(bytes)}
        </span>
      </div>
      {bytes > 0 && (
        <Button
          variant="link"
          size="sm"
          onClick={async () => {
            setClearing(true);
            try {
              await pluginClearStorage(pluginId);
              setBytes(0);
            } catch { /* ignore */ }
            finally { setClearing(false); }
          }}
          disabled={clearing}
          className="h-auto p-0 text-[10px] text-nx-error hover:text-nx-error/80"
        >
          {clearing ? "Clearing..." : "Clear data"}
        </Button>
      )}
    </div>
  );
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
  const isLocalSource = plugin.local_manifest_path != null;
  const defs = plugin.manifest.settings ?? [];
  const [values, setValues] = useState<Record<string, unknown>>({});
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [showConfirm, setShowConfirm] = useState(false);

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
        <Button
          variant="secondary"
          size="xs"
          onClick={onStop}
          disabled={busy !== null}
          className="bg-nx-warning-muted text-nx-warning hover:bg-nx-warning/20"
        >
          <Square size={10} strokeWidth={2} />
          {busy === "stopping" ? "Stopping..." : "Stop"}
        </Button>
      )}
      {showConfirm ? (
        <div className="flex items-center gap-1">
          <Button
            variant="destructive"
            size="xs"
            onClick={() => {
              onRemove();
              setShowConfirm(false);
            }}
            disabled={busy !== null}
            className="bg-nx-error hover:bg-nx-error/80 text-white"
          >
            {busy === "removing" ? "Removing..." : "Confirm"}
          </Button>
          <Button
            variant="secondary"
            size="xs"
            onClick={() => setShowConfirm(false)}
            disabled={busy !== null}
          >
            Cancel
          </Button>
        </div>
      ) : (
        <Button
          variant="destructive"
          size="xs"
          onClick={() => setShowConfirm(true)}
          disabled={busy !== null}
        >
          <Trash2 size={10} strokeWidth={2} />
          Remove
        </Button>
      )}
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
          <span className="flex items-center gap-1 text-[10px] px-1.5 py-0.5 rounded-[var(--radius-tag)] bg-nx-overlay text-nx-text-ghost flex-shrink-0">
            {isLocalSource ? <HardDrive size={9} strokeWidth={1.5} /> : <Cloud size={9} strokeWidth={1.5} />}
            {isLocalSource ? "Local" : "Registry"}
          </span>
          {actionButtons}
        </div>
        <p className="text-[11px] text-nx-text-ghost">
          No configurable settings
        </p>
        <StorageInfo pluginId={plugin.manifest.id} />
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
        <span className="flex items-center gap-1 text-[10px] px-1.5 py-0.5 rounded-[var(--radius-tag)] bg-nx-overlay text-nx-text-ghost flex-shrink-0">
          {isLocalSource ? <HardDrive size={9} strokeWidth={1.5} /> : <Cloud size={9} strokeWidth={1.5} />}
          {isLocalSource ? "Local" : "Registry"}
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
        <Button
          size="sm"
          onClick={handleSave}
          disabled={saving}
        >
          {saved ? (
            <Check size={13} strokeWidth={2} />
          ) : (
            <Save size={13} strokeWidth={1.5} />
          )}
          {saving ? "Saving..." : saved ? "Saved" : "Save"}
        </Button>
      </div>

      <StorageInfo pluginId={plugin.manifest.id} />
    </div>
  );
}

export function PluginsTab() {
  const { plugins, busyPlugins, stop, remove } = usePlugins();
  const { installedPlugins } = useAppStore();
  const [permSearch, setPermSearch] = useState("");
  const [permExpanded, setPermExpanded] = useState<Set<string>>(new Set());

  const filtered = installedPlugins.filter((p) =>
    p.manifest.name.toLowerCase().includes(permSearch.toLowerCase()) ||
    p.manifest.id.toLowerCase().includes(permSearch.toLowerCase())
  );

  function togglePerm(id: string) {
    setPermExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }

  return (
    <div className="space-y-6">
      {/* Settings */}
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

      {/* Permissions */}
      <section className="bg-nx-surface rounded-[var(--radius-card)] border border-nx-border p-5">
        <div className="flex items-center gap-2 mb-4">
          <Shield size={15} strokeWidth={1.5} className="text-nx-text-muted" />
          <h3 className="text-[14px] font-semibold text-nx-text">
            Permissions
          </h3>
        </div>

        {installedPlugins.length === 0 ? (
          <p className="text-[11px] text-nx-text-ghost">
            No plugins installed
          </p>
        ) : (
          <>
            <div className="relative mb-4">
              <Search
                size={14}
                strokeWidth={1.5}
                className="absolute left-3 top-1/2 -translate-y-1/2 text-nx-text-ghost"
              />
              <Input
                type="text"
                value={permSearch}
                onChange={(e) => setPermSearch(e.target.value)}
                placeholder="Filter plugins..."
                className="pl-9"
              />
            </div>

            <div className="space-y-2">
              {filtered.length === 0 ? (
                <p className="text-[11px] text-nx-text-ghost">
                  No plugins match "{permSearch}"
                </p>
              ) : (
                filtered.map((plugin) => {
                  const id = plugin.manifest.id;
                  const isOpen = permExpanded.has(id);
                  const permCount = plugin.manifest.permissions.length;

                  return (
                    <Collapsible key={id} open={isOpen} onOpenChange={() => togglePerm(id)}>
                      <div className="rounded-[var(--radius-button)] border border-nx-border-subtle bg-nx-deep overflow-hidden">
                        <CollapsibleTrigger asChild>
                          <button className="w-full flex items-center justify-between p-3 hover:bg-nx-wash/30 transition-colors duration-150">
                            <div className="flex items-center gap-3 min-w-0">
                              <span className="text-[13px] text-nx-text font-medium truncate">
                                {plugin.manifest.name}
                              </span>
                              <span className="text-[11px] text-nx-text-ghost font-mono flex-shrink-0">
                                v{plugin.manifest.version}
                              </span>
                              <Badge variant={plugin.status === "running" ? "success" : "secondary"} className="text-[10px]">
                                {plugin.status}
                              </Badge>
                            </div>
                            <div className="flex items-center gap-2 flex-shrink-0 ml-2">
                              <span className="text-[11px] text-nx-text-ghost">
                                {permCount} perm{permCount !== 1 ? "s" : ""}
                              </span>
                              <ChevronDown
                                size={14}
                                strokeWidth={1.5}
                                className={`text-nx-text-ghost transition-transform duration-200 ${
                                  isOpen ? "rotate-180" : ""
                                }`}
                              />
                            </div>
                          </button>
                        </CollapsibleTrigger>
                        <CollapsibleContent>
                          <div className="px-3 pb-3 border-t border-nx-border-subtle">
                            <div className="pt-3">
                              <PermissionList pluginId={id} />
                            </div>
                          </div>
                        </CollapsibleContent>
                      </div>
                    </Collapsible>
                  );
                })
              )}
            </div>
          </>
        )}
      </section>
    </div>
  );
}
