import { useTranslation } from "react-i18next";
import {
  Button,
  Modal,
  ModalContent,
  ModalHeader,
  ModalBody,
  ModalFooter,
} from "@heroui/react";
import { Trash2 } from "lucide-react";

interface ResourceDeleteDialogProps {
  isOpen: boolean;
  label: string;
  onClose: () => void;
  onConfirm: () => void;
}

export function ResourceDeleteDialog({ isOpen, label, onClose, onConfirm }: ResourceDeleteDialogProps) {
  const { t } = useTranslation("settings");

  return (
    <Modal isOpen={isOpen} onOpenChange={(open) => { if (!open) onClose(); }}>
      <ModalContent>
        {(onModalClose) => (
          <>
            <ModalHeader className="text-[14px] flex items-center gap-2">
              <Trash2 size={14} strokeWidth={1.5} />
              {t("extensionsTab.resourceDelete", { label })}
            </ModalHeader>
            <ModalBody>
              <p className="text-[13px] leading-relaxed text-default-500">
                {t("extensionsTab.resourceDeleteConfirm", { label })}
              </p>
            </ModalBody>
            <ModalFooter>
              <Button variant="flat" onPress={() => { onModalClose(); onClose(); }}>
                {t("extensionsTab.resourceCancel")}
              </Button>
              <Button
                color="danger"
                onPress={() => {
                  onConfirm();
                  onModalClose();
                  onClose();
                }}
              >
                {t("extensionsTab.resourceDeleteAction")}
              </Button>
            </ModalFooter>
          </>
        )}
      </ModalContent>
    </Modal>
  );
}
