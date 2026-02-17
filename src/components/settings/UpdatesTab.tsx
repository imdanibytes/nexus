import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import i18n from "../../i18n";
import {
  checkUpdates,
  marketplaceRefresh,
  dismissUpdate,
  updatePlugin,
  updateExtension,
  updateExtensionForceKey,
  lastUpdateCheck,
  setUpdateCheckInterval,
} from "../../lib/tauri";
import { useAppStore } from "../../stores/appStore";
import { useNotificationStore } from "../../stores/notificationStore";
import { usePlugins } from "../../hooks/usePlugins";
import type { AvailableUpdate, UpdateSecurity } from "../../types/updates";
import { KeyChangeWarningDialog } from "./KeyChangeWarningDialog";
import {
  ArrowUpCircle,
  RefreshCw,
  Loader2,
  Check,
  X,
  ShieldCheck,
  ShieldAlert,
  ShieldX,
  Clock,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";

function useCheckIntervalOptions() {
  const { t } = useTranslation("settings");
  return [
    { value: 30, label: t("updates.every30min") },
    { value: 60, label: t("updates.hourly") },
    { value: 360, label: t("updates.every6hours") },
    { value: 1440, label: t("updates.daily") },
    { value: 10080, label: t("updates.weekly") },
    { value: 0, label: t("updates.manualOnly") },
  ];
}

const SECURITY_BADGE_STYLES: Record<
  string,
  { variant: "success" | "warning" | "error"; icon: typeof ShieldCheck }
> = {
  verified: { variant: "success", icon: ShieldCheck },
  key_match: { variant: "success", icon: ShieldCheck },
  digest_available: { variant: "success", icon: ShieldCheck },
  no_digest: { variant: "warning", icon: ShieldAlert },
  untrusted_source: { variant: "warning", icon: ShieldAlert },
  key_changed: { variant: "error", icon: ShieldX },
  manifest_domain_changed: { variant: "error", icon: ShieldAlert },
};

function humanize(flag: string): string {
  return flag.replace(/_/g, " ").replace(/\b\w/g, (c) => c.toUpperCase());
}

function SecurityBadges({ security }: { security: UpdateSecurity[] }) {
  return (
    <div className="flex gap-1 flex-wrap mt-1.5">
      {security.map((flag) => {
        const style = SECURITY_BADGE_STYLES[flag] ?? SECURITY_BADGE_STYLES.no_digest;
        const Icon = style.icon;
        return (
          <Badge key={flag} variant={style.variant} className="text-[9px]">
            <Icon size={10} strokeWidth={1.5} />
            {humanize(flag)}
          </Badge>
        );
      })}
    </div>
  );
}

function RegistryBadge({ source }: { source: string }) {
  const { t } = useTranslation("settings");
  const isOfficial =
    source.toLowerCase() === "official" || source.toLowerCase() === "nexus";
  return (
    <Badge variant={isOfficial ? "secondary" : "warning"}>
      {isOfficial ? t("common:status.official") : t("common:status.community")}
    </Badge>
  );
}

export function UpdatesTab() {
  const { t } = useTranslation("settings");
  const CHECK_INTERVALS = useCheckIntervalOptions();
  const { availableUpdates, setAvailableUpdates, addNotification } =
    useAppStore();
  const { notifications, dismiss: dismissNotification } = useNotificationStore();
  const { refresh: refreshPlugins } = usePlugins();

  function dismissNotificationByItemId(itemId: string) {
    const match = notifications.find(
      (n) => (n.data as { item_id?: string })?.item_id === itemId,
    );
    if (match) dismissNotification(match.id);
  }

  const { updateCheckInterval, setUpdateCheckInterval: setStoreInterval } = useAppStore();

  const [checking, setChecking] = useState(false);
  const [updatingId, setUpdatingId] = useState<string | null>(null);
  const [lastChecked, setLastChecked] = useState<string | null>(null);
  const [keyChangeUpdate, setKeyChangeUpdate] = useState<AvailableUpdate | null>(null);

  const loadLastChecked = useCallback(async () => {
    try {
      const ts = await lastUpdateCheck();
      setLastChecked(ts);
    } catch {
      // ignore
    }
  }, []);

  useEffect(() => {
    loadLastChecked();
  }, [loadLastChecked]);

  async function handleIntervalChange(minutes: number) {
    setStoreInterval(minutes);
    try {
      await setUpdateCheckInterval(minutes);
    } catch {
      // ignore
    }
  }

  async function handleCheck() {
    setChecking(true);
    try {
      await marketplaceRefresh();
      const updates = await checkUpdates();
      setAvailableUpdates(updates);
      await loadLastChecked();
      if (updates.length === 0) {
        addNotification(i18n.t("common:notification.allUpToDate"), "success");
      }
    } catch (e) {
      addNotification(i18n.t("common:error.updateCheckFailed", { error: e }), "error");
    } finally {
      setChecking(false);
    }
  }

  async function handleDismiss(update: AvailableUpdate) {
    try {
      await dismissUpdate(update.item_id, update.available_version);
      setAvailableUpdates(
        availableUpdates.filter((u) => u.item_id !== update.item_id)
      );
      dismissNotificationByItemId(update.item_id);
    } catch (e) {
      addNotification(i18n.t("common:error.dismissFailed", { error: e }), "error");
    }
  }

  async function handleUpdate(update: AvailableUpdate) {
    setUpdatingId(update.item_id);
    try {
      if (update.item_type === "plugin") {
        await updatePlugin(update.manifest_url, update.new_image_digest, update.build_context);
      } else {
        await updateExtension(update.manifest_url);
      }
      addNotification(
        i18n.t("common:notification.updatedTo", { name: update.item_name, version: update.available_version }),
        "success"
      );
      await refreshPlugins();
      setAvailableUpdates(
        availableUpdates.filter((u) => u.item_id !== update.item_id)
      );
      dismissNotificationByItemId(update.item_id);
    } catch (e) {
      addNotification(i18n.t("common:error.updateFailed", { error: e }), "error");
    } finally {
      setUpdatingId(null);
    }
  }

  async function handleForceKeyUpdate(update: AvailableUpdate) {
    setKeyChangeUpdate(null);
    setUpdatingId(update.item_id);
    try {
      await updateExtensionForceKey(update.manifest_url);
      addNotification(
        i18n.t("common:notification.updatedToKeyChange", { name: update.item_name, version: update.available_version }),
        "success"
      );
      await refreshPlugins();
      setAvailableUpdates(
        availableUpdates.filter((u) => u.item_id !== update.item_id)
      );
      dismissNotificationByItemId(update.item_id);
    } catch (e) {
      addNotification(i18n.t("common:error.updateFailed", { error: e }), "error");
    } finally {
      setUpdatingId(null);
    }
  }

  const formattedTime = lastChecked
    ? new Date(lastChecked).toLocaleString()
    : t("updates.never");

  return (
    <div className="space-y-6">
      {/* Header */}
      <section className="bg-nx-surface rounded-[var(--radius-card)] border border-nx-border p-5">
        <div className="flex items-start justify-between">
          <div>
            <div className="flex items-center gap-2 mb-2">
              <ArrowUpCircle
                size={15}
                strokeWidth={1.5}
                className="text-nx-text-muted"
              />
              <h3 className="text-[14px] font-semibold text-nx-text">
                {t("updates.availableUpdates")}
              </h3>
            </div>
            <p className="text-[11px] text-nx-text-ghost">
              {t("updates.lastChecked", { time: formattedTime })}
            </p>
            <div className="mt-3 flex items-center gap-2">
              <span className="text-[11px] text-nx-text-muted font-medium">
                {t("updates.updatesCount", { count: availableUpdates.length })}
              </span>
            </div>
          </div>
          <Button
            onClick={handleCheck}
            disabled={checking}
            size="sm"
            className="flex-shrink-0 ml-4"
          >
            {checking ? (
              <RefreshCw size={12} strokeWidth={1.5} className="animate-spin" />
            ) : (
              <RefreshCw size={12} strokeWidth={1.5} />
            )}
            {checking ? t("common:action.checking") : t("updates.checkNow")}
          </Button>
        </div>
      </section>

      {/* Auto-check frequency */}
      <section className="bg-nx-surface rounded-[var(--radius-card)] border border-nx-border p-5">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Clock size={15} strokeWidth={1.5} className="text-nx-text-muted" />
            <div>
              <h3 className="text-[13px] font-semibold text-nx-text">
                {t("updates.autoCheckFrequency")}
              </h3>
              <p className="text-[11px] text-nx-text-ghost">
                {t("updates.autoCheckDesc")}
              </p>
            </div>
          </div>
          <Select value={String(updateCheckInterval)} onValueChange={(v) => handleIntervalChange(Number(v))}>
            <SelectTrigger className="w-auto text-[11px] font-medium">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {CHECK_INTERVALS.map((opt) => (
                <SelectItem key={opt.value} value={String(opt.value)}>
                  {opt.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
      </section>

      {/* Empty state */}
      {availableUpdates.length === 0 && (
        <section className="bg-nx-surface rounded-[var(--radius-card)] border border-nx-border p-5">
          <div className="flex items-center gap-2">
            <Check size={14} strokeWidth={1.5} className="text-nx-success" />
            <p className="text-[12px] text-nx-text-ghost">
              {t("updates.allUpToDate")}
            </p>
          </div>
        </section>
      )}

      {/* Update list */}
      {availableUpdates.map((update) => {
        const isBusy = updatingId === update.item_id;
        const hasKeyChange = update.security.includes("key_changed");

        return (
          <section
            key={update.item_id}
            className="bg-nx-surface rounded-[var(--radius-card)] border border-nx-border p-5"
          >
            <div className="flex items-start justify-between">
              <div className="min-w-0 flex-1">
                <div className="flex items-center gap-2 mb-1">
                  <h4 className="text-[13px] font-semibold text-nx-text">
                    {update.item_name}
                  </h4>
                  <span className="text-[10px] text-nx-text-ghost font-mono">
                    {update.item_type}
                  </span>
                  <RegistryBadge source={update.registry_source} />
                </div>
                <p className="text-[11px] text-nx-text-muted">
                  {update.installed_version}{" "}
                  <span className="text-nx-text-ghost mx-1">&rarr;</span>{" "}
                  <span className="text-nx-accent font-medium">
                    {update.available_version}
                  </span>
                </p>
                <SecurityBadges security={update.security} />
              </div>

              <div className="flex items-center gap-2 flex-shrink-0 ml-4">
                <Button
                  onClick={() => handleDismiss(update)}
                  disabled={isBusy}
                  variant="secondary"
                  size="sm"
                >
                  <X size={12} strokeWidth={1.5} />
                  {t("common:action.dismiss")}
                </Button>
                {hasKeyChange ? (
                  <Button
                    onClick={() => setKeyChangeUpdate(update)}
                    disabled={isBusy}
                    variant="outline"
                    size="sm"
                    className="border-nx-error text-nx-error hover:bg-nx-error-muted"
                  >
                    <ShieldX size={12} strokeWidth={1.5} />
                    {t("updates.reviewKeyChange")}
                  </Button>
                ) : (
                  <Button
                    onClick={() => handleUpdate(update)}
                    disabled={isBusy}
                    size="sm"
                  >
                    {isBusy ? (
                      <Loader2
                        size={12}
                        strokeWidth={1.5}
                        className="animate-spin"
                      />
                    ) : (
                      <ArrowUpCircle size={12} strokeWidth={1.5} />
                    )}
                    {isBusy ? t("updates.updating") : t("updates.update")}
                  </Button>
                )}
              </div>
            </div>
          </section>
        );
      })}

      {/* Key change dialog */}
      {keyChangeUpdate && (
        <KeyChangeWarningDialog
          update={keyChangeUpdate}
          onCancel={() => setKeyChangeUpdate(null)}
          onForceUpdate={handleForceKeyUpdate}
        />
      )}
    </div>
  );
}
