import { useCallback, useEffect, useState } from "react";
import { checkDocker, openDockerDesktop, type DockerStatus } from "../../lib/tauri";
import { Container, RefreshCw, ExternalLink } from "lucide-react";

type RuntimeEngine = "docker" | "podman" | "finch";

const ENGINES: { id: RuntimeEngine; label: string; available: boolean }[] = [
  { id: "docker", label: "Docker", available: true },
  { id: "podman", label: "Podman", available: false },
  { id: "finch", label: "Finch", available: false },
];

export function RuntimeTab() {
  const [engine, setEngine] = useState<RuntimeEngine>("docker");
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
    <div className="space-y-6">
      {/* Engine selector */}
      <section className="bg-nx-surface rounded-[var(--radius-card)] border border-nx-border p-5">
        <div className="flex items-center gap-2 mb-4">
          <Container size={15} strokeWidth={1.5} className="text-nx-text-muted" />
          <h3 className="text-[14px] font-semibold text-nx-text">
            Container Engine
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
                  SOON
                </span>
              )}
            </button>
          ))}
        </div>

        {/* Docker status (only shown when Docker is selected) */}
        {engine === "docker" && (
          <div className="space-y-4">
            <div className="flex items-center justify-between">
              <span className="text-[12px] text-nx-text-muted">Status</span>
              <button
                onClick={refresh}
                disabled={checking}
                className="flex items-center gap-1.5 px-2.5 py-1 text-[11px] font-medium rounded-[var(--radius-button)] bg-nx-overlay hover:bg-nx-wash text-nx-text-secondary transition-all duration-150 disabled:opacity-50"
              >
                <RefreshCw
                  size={12}
                  strokeWidth={1.5}
                  className={checking ? "animate-spin" : ""}
                />
                {checking ? "Checking..." : "Refresh"}
              </button>
            </div>

            {status === null ? (
              <div className="text-[13px] text-nx-text-muted">
                Checking Docker status...
              </div>
            ) : (
              <div className="space-y-3">
                {/* Installed */}
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2.5">
                    <span
                      className={`w-1.5 h-1.5 rounded-full ${
                        status.installed ? "bg-nx-success" : "bg-nx-error"
                      }`}
                    />
                    <span className="text-[13px] text-nx-text-secondary">
                      Installed
                    </span>
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
                      style={
                        status.running
                          ? {
                              animation:
                                "pulse-status 2s ease-in-out infinite",
                            }
                          : undefined
                      }
                    />
                    <span className="text-[13px] text-nx-text-secondary">
                      Engine
                    </span>
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

                {/* Socket path */}
                <div className="flex items-center justify-between">
                  <span className="text-[12px] text-nx-text-muted">Socket</span>
                  <span className="text-[11px] text-nx-text-ghost font-mono">
                    /var/run/docker.sock
                  </span>
                </div>

                {/* Message */}
                <p className="text-[11px] text-nx-text-ghost">
                  {status.message}
                </p>

                {/* Open Desktop App */}
                {status.installed && !status.running && (
                  <button
                    onClick={() => openDockerDesktop()}
                    className="flex items-center gap-1.5 px-3 py-1.5 text-[11px] font-medium rounded-[var(--radius-button)] bg-nx-overlay hover:bg-nx-wash text-nx-text-secondary transition-all duration-150"
                  >
                    <ExternalLink size={12} strokeWidth={1.5} />
                    Open Docker Desktop
                  </button>
                )}
              </div>
            )}
          </div>
        )}

        {/* Placeholder for non-Docker engines */}
        {engine !== "docker" && (
          <div className="text-center py-8">
            <p className="text-[13px] text-nx-text-muted">
              {engine === "podman" ? "Podman" : "Finch"} support is coming soon.
            </p>
            <p className="text-[11px] text-nx-text-ghost mt-1">
              Nexus will support alternative container runtimes in a future
              release.
            </p>
          </div>
        )}
      </section>
    </div>
  );
}
