import { useState, useMemo } from "react";
import { RefreshCw, Download, RotateCcw, Check, ChevronDown, ChevronUp } from "lucide-react";
import type { Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { marked } from "marked";

type UpdateState =
  | { phase: "idle" }
  | { phase: "checking" }
  | { phase: "up-to-date" }
  | { phase: "available"; update: Update }
  | { phase: "downloading"; progress: number; notes?: string }
  | { phase: "ready"; notes?: string }
  | { phase: "error"; message: string };

function ReleaseNotes({ markdown }: { markdown: string }) {
  const html = useMemo(() => marked.parse(markdown, { async: false }) as string, [markdown]);
  return (
    <div
      className="release-notes"
      dangerouslySetInnerHTML={{ __html: html }}
    />
  );
}

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
    const notes = update.body || undefined;

    setState({ phase: "downloading", progress: 0, notes });
    try {
      let totalBytes = 0;
      let downloadedBytes = 0;

      await update.downloadAndInstall((event) => {
        if (event.event === "Started" && event.data.contentLength) {
          totalBytes = event.data.contentLength;
        } else if (event.event === "Progress") {
          downloadedBytes += event.data.chunkLength;
          const progress = totalBytes > 0 ? (downloadedBytes / totalBytes) * 100 : 0;
          setState({ phase: "downloading", progress: Math.min(progress, 100), notes });
        } else if (event.event === "Finished") {
          setState({ phase: "ready", notes });
        }
      });

      setState({ phase: "ready", notes });
    } catch (e) {
      setState({ phase: "error", message: `Download failed: ${e}` });
    }
  }

  async function handleRelaunch() {
    await relaunch();
  }

  const [notesOpen, setNotesOpen] = useState(true);

  // Get release notes from whichever state carries them
  const releaseNotes =
    state.phase === "available"
      ? state.update.body || null
      : state.phase === "downloading" || state.phase === "ready"
        ? state.notes || null
        : null;

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

      {/* Release notes */}
      {releaseNotes && (
        <div className="rounded-[var(--radius-card)] border border-nx-border bg-nx-surface overflow-hidden">
          <button
            onClick={() => setNotesOpen(!notesOpen)}
            className="flex items-center justify-between w-full px-3 py-2 text-left hover:bg-nx-overlay/50 transition-colors"
          >
            <span className="text-[11px] font-semibold uppercase tracking-wider text-nx-text-muted">
              What's New
              {state.phase === "available" && (
                <span className="ml-2 normal-case tracking-normal font-normal text-nx-text-ghost">
                  v{state.update.version}
                </span>
              )}
            </span>
            {notesOpen ? (
              <ChevronUp size={12} className="text-nx-text-muted" />
            ) : (
              <ChevronDown size={12} className="text-nx-text-muted" />
            )}
          </button>
          {notesOpen && (
            <div className="px-3 pb-3 text-[12px] leading-relaxed text-nx-text-secondary border-t border-nx-border/50 pt-2 max-h-60 overflow-y-auto">
              <ReleaseNotes markdown={releaseNotes} />
            </div>
          )}
        </div>
      )}

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
