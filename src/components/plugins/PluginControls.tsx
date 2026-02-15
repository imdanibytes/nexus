import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { PluginStatus } from "../../types/plugin";
import { Play, Square, Trash2, ScrollText, Hammer, Wrench, TriangleAlert } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";

interface Props {
  status: PluginStatus;
  pluginName?: string;
  disabled?: boolean;
  isLocal?: boolean;
  devMode?: boolean;
  onStart: () => void;
  onStop: () => void;
  onRemove: () => void;
  onShowLogs: () => void;
  onRebuild?: () => void;
  onToggleDevMode?: (enabled: boolean) => void;
}

export function PluginControls({
  status,
  pluginName,
  disabled = false,
  isLocal = false,
  devMode = false,
  onStart,
  onStop,
  onRemove,
  onShowLogs,
  onRebuild,
  onToggleDevMode,
}: Props) {
  const { t } = useTranslation("plugins");
  const [dialogOpen, setDialogOpen] = useState(false);

  return (
    <div className={`flex items-center gap-2 ${disabled ? "opacity-40 pointer-events-none" : ""}`}>
      <Button
        variant="secondary"
        size="xs"
        onClick={onShowLogs}
        disabled={disabled}
        title={t("controls.viewLogs")}
      >
        <ScrollText size={12} strokeWidth={1.5} />
        {t("logs.title")}
      </Button>

      {isLocal && onRebuild && (
        <Button
          variant="secondary"
          size="xs"
          onClick={onRebuild}
          disabled={disabled}
          className="bg-nx-accent-muted text-nx-accent hover:bg-nx-accent/20"
          title={t("controls.rebuildFromSource")}
        >
          <Hammer size={12} strokeWidth={1.5} />
          {t("menu.rebuild")}
        </Button>
      )}

      {isLocal && onToggleDevMode && (
        <Button
          variant={devMode ? "default" : "secondary"}
          size="xs"
          onClick={() => onToggleDevMode(!devMode)}
          disabled={disabled}
          title={devMode ? t("controls.disableDevModeTooltip") : t("controls.enableDevModeTooltip")}
        >
          <Wrench size={12} strokeWidth={1.5} />
          {t("menu.dev")}
        </Button>
      )}

      {status === "running" ? (
        <Button
          variant="secondary"
          size="xs"
          onClick={onStop}
          disabled={disabled}
          className="bg-nx-warning-muted text-nx-warning hover:bg-nx-warning/20"
        >
          <Square size={12} strokeWidth={1.5} />
          {t("common:action.stop")}
        </Button>
      ) : (
        <Button
          variant="secondary"
          size="xs"
          onClick={onStart}
          disabled={disabled}
          className="bg-nx-success-muted text-nx-success hover:bg-nx-success/20"
        >
          <Play size={12} strokeWidth={1.5} />
          {t("common:action.start")}
        </Button>
      )}

      <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
        <DialogTrigger asChild>
          <Button
            variant="destructive"
            size="xs"
            disabled={disabled}
          >
            <Trash2 size={12} strokeWidth={1.5} />
            {t("common:action.remove")}
          </Button>
        </DialogTrigger>
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2 text-base">
              <TriangleAlert size={18} className="text-nx-warning" />
              {t("common:confirm.removePlugin", { name: pluginName || "plugin" })}
            </DialogTitle>
            <DialogDescription className="text-[13px] leading-relaxed pt-1">
              {t("common:confirm.removePluginDesc")}
            </DialogDescription>
          </DialogHeader>
          <DialogFooter className="pt-2">
            <Button
              variant="secondary"
              size="sm"
              onClick={() => setDialogOpen(false)}
            >
              {t("common:action.cancel")}
            </Button>
            <Button
              variant="destructive"
              size="sm"
              onClick={() => {
                onRemove();
                setDialogOpen(false);
              }}
              className="bg-nx-error text-white hover:bg-nx-error/80"
            >
              {t("common:confirm.removeAndDeleteData")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
