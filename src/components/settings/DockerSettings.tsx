import { useCallback, useEffect, useState } from "react";
import { checkDocker, type DockerStatus } from "../../lib/tauri";
import { Container, RefreshCw } from "lucide-react";

export function DockerSettings() {
  const [status, setStatus] = useState<DockerStatus | null>(null);
  const [checking, setChecking] = useState(false);

  const refresh = useCallback(async () => {
    setChecking(true);
    try {
      const s = await checkDocker();
      setStatus(s);
    } catch {
      setStatus({
        installed: false,
        running: false,
        version: null,
        message: "Failed to check Docker status",
      });
    } finally {
      setChecking(false);
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  return (
    <section className="bg-nx-surface rounded-[var(--radius-card)] border border-nx-border p-5">
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center gap-2">
          <Container size={15} strokeWidth={1.5} className="text-nx-text-muted" />
          <h3 className="text-[14px] font-semibold text-nx-text">Docker</h3>
        </div>
        <button
          onClick={refresh}
          disabled={checking}
          className="flex items-center gap-1.5 px-2.5 py-1 text-[11px] font-medium rounded-[var(--radius-button)] bg-nx-overlay hover:bg-nx-wash text-nx-text-secondary transition-all duration-150 disabled:opacity-50"
        >
          <RefreshCw size={12} strokeWidth={1.5} className={checking ? "animate-spin" : ""} />
          {checking ? "Checking..." : "Refresh"}
        </button>
      </div>

      {status === null ? (
        <div className="text-[13px] text-nx-text-muted">Checking Docker status...</div>
      ) : (
        <div className="space-y-4">
          <div className="space-y-3">
            {/* Installed */}
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2.5">
                <span
                  className={`w-1.5 h-1.5 rounded-full ${
                    status.installed ? "bg-nx-success" : "bg-nx-error"
                  }`}
                />
                <span className="text-[13px] text-nx-text-secondary">Installed</span>
              </div>
              {status.installed ? (
                <span className="text-[11px] text-nx-text-muted font-mono">
                  {status.version ? `v${status.version}` : "Yes"}
                </span>
              ) : (
                <a
                  href="https://www.docker.com/products/docker-desktop/"
                  target="_blank"
                  rel="noopener noreferrer"
                  className="text-[11px] font-medium px-2.5 py-1 rounded-[var(--radius-button)] bg-nx-accent hover:bg-nx-accent-hover text-nx-deep transition-all duration-150"
                >
                  Download
                </a>
              )}
            </div>

            {/* Engine running */}
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2.5">
                <span
                  className={`w-1.5 h-1.5 rounded-full ${
                    status.running ? "bg-nx-success" : "bg-nx-error"
                  }`}
                  style={status.running ? { animation: "pulse-status 2s ease-in-out infinite" } : undefined}
                />
                <span className="text-[13px] text-nx-text-secondary">Engine</span>
              </div>
              <span
                className={`text-[11px] font-medium px-2 py-0.5 rounded-[var(--radius-tag)] ${
                  status.running
                    ? "bg-nx-success-muted text-nx-success"
                    : "bg-nx-error-muted text-nx-error"
                }`}
              >
                {status.running ? "Running" : "Stopped"}
              </span>
            </div>
          </div>

          {/* Message */}
          <p className="text-[11px] text-nx-text-ghost">{status.message}</p>
        </div>
      )}
    </section>
  );
}
