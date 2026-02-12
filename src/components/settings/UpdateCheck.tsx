import { useState } from "react";

export function UpdateCheck() {
  const [checking, setChecking] = useState(false);
  const [status, setStatus] = useState<string | null>(null);

  async function handleCheck() {
    setChecking(true);
    setStatus(null);
    try {
      const { check } = await import("@tauri-apps/plugin-updater");
      const update = await check();
      if (update) {
        setStatus(`Update available: v${update.version}`);
      } else {
        setStatus("You're on the latest version");
      }
    } catch (e) {
      setStatus(`Update check failed: ${e}`);
    } finally {
      setChecking(false);
    }
  }

  return (
    <div>
      <div className="flex items-center gap-3">
        <button
          onClick={handleCheck}
          disabled={checking}
          className="px-3 py-1.5 text-xs rounded-lg bg-slate-700 hover:bg-slate-600 text-slate-300 transition-colors disabled:opacity-50"
        >
          {checking ? "Checking..." : "Check for Updates"}
        </button>
        {status && <span className="text-xs text-slate-400">{status}</span>}
      </div>
    </div>
  );
}
