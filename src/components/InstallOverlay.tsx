import { Loader2 } from "lucide-react";
import { useAppStore } from "../stores/appStore";
import { Modal, ModalContent, ModalBody } from "@heroui/react";

export function InstallOverlay() {
  const installStatus = useAppStore((s) => s.installStatus);

  return (
    <Modal
      isOpen={installStatus.active}
      hideCloseButton
      isDismissable={false}
      isKeyboardDismissDisabled

    >
      <ModalContent>
        <ModalBody className="flex flex-col items-center gap-4 px-8 py-6">
          <Loader2 size={28} strokeWidth={1.5} className="animate-spin text-primary" />
          <p className="text-[13px] font-medium text-center">
            {installStatus.message}
          </p>
        </ModalBody>
      </ModalContent>
    </Modal>
  );
}
