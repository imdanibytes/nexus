import { useTranslation, Trans } from "react-i18next";
import { AlertTriangle } from "lucide-react";
import type { AvailableUpdate } from "../../types/updates";
import {
  Modal,
  ModalContent,
  ModalHeader,
  ModalBody,
  ModalFooter,
  Button,
} from "@heroui/react";

interface KeyChangeWarningDialogProps {
  update: AvailableUpdate;
  onCancel: () => void;
  onForceUpdate: (update: AvailableUpdate) => void;
}

export function KeyChangeWarningDialog({
  update,
  onCancel,
  onForceUpdate,
}: KeyChangeWarningDialogProps) {
  const { t } = useTranslation("settings");

  return (
    <Modal
      isOpen
      onOpenChange={(open) => { if (!open) onCancel(); }}
    >
      <ModalContent>
        {(onClose) => (
          <>
            <ModalHeader className="flex items-center gap-3 text-[15px]">
              <AlertTriangle size={20} strokeWidth={1.5} className="text-danger shrink-0" />
              {t("keyChange.securityWarning")}
            </ModalHeader>
            <ModalBody>
              <p className="text-[12px] text-default-500 leading-relaxed">
                <Trans
                  i18nKey="keyChange.keyChangedDesc"
                  ns="settings"
                  values={{ name: update.item_name }}
                  components={{ strong: <strong /> }}
                />
              </p>

              {/* Key details */}
              <div className="bg-danger-50 rounded-[14px] p-3 text-[11px] font-mono text-default-500 space-y-1">
                <p>
                  <span className="text-default-500">{t("keyChange.extension")}</span>{" "}
                  {update.item_id}
                </p>
                <p>
                  <span className="text-default-500">{t("keyChange.newVersion")}</span>{" "}
                  {update.available_version}
                </p>
                <p>
                  <span className="text-default-500">{t("keyChange.source")}</span>{" "}
                  {update.registry_source}
                </p>
              </div>
            </ModalBody>
            <ModalFooter>
              <Button onPress={onClose}>
                {t("common:action.cancel")}
              </Button>
              <Button
                onPress={() => onForceUpdate(update)}
                color="danger"
              >
                {t("keyChange.understandUpdate")}
              </Button>
            </ModalFooter>
          </>
        )}
      </ModalContent>
    </Modal>
  );
}
