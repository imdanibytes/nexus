import { Loader2 } from "lucide-react";
import { useAppStore } from "../stores/appStore";
import { Dialog, DialogContent } from "@/components/ui/dialog";

export function InstallOverlay() {
  const { installStatus } = useAppStore();

  return (
    <Dialog open={installStatus.active}>
      <DialogContent
        showCloseButton={false}
        className="max-w-xs flex flex-col items-center gap-4 px-8 py-6"
        onPointerDownOutside={(e) => e.preventDefault()}
        onEscapeKeyDown={(e) => e.preventDefault()}
      >
        <Loader2 size={28} strokeWidth={1.5} className="animate-spin text-nx-accent" />
        <p className="text-[13px] font-medium text-nx-text text-center">
          {installStatus.message}
        </p>
      </DialogContent>
    </Dialog>
  );
}
