import { useState, useMemo } from "react";
import { useTranslation } from "react-i18next";
import { RefreshCw, Download, RotateCcw, Check, ChevronDown, ChevronUp } from "lucide-react";
import type { Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { marked } from "marked";
import { Button, Card, CardBody } from "@heroui/react";

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
  const { t } = useTranslation("settings");
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
          <Button
            onPress={handleCheck}
          >
            <RefreshCw size={12} strokeWidth={1.5} />
            {t("updateCheck.checkForUpdates")}
          </Button>
        )}

        {state.phase === "checking" && (
          <Button
            isDisabled
          >
            <RefreshCw size={12} strokeWidth={1.5} className="animate-spin" />
            {t("common:action.checking")}
          </Button>
        )}

        {state.phase === "available" && (
          <Button
            onPress={handleDownload}
          >
            <Download size={12} strokeWidth={1.5} />
            {t("updateCheck.installVersion", { version: state.update.version })}
          </Button>
        )}

        {state.phase === "ready" && (
          <Button
            onPress={handleRelaunch}
            color="success"
          >
            <RotateCcw size={12} strokeWidth={1.5} />
            {t("updateCheck.restartToUpdate")}
          </Button>
        )}

        {/* Status text */}
        {state.phase === "up-to-date" && (
          <span className="flex items-center gap-1 text-[11px] text-success">
            <Check size={12} strokeWidth={1.5} />
            {t("updateCheck.latestVersion")}
          </span>
        )}

        {state.phase === "error" && (
          <span className="text-[11px] text-danger">{state.message}</span>
        )}
      </div>

      {/* Release notes */}
      {releaseNotes && (
        <Card className="overflow-hidden">
          <button
            onClick={() => setNotesOpen(!notesOpen)}
            className="flex items-center justify-between w-full px-3 py-2 text-left hover:bg-default-100 transition-colors"
          >
            <span className="text-[11px] font-semibold uppercase tracking-wider text-default-500">
              {t("updateCheck.whatsNew")}
              {state.phase === "available" && (
                <span className="ml-2 normal-case tracking-normal font-normal text-default-400">
                  {t("updateCheck.version", { version: state.update.version })}
                </span>
              )}
            </span>
            {notesOpen ? (
              <ChevronUp size={12} className="text-default-500" />
            ) : (
              <ChevronDown size={12} className="text-default-500" />
            )}
          </button>
          {notesOpen && (
            <CardBody className="px-3 pb-3 pt-2 text-[12px] leading-relaxed text-default-500 max-h-60 overflow-y-auto">
              <ReleaseNotes markdown={releaseNotes} />
            </CardBody>
          )}
        </Card>
      )}

      {/* Download progress bar */}
      {state.phase === "downloading" && (
        <div className="space-y-1.5">
          <div className="h-1.5 bg-default-100 rounded-full overflow-hidden">
            <div
              className="h-full bg-primary rounded-full transition-[width] duration-300 ease-out"
              style={{ width: `${state.progress}%` }}
            />
          </div>
          <p className="text-[10px] text-default-500">
            {t("updateCheck.downloading", { percent: Math.round(state.progress) })}
          </p>
        </div>
      )}
    </div>
  );
}
