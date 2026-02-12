import { useCallback, useEffect, useRef, useState } from "react";
import {
  containerResourceUsage,
  getResourceQuotas,
  saveResourceQuotas,
  type ResourceUsage,
  type ResourceQuotas,
} from "../../lib/tauri";
import { Gauge, Save, Check } from "lucide-react";

export function ResourcesTab() {
  const [usage, setUsage] = useState<ResourceUsage | null>(null);
  const [quotas, setQuotas] = useState<ResourceQuotas>({
    cpu_percent: null,
    memory_mb: null,
  });
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);
  const [dirty, setDirty] = useState(false);
  const savedTimer = useRef<ReturnType<typeof setTimeout>>();

  const refreshUsage = useCallback(async () => {
    try {
      const u = await containerResourceUsage();
      setUsage(u);
    } catch {
      // Docker may not be running
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
    refreshUsage();
    loadQuotas();
    const interval = setInterval(refreshUsage, 5000);
    return () => clearInterval(interval);
  }, [refreshUsage, loadQuotas]);

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
      {/* Live usage */}
      <section className="bg-nx-surface rounded-[var(--radius-card)] border border-nx-border p-5">
        <div className="flex items-center gap-2 mb-4">
          <Gauge size={15} strokeWidth={1.5} className="text-nx-text-muted" />
          <h3 className="text-[14px] font-semibold text-nx-text">
            Live Resource Usage
          </h3>
        </div>

        {usage === null ? (
          <p className="text-[12px] text-nx-text-ghost">
            Waiting for container stats...
          </p>
        ) : (
          <div className="space-y-4">
            {/* CPU */}
            <div>
              <div className="flex justify-between items-center mb-1.5">
                <span className="text-[12px] text-nx-text-muted">CPU</span>
                <span className="text-[13px] text-nx-text font-mono">
                  {usage.cpu_percent}%
                </span>
              </div>
              <div className="h-2 bg-nx-overlay rounded-full overflow-hidden">
                <div
                  className="h-full bg-nx-accent rounded-full transition-[width] duration-500 ease-out"
                  style={{
                    width: `${Math.min(usage.cpu_percent, 100)}%`,
                  }}
                />
              </div>
            </div>

            {/* Memory */}
            <div>
              <div className="flex justify-between items-center mb-1.5">
                <span className="text-[12px] text-nx-text-muted">Memory</span>
                <span className="text-[13px] text-nx-text font-mono">
                  {usage.memory_mb} MB
                </span>
              </div>
              <div className="h-2 bg-nx-overlay rounded-full overflow-hidden">
                <div
                  className="h-full bg-nx-info rounded-full transition-[width] duration-500 ease-out"
                  style={{
                    width: `${
                      quotas.memory_mb
                        ? Math.min(
                            (usage.memory_mb / quotas.memory_mb) * 100,
                            100
                          )
                        : Math.min(usage.memory_mb / 1024, 100)
                    }%`,
                  }}
                />
              </div>
            </div>

            <p className="text-[11px] text-nx-text-ghost">
              Total across all Nexus plugin containers. Refreshes every 5s.
            </p>
          </div>
        )}
      </section>

      {/* Quotas */}
      <section className="bg-nx-surface rounded-[var(--radius-card)] border border-nx-border p-5">
        <div className="flex items-center justify-between mb-4">
          <h3 className="text-[14px] font-semibold text-nx-text">
            Resource Quotas
          </h3>
          <button
            onClick={handleSave}
            disabled={!dirty || saving}
            className={`flex items-center gap-1.5 px-3 py-1.5 text-[11px] font-medium rounded-[var(--radius-button)] transition-all duration-150 ${
              saved
                ? "bg-nx-success-muted text-nx-success"
                : dirty
                  ? "bg-nx-accent hover:bg-nx-accent-hover text-nx-deep"
                  : "bg-nx-overlay text-nx-text-ghost cursor-default"
            }`}
          >
            {saved ? (
              <>
                <Check size={12} strokeWidth={1.5} />
                Saved
              </>
            ) : (
              <>
                <Save size={12} strokeWidth={1.5} />
                {saving ? "Saving..." : "Save"}
              </>
            )}
          </button>
        </div>

        <div className="space-y-5">
          {/* CPU quota */}
          <div>
            <label className="block text-[12px] text-nx-text-muted mb-2">
              CPU Limit (%)
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
                {quotas.cpu_percent != null ? `${quotas.cpu_percent}%` : "None"}
              </span>
            </div>
            <p className="text-[11px] text-nx-text-ghost mt-1">
              Maximum CPU percentage for all plugin containers. Leave at 0 for
              no limit.
            </p>
          </div>

          {/* Memory quota */}
          <div>
            <label className="block text-[12px] text-nx-text-muted mb-2">
              Memory Limit (MB)
            </label>
            <input
              type="number"
              min={0}
              step={64}
              value={quotas.memory_mb ?? ""}
              onChange={(e) => updateMemory(e.target.value)}
              placeholder="No limit"
              className="w-full px-3 py-2 text-[13px] bg-nx-wash border border-nx-border-strong rounded-[var(--radius-input)] text-nx-text placeholder:text-nx-text-muted focus:outline-none focus:shadow-[var(--shadow-focus)] transition-shadow duration-150 font-mono"
            />
            <p className="text-[11px] text-nx-text-ghost mt-1">
              Maximum memory in megabytes for all plugin containers. Leave empty
              for no limit.
            </p>
          </div>
        </div>

        <p className="text-[11px] text-nx-text-ghost mt-4 pt-4 border-t border-nx-border-subtle">
          Quotas are applied when containers are created or restarted.
        </p>
      </section>
    </div>
  );
}
