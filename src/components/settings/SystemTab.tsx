import { useCallback, useEffect, useRef, useState } from "react";
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
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";

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

  return (
    <div className="space-y-6">
      {/* Container engine */}
      <section className="bg-nx-surface rounded-[var(--radius-card)] border border-nx-border p-5">
        <div className="flex items-center gap-2 mb-4">
          <Container size={15} strokeWidth={1.5} className="text-nx-text-muted" />
          <h3 className="text-[14px] font-semibold text-nx-text">
            {t("system.containerEngine")}
          </h3>
        </div>

        <div className="flex gap-2 mb-5">
          {ENGINES.map((e) => (
            <button
              key={e.id}
              onClick={() => e.available && setEngine(e.id)}
              disabled={!e.available}
              className={`relative flex items-center gap-2 px-4 py-2 text-[12px] font-medium rounded-[var(--radius-button)] transition-all duration-150 ${
                engine === e.id
                  ? "bg-nx-accent text-nx-deep"
                  : e.available
                    ? "bg-nx-overlay text-nx-text-muted hover:text-nx-text-secondary hover:bg-nx-wash"
                    : "bg-nx-overlay text-nx-text-ghost cursor-not-allowed opacity-50"
              }`}
            >
              {e.label}
              {!e.available && (
                <span className="text-[9px] px-1.5 py-0.5 rounded-[var(--radius-tag)] bg-nx-highlight-muted text-nx-highlight font-semibold tracking-wide">
                  {t("common:status.soon")}
                </span>
              )}
            </button>
          ))}
        </div>

        {/* Docker status */}
        {engine === "docker" && (
          <div className="space-y-4">
            <div className="flex items-center justify-between">
              <span className="text-[12px] text-nx-text-muted">{t("system.status")}</span>
              <button
                onClick={refreshEngine}
                disabled={checking}
                className="flex items-center gap-1.5 px-2.5 py-1 text-[11px] font-medium rounded-[var(--radius-button)] bg-nx-overlay hover:bg-nx-wash text-nx-text-secondary transition-all duration-150 disabled:opacity-50"
              >
                <RefreshCw
                  size={12}
                  strokeWidth={1.5}
                  className={checking ? "animate-spin" : ""}
                />
                {checking ? t("common:action.checking") : t("common:action.refresh")}
              </button>
            </div>

            {status === null ? (
              <div className="text-[13px] text-nx-text-muted">
                {t("system.checkingEngine")}
              </div>
            ) : (
              <div className="space-y-3">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2.5">
                    <span
                      className={`w-1.5 h-1.5 rounded-full ${
                        status.running
                          ? "bg-nx-success"
                          : status.installed
                            ? "bg-nx-warning"
                            : "bg-nx-error"
                      }`}
                      style={
                        status.running
                          ? { animation: "pulse-status 2s ease-in-out infinite" }
                          : undefined
                      }
                    />
                    <span className="text-[13px] text-nx-text-secondary">
                      {t("system.engine")}
                    </span>
                  </div>
                  <span
                    className={`text-[11px] font-medium px-2 py-0.5 rounded-[var(--radius-tag)] ${
                      status.running
                        ? "bg-nx-success-muted text-nx-success"
                        : status.installed
                          ? "bg-nx-warning-muted text-nx-warning"
                          : "bg-nx-error-muted text-nx-error"
                    }`}
                  >
                    {status.running
                      ? status.version
                        ? t("system.runningVersion", { version: status.version })
                        : t("common:status.running")
                      : status.installed
                        ? t("system.stopped")
                        : t("system.notFound")}
                  </span>
                </div>

                <div className="flex items-center justify-between">
                  <span className="text-[12px] text-nx-text-muted">{t("system.socket")}</span>
                  <span className="text-[11px] text-nx-text-ghost font-mono truncate ml-4">
                    {status.socket}
                  </span>
                </div>

                <p className="text-[11px] text-nx-text-ghost">
                  {status.message}
                </p>
              </div>
            )}
          </div>
        )}

        {engine !== "docker" && (
          <div className="text-center py-8">
            <p className="text-[13px] text-nx-text-muted">
              {t("system.engineSoon", { engine: engine === "podman" ? "Podman" : "Finch" })}
            </p>
            <p className="text-[11px] text-nx-text-ghost mt-1">
              {t("system.engineFuture")}
            </p>
          </div>
        )}
      </section>

      {/* Live usage */}
      <section className="bg-nx-surface rounded-[var(--radius-card)] border border-nx-border p-5">
        <div className="flex items-center gap-2 mb-4">
          <Gauge size={15} strokeWidth={1.5} className="text-nx-text-muted" />
          <h3 className="text-[14px] font-semibold text-nx-text">
            {t("system.resourceUsage")}
          </h3>
        </div>

        {usage === null ? (
          <p className="text-[12px] text-nx-text-ghost">
            {t("system.waitingStats")}
          </p>
        ) : (
          <div className="space-y-4">
            <div>
              <div className="flex justify-between items-center mb-1.5">
                <span className="text-[12px] text-nx-text-muted">{t("system.cpu")}</span>
                <span className="text-[13px] text-nx-text font-mono">
                  {t("system.percent", { value: usage.cpu_percent })}
                </span>
              </div>
              <div className="h-2 bg-nx-overlay rounded-full overflow-hidden">
                <div
                  className="h-full bg-nx-accent rounded-full transition-[width] duration-500 ease-out"
                  style={{ width: `${Math.min(usage.cpu_percent, 100)}%` }}
                />
              </div>
            </div>

            <div>
              <div className="flex justify-between items-center mb-1.5">
                <span className="text-[12px] text-nx-text-muted">{t("system.memory")}</span>
                <span className="text-[13px] text-nx-text font-mono">
                  {t("system.mb", { value: usage.memory_mb })}
                </span>
              </div>
              <div className="h-2 bg-nx-overlay rounded-full overflow-hidden">
                <div
                  className="h-full bg-nx-info rounded-full transition-[width] duration-500 ease-out"
                  style={{
                    width: `${
                      quotas.memory_mb
                        ? Math.min((usage.memory_mb / quotas.memory_mb) * 100, 100)
                        : Math.min(usage.memory_mb / 1024, 100)
                    }%`,
                  }}
                />
              </div>
            </div>

            <p className="text-[11px] text-nx-text-ghost">
              {t("system.resourceTotal")}
            </p>
          </div>
        )}
      </section>

      {/* Quotas */}
      <section className="bg-nx-surface rounded-[var(--radius-card)] border border-nx-border p-5">
        <div className="flex items-center justify-between mb-4">
          <h3 className="text-[14px] font-semibold text-nx-text">
            {t("system.resourceQuotas")}
          </h3>
          <Button
            onClick={handleSave}
            disabled={!dirty || saving}
            variant={saved ? "secondary" : dirty ? "default" : "secondary"}
            size="sm"
            className={
              saved
                ? "bg-nx-success-muted text-nx-success hover:bg-nx-success-muted"
                : !dirty
                  ? "text-nx-text-ghost cursor-default"
                  : undefined
            }
          >
            {saved ? (
              <>
                <Check size={12} strokeWidth={1.5} />
                {t("common:action.saved")}
              </>
            ) : (
              <>
                <Save size={12} strokeWidth={1.5} />
                {saving ? t("common:action.saving") : t("common:action.save")}
              </>
            )}
          </Button>
        </div>

        <div className="space-y-5">
          <div>
            <label className="block text-[12px] text-nx-text-muted mb-2">
              {t("system.cpuLimit")}
            </label>
            <div className="flex items-center gap-3">
              <input
                type="range"
                min={0}
                max={100}
                step={5}
                value={quotas.cpu_percent ?? 0}
                onChange={(e) => updateCpu(e.target.value === "0" ? "" : e.target.value)}
                className="flex-1 accent-nx-accent h-1.5"
              />
              <span className="text-[13px] text-nx-text font-mono w-14 text-right">
                {quotas.cpu_percent != null ? `${quotas.cpu_percent}%` : t("common:status.none")}
              </span>
            </div>
            <p className="text-[11px] text-nx-text-ghost mt-1">
              {t("system.cpuHint")}
            </p>
          </div>

          <div>
            <label className="block text-[12px] text-nx-text-muted mb-2">
              {t("system.memoryLimit")}
            </label>
            <Input
              type="number"
              min={0}
              step={64}
              value={quotas.memory_mb ?? ""}
              onChange={(e) => updateMemory(e.target.value)}
              placeholder={t("system.noLimit")}
              className="font-mono"
            />
            <p className="text-[11px] text-nx-text-ghost mt-1">
              {t("system.memoryHint")}
            </p>
          </div>
        </div>

        <p className="text-[11px] text-nx-text-ghost mt-4 pt-4 border-t border-nx-border-subtle">
          {t("system.quotasApplied")}
        </p>
      </section>
    </div>
  );
}
