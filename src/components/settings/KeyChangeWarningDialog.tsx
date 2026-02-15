import { useTranslation, Trans } from "react-i18next";
import { AlertTriangle } from "lucide-react";
import type { AvailableUpdate } from "../../types/updates";
import {
  AlertDialog,
  AlertDialogContent,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogAction,
  AlertDialogCancel,
} from "@/components/ui/alert-dialog";

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
    <AlertDialog open>
      <AlertDialogContent className="max-w-md">
        <AlertDialogHeader>
          <AlertDialogTitle className="flex items-center gap-3 text-[15px]">
            <AlertTriangle size={20} strokeWidth={1.5} className="text-nx-error shrink-0" />
            {t("keyChange.securityWarning")}
          </AlertDialogTitle>
          <AlertDialogDescription className="text-[12px] text-nx-text-secondary leading-relaxed">
            <Trans
              i18nKey="keyChange.keyChangedDesc"
              ns="settings"
              values={{ name: update.item_name }}
              components={{ strong: <strong className="text-nx-text" /> }}
            />
          </AlertDialogDescription>
        </AlertDialogHeader>

        {/* Key details */}
        <div className="bg-nx-error-muted rounded-[var(--radius-card)] p-3 text-[11px] font-mono text-nx-text-secondary space-y-1">
          <p>
            <span className="text-nx-text-muted">{t("keyChange.extension")}</span>{" "}
            {update.item_id}
          </p>
          <p>
            <span className="text-nx-text-muted">{t("keyChange.newVersion")}</span>{" "}
            {update.available_version}
          </p>
          <p>
            <span className="text-nx-text-muted">{t("keyChange.source")}</span>{" "}
            {update.registry_source}
          </p>
        </div>

        <AlertDialogFooter>
          <AlertDialogCancel onClick={onCancel} className="bg-primary text-primary-foreground hover:bg-primary/90">
            {t("common:action.cancel")}
          </AlertDialogCancel>
          <AlertDialogAction
            variant="outline"
            onClick={() => onForceUpdate(update)}
            className="border-nx-error text-nx-error hover:bg-nx-error-muted"
          >
            {t("keyChange.understandUpdate")}
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
