import { useState, useCallback } from "react";
import { useTranslation } from "react-i18next";
import type { PluginStatus } from "../../types/plugin";
import { Play, Square, Trash2, ScrollText, Hammer, Wrench, TriangleAlert } from "lucide-react";
import {
  Button,
  Modal,
  ModalContent,
  ModalHeader,
  ModalBody,
  ModalFooter,
} from "@heroui/react";

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

  const handleToggleDevMode = useCallback(() => onToggleDevMode?.(!devMode), [onToggleDevMode, devMode]);
  const handleOpenDialog = useCallback(() => setDialogOpen(true), []);

  return (
    <div className={`flex items-center gap-2 ${disabled ? "opacity-40 pointer-events-none" : ""}`}>
      <Button
        onPress={onShowLogs}
        isDisabled={disabled}
        title={t("controls.viewLogs")}
        startContent={<ScrollText size={12} strokeWidth={1.5} />}
      >
        {t("logs.title")}
      </Button>

      {isLocal && onRebuild && (
        <Button
          color="primary"
          onPress={onRebuild}
          isDisabled={disabled}
          title={t("controls.rebuildFromSource")}
          startContent={<Hammer size={12} strokeWidth={1.5} />}
        >
          {t("menu.rebuild")}
        </Button>
      )}

      {isLocal && onToggleDevMode && (
        <Button
          onPress={handleToggleDevMode}
          isDisabled={disabled}
          title={devMode ? t("controls.disableDevModeTooltip") : t("controls.enableDevModeTooltip")}
          startContent={<Wrench size={12} strokeWidth={1.5} />}
        >
          {t("menu.dev")}
        </Button>
      )}

      {status === "running" ? (
        <Button
          onPress={onStop}
          isDisabled={disabled}
          color="warning"
          startContent={<Square size={12} strokeWidth={1.5} />}
        >
          {t("common:action.stop")}
        </Button>
      ) : (
        <Button
          onPress={onStart}
          isDisabled={disabled}
          color="success"
          startContent={<Play size={12} strokeWidth={1.5} />}
        >
          {t("common:action.start")}
        </Button>
      )}

      <Button
        color="danger"
        onPress={handleOpenDialog}
        isDisabled={disabled}
        startContent={<Trash2 size={12} strokeWidth={1.5} />}
      >
        {t("common:action.remove")}
      </Button>

      <Modal isOpen={dialogOpen} onOpenChange={setDialogOpen}>
        <ModalContent>
          {(onClose) => (
            <>
              <ModalHeader className="flex items-center gap-2 text-base">
                <TriangleAlert size={18} className="text-warning" />
                {t("common:confirm.removePlugin", { name: pluginName || "plugin" })}
              </ModalHeader>
              <ModalBody>
                <p className="text-[13px] leading-relaxed text-default-500">
                  {t("common:confirm.removePluginDesc")}
                </p>
              </ModalBody>
              <ModalFooter>
                <Button
                  onPress={onClose}
                >
                  {t("common:action.cancel")}
                </Button>
                <Button
                  color="danger"
                  // eslint-disable-next-line react-perf/jsx-no-new-function-as-prop
                  onPress={() => {
                    onRemove();
                    onClose();
                  }}
                >
                  {t("common:confirm.removeAndDeleteData")}
                </Button>
              </ModalFooter>
            </>
          )}
        </ModalContent>
      </Modal>
    </div>
  );
}
