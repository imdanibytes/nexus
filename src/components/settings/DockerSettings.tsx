import { useCallback, useEffect, useState } from "react";
import { checkDocker, type DockerStatus } from "../../lib/tauri";

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
    <section className="bg-slate-800 rounded-xl border border-slate-700 p-5">
      <div className="flex items-center justify-between mb-4">
        <h3 className="text-sm font-semibold text-white">Docker</h3>
        <button
          onClick={refresh}
          disabled={checking}
          className="px-2.5 py-1 text-xs rounded-lg bg-slate-700 hover:bg-slate-600 text-slate-300 transition-colors disabled:opacity-50"
        >
          {checking ? "Checking..." : "Refresh"}
        </button>
      </div>

      {status === null ? (
        <div className="text-sm text-slate-400">Checking Docker status...</div>
      ) : (
        <div className="space-y-4">
          <div className="space-y-3">
            {/* Installed */}
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <span
                  className={`w-2 h-2 rounded-full ${
                    status.installed ? "bg-green-500" : "bg-red-500"
                  }`}
                />
                <span className="text-sm text-slate-300">Installed</span>
              </div>
              {status.installed ? (
                <span className="text-xs text-slate-400">
                  {status.version ? `v${status.version}` : "Yes"}
                </span>
              ) : (
                <a
                  href="https://www.docker.com/products/docker-desktop/"
                  target="_blank"
                  rel="noopener noreferrer"
                  className="text-xs px-2.5 py-1 rounded-lg bg-indigo-500 hover:bg-indigo-600 text-white transition-colors"
                >
                  Download
                </a>
              )}
            </div>

            {/* Engine running */}
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <span
                  className={`w-2 h-2 rounded-full ${
                    status.running ? "bg-green-500" : "bg-red-500"
                  }`}
                />
                <span className="text-sm text-slate-300">Engine</span>
              </div>
              <span
                className={`text-xs ${
                  status.running ? "text-green-400" : "text-red-400"
                }`}
              >
                {status.running ? "Running" : "Stopped"}
              </span>
            </div>
          </div>

          {/* Message */}
          <p className="text-xs text-slate-500">{status.message}</p>
        </div>
      )}
    </section>
  );
}
