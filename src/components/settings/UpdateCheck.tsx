import { useState, useEffect, useMemo } from "react";
import { useTranslation } from "react-i18next";
import { RefreshCw, Download, RotateCcw, Check, ChevronDown, ChevronUp } from "lucide-react";
import { relaunch } from "@tauri-apps/plugin-process";
import { listen } from "@tauri-apps/api/event";
import { marked } from "marked";
import { Button, Card, CardBody, RadioGroup, Radio } from "@heroui/react";
import {
  checkAppUpdate,
  downloadAppUpdate,
  getUpdateChannel,
  setUpdateChannel as setUpdateChannelApi,
  type AppUpdateInfo,
} from "../../lib/tauri";
import { useAppStore } from "../../stores/appStore";

type UpdateState =
  | { phase: "idle" }
  | { phase: "checking" }
  | { phase: "up-to-date" }
  | { phase: "available"; info: AppUpdateInfo }
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
  const channel = useAppStore((s) => s.updateChannel);
  const setChannel = useAppStore((s) => s.setUpdateChannel);

  // Load persisted channel on mount
  useEffect(() => {
    getUpdateChannel().then((ch) => {
      if (ch === "stable" || ch === "nightly") setChannel(ch);
    }).catch(() => {});
  }, [setChannel]);

  async function handleCheck() {
    setState({ phase: "checking" });
    try {
      const info = await checkAppUpdate();
      if (info) {
        setState({ phase: "available", info });
      } else {
        setState({ phase: "up-to-date" });
      }
    } catch (e) {
      setState({ phase: "error", message: `${e}` });
    }
  }

  async function handleDownload() {
    if (state.phase !== "available") return;
    const notes = state.info.body || undefined;

    setState({ phase: "downloading", progress: 0, notes });

    let totalBytes = 0;
    let downloadedBytes = 0;

    const unlisten = await listen<{ event: string; chunkLength?: number; contentLength?: number }>(
      "nexus://app-update-progress",
      (ev) => {
        const data = ev.payload;
        if (data.event === "progress") {
          if (data.contentLength && totalBytes === 0) {
            totalBytes = data.contentLength;
          }
          downloadedBytes += data.chunkLength ?? 0;
          const progress = totalBytes > 0 ? (downloadedBytes / totalBytes) * 100 : 0;
          setState({ phase: "downloading", progress: Math.min(progress, 100), notes });
        } else if (data.event === "finished") {
          setState({ phase: "ready", notes });
        }
      },
    );

    try {
      await downloadAppUpdate();
      setState({ phase: "ready", notes });
    } catch (e) {
      setState({ phase: "error", message: `Download failed: ${e}` });
    } finally {
      unlisten();
    }
  }

  async function handleRelaunch() {
    await relaunch();
  }

  async function handleChannelChange(value: string) {
    if (value !== "stable" && value !== "nightly") return;
    setChannel(value);
    try {
      await setUpdateChannelApi(value);
      // Re-check for updates on the new channel
      setState({ phase: "checking" });
      const info = await checkAppUpdate();
      if (info) {
        setState({ phase: "available", info });
      } else {
        setState({ phase: "up-to-date" });
      }
    } catch (e) {
      setState({ phase: "error", message: `${e}` });
    }
  }

  const [notesOpen, setNotesOpen] = useState(true);

  const releaseNotes =
    state.phase === "available"
      ? state.info.body || null
      : state.phase === "downloading" || state.phase === "ready"
        ? state.notes || null
        : null;

  return (
    <div className="space-y-3">
      {/* Channel selector */}
      <RadioGroup
        label={t("updateCheck.updateChannel")}
        orientation="horizontal"
        value={channel}
        onValueChange={handleChannelChange}
        classNames={{ label: "text-xs text-default-500" }}
      >
        <Radio value="stable" size="sm">
          {t("updateCheck.channelStable")}
        </Radio>
        <Radio value="nightly" size="sm">
          {t("updateCheck.channelNightly")}
        </Radio>
      </RadioGroup>

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
            {t("updateCheck.installVersion", { version: state.info.version })}
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
                  {t("updateCheck.version", { version: state.info.version })}
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
