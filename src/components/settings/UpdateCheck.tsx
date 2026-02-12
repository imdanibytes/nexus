import { useState } from "react";
import { RefreshCw } from "lucide-react";

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
    <div className="flex items-center gap-3">
      <button
        onClick={handleCheck}
        disabled={checking}
        className="flex items-center gap-1.5 px-3 py-1.5 text-[11px] font-medium rounded-[var(--radius-button)] bg-nx-overlay hover:bg-nx-wash text-nx-text-secondary transition-all duration-150 disabled:opacity-50"
      >
        <RefreshCw size={12} strokeWidth={1.5} className={checking ? "animate-spin" : ""} />
        {checking ? "Checking..." : "Check for Updates"}
      </button>
      {status && (
        <span className="text-[11px] text-nx-text-muted">{status}</span>
      )}
    </div>
  );
}
