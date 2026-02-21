import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  checkEngine,
  containerResourceUsage,
  getResourceQuotas,
  saveResourceQuotas,
  type EngineStatus,
  type ResourceUsage,
  type ResourceQuotas,
} from "../../lib/tauri";
import { Container, RefreshCw, Gauge, Save, Check } from "lucide-react";
import { Button, Input, Card, CardBody, Chip, Divider } from "@heroui/react";

type RuntimeEngine = "docker" | "podman" | "finch";

const ENGINES: { id: RuntimeEngine; label: string; available: boolean }[] = [
  { id: "docker", label: "Docker", available: true },
  { id: "podman", label: "Podman", available: false },
  { id: "finch", label: "Finch", available: false },
];

export function SystemTab() {
  const { t } = useTranslation("settings");

  // --- Runtime state ---
  const [engine, setEngine] = useState<RuntimeEngine>("docker");
  const [status, setStatus] = useState<EngineStatus | null>(null);
  const [checking, setChecking] = useState(false);

  const refreshEngine = useCallback(async () => {
    setChecking(true);
    try {
      const s = await checkEngine();
      setStatus(s);
    } catch {
      setStatus({
        engine_id: "unknown",
        installed: false,
        running: false,
        version: null,
        socket: "",
        message: t("common:error.engineCheckFailed"),
      });
    } finally {
      setChecking(false);
    }
  }, [t]);

  // --- Resources state ---
  const [usage, setUsage] = useState<ResourceUsage | null>(null);
  const [quotas, setQuotas] = useState<ResourceQuotas>({
    cpu_percent: null,
    memory_mb: null,
  });
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);
  const [dirty, setDirty] = useState(false);
  const savedTimer = useRef<ReturnType<typeof setTimeout>>(undefined);

  const refreshUsage = useCallback(async () => {
    try {
      const u = await containerResourceUsage();
      setUsage(u);
    } catch {
      // Engine may not be running
    }
  }, []);

  const loadQuotas = useCallback(async () => {
    try {
      const q = await getResourceQuotas();
      setQuotas(q);
    } catch {
      // ignore
    }
  }, []);

  useEffect(() => {
    refreshEngine();
    refreshUsage();
    loadQuotas();
    const interval = setInterval(refreshUsage, 5000);
    return () => clearInterval(interval);
  }, [refreshEngine, refreshUsage, loadQuotas]);

  async function handleSave() {
    setSaving(true);
    try {
      await saveResourceQuotas(quotas.cpu_percent, quotas.memory_mb);
      setDirty(false);
      setSaved(true);
      clearTimeout(savedTimer.current);
      savedTimer.current = setTimeout(() => setSaved(false), 2000);
    } catch {
      // TODO: toast
    } finally {
      setSaving(false);
    }
  }

  function updateCpu(val: string) {
    const n = val === "" ? null : parseFloat(val);
    setQuotas((q) => ({ ...q, cpu_percent: n }));
    setDirty(true);
  }

  function updateMemory(val: string) {
    const n = val === "" ? null : parseInt(val, 10);
    setQuotas((q) => ({ ...q, memory_mb: Number.isNaN(n) ? null : n }));
    setDirty(true);
  }

  const handleCpuChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) =>
      updateCpu(e.target.value === "0" ? "" : e.target.value),
    []
  );

  const handleMemoryChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => updateMemory(e.target.value),
    []
  );

  const engineStatusStyle = useMemo(
    () =>
      status?.running
        ? { animation: "pulse-status 2s ease-in-out infinite" }
        : undefined,
    [status?.running]
  );

  const cpuBarStyle = useMemo(
    () => ({ width: `${Math.min(usage?.cpu_percent ?? 0, 100)}%` }),
    [usage?.cpu_percent]
  );

  const memBarStyle = useMemo(
    () => ({
      width: `${
        quotas.memory_mb
          ? Math.min(((usage?.memory_mb ?? 0) / quotas.memory_mb) * 100, 100)
          : Math.min((usage?.memory_mb ?? 0) / 1024, 100)
      }%`,
    }),
    [quotas.memory_mb, usage?.memory_mb]
  );

  return (
    <div className="space-y-6">
      {/* Container engine */}
      <Card>
        <CardBody className="p-5">
          <div className="flex items-center gap-2 mb-4">
            <Container size={15} strokeWidth={1.5} className="text-default-500" />
            <h3 className="text-[14px] font-semibold">
              {t("system.containerEngine")}
            </h3>
          </div>

          <div className="flex gap-2 mb-5">
            {ENGINES.map((e) => (
              <Button
                key={e.id}
                // eslint-disable-next-line react-perf/jsx-no-new-function-as-prop
                onPress={() => e.available && setEngine(e.id)}
                isDisabled={!e.available}
              >
                {e.label}
                {!e.available && (
                  <Chip size="sm">
                    {t("common:status.soon")}
                  </Chip>
                )}
              </Button>
            ))}
          </div>

          {/* Docker status */}
          {engine === "docker" && (
            <div className="space-y-4">
              <div className="flex items-center justify-between">
                <span className="text-[12px] text-default-500">{t("system.status")}</span>
                <Button
                  onPress={refreshEngine}
                  isDisabled={checking}
                  startContent={
                    <RefreshCw
                      size={12}
                      strokeWidth={1.5}
                      className={checking ? "animate-spin" : ""}
                    />
                  }
                >
                  {checking ? t("common:action.checking") : t("common:action.refresh")}
                </Button>
              </div>

              {status === null ? (
                <div className="text-[13px] text-default-500">
                  {t("system.checkingEngine")}
                </div>
              ) : (
                <div className="space-y-3">
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-2.5">
                      <span
                        className={`w-1.5 h-1.5 rounded-full ${
                          status.running
                            ? "bg-success"
                            : status.installed
                              ? "bg-warning"
                              : "bg-danger"
                        }`}
                        style={engineStatusStyle}
                      />
                      <span className="text-[13px] text-default-500">
                        {t("system.engine")}
                      </span>
                    </div>
                    <Chip
                      size="sm"
                      variant="flat"
                      color={status.running ? "success" : status.installed ? "warning" : "danger"}
                    >
                      {status.running
                        ? status.version
                          ? t("system.runningVersion", { version: status.version })
                          : t("common:status.running")
                        : status.installed
                          ? t("system.stopped")
                          : t("system.notFound")}
                    </Chip>
                  </div>

                  <div className="flex items-center justify-between">
                    <span className="text-[12px] text-default-500">{t("system.socket")}</span>
                    <span className="text-[11px] text-default-400 font-mono truncate ml-4">
                      {status.socket}
                    </span>
                  </div>

                  <p className="text-[11px] text-default-400">
                    {status.message}
                  </p>
                </div>
              )}
            </div>
          )}

          {engine !== "docker" && (
            <div className="text-center py-8">
              <p className="text-[13px] text-default-500">
                {t("system.engineSoon", { engine: engine === "podman" ? "Podman" : "Finch" })}
              </p>
              <p className="text-[11px] text-default-400 mt-1">
                {t("system.engineFuture")}
              </p>
            </div>
          )}
        </CardBody>
      </Card>

      {/* Live usage */}
      <Card>
        <CardBody className="p-5">
          <div className="flex items-center gap-2 mb-4">
            <Gauge size={15} strokeWidth={1.5} className="text-default-500" />
            <h3 className="text-[14px] font-semibold">
              {t("system.resourceUsage")}
            </h3>
          </div>

          {usage === null ? (
            <p className="text-[12px] text-default-400">
              {t("system.waitingStats")}
            </p>
          ) : (
            <div className="space-y-4">
              <div>
                <div className="flex justify-between items-center mb-1.5">
                  <span className="text-[12px] text-default-500">{t("system.cpu")}</span>
                  <span className="text-[13px] font-mono">
                    {t("system.percent", { value: usage.cpu_percent })}
                  </span>
                </div>
                <div className="h-2 bg-default-100 rounded-full overflow-hidden">
                  <div
                    className="h-full bg-primary rounded-full transition-[width] duration-500 ease-out"
                    style={cpuBarStyle}
                  />
                </div>
              </div>

              <div>
                <div className="flex justify-between items-center mb-1.5">
                  <span className="text-[12px] text-default-500">{t("system.memory")}</span>
                  <span className="text-[13px] font-mono">
                    {t("system.mb", { value: usage.memory_mb })}
                  </span>
                </div>
                <div className="h-2 bg-default-100 rounded-full overflow-hidden">
                  <div
                    className="h-full bg-secondary rounded-full transition-[width] duration-500 ease-out"
                    style={memBarStyle}
                  />
                </div>
              </div>

              <p className="text-[11px] text-default-400">
                {t("system.resourceTotal")}
              </p>
            </div>
          )}
        </CardBody>
      </Card>

      {/* Quotas */}
      <Card>
        <CardBody className="p-5">
          <div className="flex items-center justify-between mb-4">
            <h3 className="text-[14px] font-semibold">
              {t("system.resourceQuotas")}
            </h3>
            <Button
              onPress={handleSave}
              isDisabled={!dirty || saving}
              color={saved ? "success" : dirty ? "primary" : undefined}
              startContent={saved ? <Check size={12} strokeWidth={1.5} /> : <Save size={12} strokeWidth={1.5} />}
            >
              {saved ? t("common:action.saved") : saving ? t("common:action.saving") : t("common:action.save")}
            </Button>
          </div>

          <div className="space-y-5">
            <div>
              <label className="block text-[12px] text-default-500 mb-2">
                {t("system.cpuLimit")}
              </label>
              <div className="flex items-center gap-3">
                <input
                  type="range"
                  min={0}
                  max={100}
                  step={5}
                  value={quotas.cpu_percent ?? 0}
                  onChange={handleCpuChange}
                  className="flex-1 accent-primary h-1.5"
                />
                <span className="text-[13px] font-mono w-14 text-right">
                  {quotas.cpu_percent != null ? `${quotas.cpu_percent}%` : t("common:status.none")}
                </span>
              </div>
              <p className="text-[11px] text-default-400 mt-1">
                {t("system.cpuHint")}
              </p>
            </div>

            <div>
              <label className="block text-[12px] text-default-500 mb-2">
                {t("system.memoryLimit")}
              </label>
              <Input
                type="number"
                min={0}
                step={64}
                value={String(quotas.memory_mb ?? "")}
                onChange={handleMemoryChange}
                placeholder={t("system.noLimit")}
                variant="bordered"
              />
              <p className="text-[11px] text-default-400 mt-1">
                {t("system.memoryHint")}
              </p>
            </div>
          </div>

          <Divider className="my-4" />
          <p className="text-[11px] text-default-400">
            {t("system.quotasApplied")}
          </p>
        </CardBody>
      </Card>
    </div>
  );
}
