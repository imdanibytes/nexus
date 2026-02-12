import { useState } from "react";
import { RefreshCw, Download, RotateCcw, Check } from "lucide-react";
import type { Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

type UpdateState =
  | { phase: "idle" }
  | { phase: "checking" }
  | { phase: "up-to-date" }
  | { phase: "available"; update: Update }
  | { phase: "downloading"; progress: number }
  | { phase: "ready" }
  | { phase: "error"; message: string };

export function UpdateCheck() {
  const [state, setState] = useState<UpdateState>({ phase: "idle" });

  async function handleCheck() {
    setState({ phase: "checking" });
    try {
      const { check } = await import("@tauri-apps/plugin-updater");
      const update = await check();
      if (update) {
        setState({ phase: "available", update });
      } else {
        setState({ phase: "up-to-date" });
      }
    } catch (e) {
      setState({ phase: "error", message: `${e}` });
    }
  }

  async function handleDownload() {
    if (state.phase !== "available") return;
    const { update } = state;

    setState({ phase: "downloading", progress: 0 });
    try {
      let totalBytes = 0;
      let downloadedBytes = 0;

      await update.downloadAndInstall((event) => {
        if (event.event === "Started" && event.data.contentLength) {
          totalBytes = event.data.contentLength;
        } else if (event.event === "Progress") {
          downloadedBytes += event.data.chunkLength;
          const progress = totalBytes > 0 ? (downloadedBytes / totalBytes) * 100 : 0;
          setState({ phase: "downloading", progress: Math.min(progress, 100) });
        } else if (event.event === "Finished") {
          setState({ phase: "ready" });
        }
      });

      setState({ phase: "ready" });
    } catch (e) {
      setState({ phase: "error", message: `Download failed: ${e}` });
    }
  }

  async function handleRelaunch() {
    await relaunch();
  }

  return (
    <div className="space-y-3">
      <div className="flex items-center gap-3">
        {/* Check / Download / Restart button */}
        {(state.phase === "idle" ||
          state.phase === "up-to-date" ||
          state.phase === "error") && (
          <button
            onClick={handleCheck}
            className="flex items-center gap-1.5 px-3 py-1.5 text-[11px] font-medium rounded-[var(--radius-button)] bg-nx-overlay hover:bg-nx-wash text-nx-text-secondary transition-all duration-150"
          >
            <RefreshCw size={12} strokeWidth={1.5} />
            Check for Updates
          </button>
        )}

        {state.phase === "checking" && (
          <button
            disabled
            className="flex items-center gap-1.5 px-3 py-1.5 text-[11px] font-medium rounded-[var(--radius-button)] bg-nx-overlay text-nx-text-muted opacity-60"
          >
            <RefreshCw size={12} strokeWidth={1.5} className="animate-spin" />
            Checking...
          </button>
        )}

        {state.phase === "available" && (
          <button
            onClick={handleDownload}
            className="flex items-center gap-1.5 px-3 py-1.5 text-[11px] font-medium rounded-[var(--radius-button)] bg-nx-accent hover:bg-nx-accent-hover text-nx-deep transition-all duration-150"
          >
            <Download size={12} strokeWidth={1.5} />
            Install v{state.update.version}
          </button>
        )}

        {state.phase === "ready" && (
          <button
            onClick={handleRelaunch}
            className="flex items-center gap-1.5 px-3 py-1.5 text-[11px] font-medium rounded-[var(--radius-button)] bg-nx-success hover:brightness-110 text-nx-deep transition-all duration-150"
          >
            <RotateCcw size={12} strokeWidth={1.5} />
            Restart to Update
          </button>
        )}

        {/* Status text */}
        {state.phase === "up-to-date" && (
          <span className="flex items-center gap-1 text-[11px] text-nx-success">
            <Check size={12} strokeWidth={1.5} />
            You're on the latest version
          </span>
        )}

        {state.phase === "error" && (
          <span className="text-[11px] text-nx-error">{state.message}</span>
        )}
      </div>

      {/* Download progress bar */}
      {state.phase === "downloading" && (
        <div className="space-y-1.5">
          <div className="h-1.5 bg-nx-overlay rounded-full overflow-hidden">
            <div
              className="h-full bg-nx-accent rounded-full transition-[width] duration-300 ease-out"
              style={{ width: `${state.progress}%` }}
            />
          </div>
          <p className="text-[10px] text-nx-text-muted">
            Downloading... {Math.round(state.progress)}%
          </p>
        </div>
      )}
    </div>
  );
}
