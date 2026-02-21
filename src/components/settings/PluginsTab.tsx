import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { usePluginActions } from "../../hooks/usePlugins";
import { useAppStore } from "../../stores/appStore";
import { pluginGetSettings, pluginSaveSettings, pluginStorageInfo, pluginClearStorage } from "../../lib/tauri";
import type { InstalledPlugin, SettingDef } from "../../types/plugin";
import { Puzzle, Save, Check, Square, Trash2, Database, HardDrive, Cloud } from "lucide-react";
import { Switch, Button, Input, Select, SelectItem, Card, CardBody, Chip, Divider } from "@heroui/react";
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
  const handleBooleanChange = useCallback(
    (checked: boolean) => onChange(def.key, checked),
    [onChange, def.key]
  );
  const handleNumberChange = useCallback(
    (v: string) => onChange(def.key, v === "" ? null : Number(v)),
    [onChange, def.key]
  );
  const handleSelectChange = useCallback(
    (keys: Iterable<unknown>) => {
      const selected = Array.from(keys)[0];
      if (selected) onChange(def.key, String(selected));
    },
    [onChange, def.key]
  );
  const selectedKeys = useMemo(
    () => (value ? [String(value)] : []),
    [value]
  );
  const emptyArray = useMemo(() => [], []);
  const handleStringChange = useCallback(
    (v: string) => onChange(def.key, v),
    [onChange, def.key]
  );

  switch (def.type) {
    case "boolean":
      return (
        <label className="flex items-center gap-3 cursor-pointer">
          <Switch
            isSelected={!!value}
            onValueChange={handleBooleanChange}
          />
          <span className="text-[13px]">{def.label}</span>
        </label>
      );

    case "number":
      return (
        <div>
          <label className="block text-[12px] text-default-500 mb-1.5">
            {def.label}
          </label>
          <Input
            type="number"
            value={String(value ?? "")}
            onValueChange={handleNumberChange}
            variant="bordered"
          />
        </div>
      );

    case "select":
      return (
        <div>
          <label className="block text-[12px] text-default-500 mb-1.5">
            {def.label}
          </label>
          <Select
            selectedKeys={selectedKeys}
            onSelectionChange={handleSelectChange}
            variant="bordered"
          >
            {(def.options ?? emptyArray).map((opt) => (
              <SelectItem key={opt}>{opt}</SelectItem>
            ))}
          </Select>
        </div>
      );

    // string and fallback
    default:
      return (
        <div>
          <label className="block text-[12px] text-default-500 mb-1.5">
            {def.label}
          </label>
          <Input
            type="text"
            value={String(value ?? "")}
            onValueChange={handleStringChange}
            variant="bordered"
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
  const { t } = useTranslation("settings");
  const [bytes, setBytes] = useState<number | null>(null);
  const [clearing, setClearing] = useState(false);

  const load = useCallback(() => {
    pluginStorageInfo(pluginId)
      .then(setBytes)
      .catch(() => {});
  }, [pluginId]);

  useEffect(() => { load(); }, [load]);

  const handleClearStorage = useCallback(async () => {
    setClearing(true);
    try {
      await pluginClearStorage(pluginId);
      setBytes(0);
    } catch { /* ignore */ }
    finally { setClearing(false); }
  }, [pluginId]);

  if (bytes === null) return null;

  return (
    <>
      <Divider className="my-3" />
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-1.5">
          <Database size={11} strokeWidth={1.5} className="text-default-400" />
          <span className="text-[11px] text-default-400">
            Storage: {formatBytes(bytes)}
          </span>
        </div>
        {bytes > 0 && (
          <Button
            color="danger"
            onPress={handleClearStorage}
            isDisabled={clearing}
          >
            {clearing ? t("pluginsTab.clearing") : t("pluginsTab.clearData")}
          </Button>
        )}
      </div>
    </>
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
  const { t } = useTranslation("settings");
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

  const handleConfirmRemove = useCallback(() => {
    onRemove();
    setShowConfirm(false);
  }, [onRemove]);

  const handleCancelConfirm = useCallback(() => setShowConfirm(false), []);
  const handleShowConfirm = useCallback(() => setShowConfirm(true), []);

  const actionButtons = (
    <div className="flex items-center gap-1.5 flex-shrink-0 ml-auto">
      {plugin.status === "running" && (
        <Button
          color="warning"
          onPress={onStop}
          isDisabled={busy !== null}
          startContent={<Square size={10} strokeWidth={2} />}
        >
          {busy === "stopping" ? t("pluginsTab.stopping") : t("common:action.stop")}
        </Button>
      )}
      {showConfirm ? (
        <div className="flex items-center gap-1">
          <Button
            color="danger"
            onPress={handleConfirmRemove}
            isDisabled={busy !== null}
          >
            {busy === "removing" ? t("pluginsTab.removing") : t("common:action.confirm")}
          </Button>
          <Button
            onPress={handleCancelConfirm}
            isDisabled={busy !== null}
          >
            {t("common:action.cancel")}
          </Button>
        </div>
      ) : (
        <Button
          color="danger"
          onPress={handleShowConfirm}
          isDisabled={busy !== null}
          startContent={<Trash2 size={10} strokeWidth={2} />}
        >
          {t("common:action.remove")}
        </Button>
      )}
    </div>
  );

  if (defs.length === 0) {
    return (
      <Card>
        <CardBody className="p-4">
          <div className="flex items-center gap-3 mb-2">
            <span className="text-[13px] font-medium">
              {plugin.manifest.name}
            </span>
            <span className="text-[11px] text-default-400 font-mono">
              v{plugin.manifest.version}
            </span>
            <Chip
              size="sm"
              variant="flat"
              color={plugin.status === "running" ? "success" : "default"}
            >
              {plugin.status.toUpperCase()}
            </Chip>
            <Chip
              size="sm"
              variant="flat"
              startContent={isLocalSource ? <HardDrive size={9} strokeWidth={1.5} /> : <Cloud size={9} strokeWidth={1.5} />}
            >
              {isLocalSource ? t("common:status.local") : t("common:status.registry")}
            </Chip>
            {actionButtons}
          </div>
          <p className="text-[11px] text-default-400">
            {t("pluginsTab.noConfigurable")}
          </p>
          <StorageInfo pluginId={plugin.manifest.id} />
        </CardBody>
      </Card>
    );
  }

  return (
    <Card>
      <CardBody className="p-4">
        <div className="flex items-center gap-3 mb-4">
          <span className="text-[13px] font-medium">
            {plugin.manifest.name}
          </span>
          <span className="text-[11px] text-default-400 font-mono">
            v{plugin.manifest.version}
          </span>
          <Chip
            size="sm"
            variant="flat"
            color={plugin.status === "running" ? "success" : "default"}
          >
            {plugin.status.toUpperCase()}
          </Chip>
          <Chip
            size="sm"
            variant="flat"
            startContent={isLocalSource ? <HardDrive size={9} strokeWidth={1.5} /> : <Cloud size={9} strokeWidth={1.5} />}
          >
            {isLocalSource ? t("common:status.local") : t("common:status.registry")}
          </Chip>
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
          <p className="mt-3 text-[11px] text-danger">{error}</p>
        )}

        <div className="mt-4 flex items-center gap-2">
          <Button
            color="primary"
            onPress={handleSave}
            isDisabled={saving}
            startContent={saved ? <Check size={13} strokeWidth={2} /> : <Save size={13} strokeWidth={1.5} />}
          >
            {saving ? t("common:action.saving") : saved ? t("common:action.saved") : t("common:action.save")}
          </Button>
        </div>

        <StorageInfo pluginId={plugin.manifest.id} />
      </CardBody>
    </Card>
  );
}

export function PluginsTab() {
  const { t } = useTranslation("settings");
  const plugins = useAppStore((s) => s.installedPlugins);
  const busyPlugins = useAppStore((s) => s.busyPlugins);
  const { stop, remove } = usePluginActions();

  return (
    <div className="space-y-6">
      {/* Settings */}
      <Card>
        <CardBody className="p-5">
          <div className="flex items-center gap-2 mb-4">
            <Puzzle size={15} strokeWidth={1.5} className="text-default-500" />
            <h3 className="text-[14px] font-semibold">
              {t("pluginsTab.pluginSettings")}
            </h3>
          </div>

          {plugins.length === 0 ? (
            <p className="text-[11px] text-default-400">
              {t("pluginsTab.noPlugins")}
            </p>
          ) : (
            <div className="space-y-3">
              {plugins.map((plugin) => (
                <ErrorBoundary key={plugin.manifest.id} inline label={plugin.manifest.name}>
                  <PluginSettingsCard
                    plugin={plugin}
                    busy={busyPlugins[plugin.manifest.id] ?? null}
                    // eslint-disable-next-line react-perf/jsx-no-new-function-as-prop
                    onStop={() => stop(plugin.manifest.id)}
                    // eslint-disable-next-line react-perf/jsx-no-new-function-as-prop
                    onRemove={() => remove(plugin.manifest.id)}
                  />
                </ErrorBoundary>
              ))}
            </div>
          )}
        </CardBody>
      </Card>
    </div>
  );
}
