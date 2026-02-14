import { useCallback, useEffect, useState } from "react";
import {
  checkUpdates,
  marketplaceRefresh,
  dismissUpdate,
  updatePlugin,
  updateExtension,
  updateExtensionForceKey,
  lastUpdateCheck,
} from "../../lib/tauri";
import { useAppStore } from "../../stores/appStore";
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
} from "lucide-react";

const SECURITY_BADGE_STYLES: Record<
  string,
  { bg: string; text: string; icon: typeof ShieldCheck }
> = {
  verified: { bg: "bg-nx-success-muted", text: "text-nx-success", icon: ShieldCheck },
  key_match: { bg: "bg-nx-success-muted", text: "text-nx-success", icon: ShieldCheck },
  digest_available: { bg: "bg-nx-success-muted", text: "text-nx-success", icon: ShieldCheck },
  no_digest: { bg: "bg-nx-warning-muted", text: "text-nx-warning", icon: ShieldAlert },
  untrusted_source: { bg: "bg-nx-warning-muted", text: "text-nx-warning", icon: ShieldAlert },
  key_changed: { bg: "bg-nx-error-muted", text: "text-nx-error", icon: ShieldX },
  manifest_domain_changed: { bg: "bg-nx-error-muted", text: "text-nx-error", icon: ShieldAlert },
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
          <span
            key={flag}
            className={`inline-flex items-center gap-1 text-[9px] font-medium px-1.5 py-0.5 rounded-[var(--radius-tag)] ${style.bg} ${style.text}`}
          >
            <Icon size={10} strokeWidth={1.5} />
            {humanize(flag)}
          </span>
        );
      })}
    </div>
  );
}

function RegistryBadge({ source }: { source: string }) {
  const isOfficial =
    source.toLowerCase() === "official" || source.toLowerCase() === "nexus";
  return (
    <span
      className={`inline-flex text-[9px] font-medium px-1.5 py-0.5 rounded-[var(--radius-tag)] ${
        isOfficial
          ? "bg-nx-overlay text-nx-text-muted"
          : "bg-nx-warning-muted text-nx-warning"
      }`}
    >
      {isOfficial ? "Official" : "Community"}
    </span>
  );
}

export function UpdatesTab() {
  const { availableUpdates, setAvailableUpdates, addNotification } =
    useAppStore();
  const { refresh: refreshPlugins } = usePlugins();

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

  async function handleCheck() {
    setChecking(true);
    try {
      await marketplaceRefresh();
      const updates = await checkUpdates();
      setAvailableUpdates(updates);
      await loadLastChecked();
      if (updates.length === 0) {
        addNotification("Everything is up to date", "success");
      }
    } catch (e) {
      addNotification(`Update check failed: ${e}`, "error");
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
    } catch (e) {
      addNotification(`Failed to dismiss: ${e}`, "error");
    }
  }

  async function handleUpdate(update: AvailableUpdate) {
    setUpdatingId(update.item_id);
    try {
      if (update.item_type === "plugin") {
        await updatePlugin(update.manifest_url, update.new_image_digest);
      } else {
        await updateExtension(update.manifest_url);
      }
      addNotification(
        `${update.item_name} updated to ${update.available_version}`,
        "success"
      );
      await refreshPlugins();
      setAvailableUpdates(
        availableUpdates.filter((u) => u.item_id !== update.item_id)
      );
    } catch (e) {
      addNotification(`Update failed: ${e}`, "error");
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
        `${update.item_name} updated to ${update.available_version} (key change accepted)`,
        "success"
      );
      await refreshPlugins();
      setAvailableUpdates(
        availableUpdates.filter((u) => u.item_id !== update.item_id)
      );
    } catch (e) {
      addNotification(`Update failed: ${e}`, "error");
    } finally {
      setUpdatingId(null);
    }
  }

  const formattedTime = lastChecked
    ? new Date(lastChecked).toLocaleString()
    : "Never";

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
                Available Updates
              </h3>
            </div>
            <p className="text-[11px] text-nx-text-ghost">
              Last checked: {formattedTime}
            </p>
            <div className="mt-3 flex items-center gap-2">
              <span className="text-[11px] text-nx-text-muted font-medium">
                {availableUpdates.length} update
                {availableUpdates.length !== 1 ? "s" : ""} available
              </span>
            </div>
          </div>
          <button
            onClick={handleCheck}
            disabled={checking}
            className="flex items-center gap-1.5 px-3 py-1.5 text-[11px] font-medium rounded-[var(--radius-button)] bg-nx-accent hover:bg-nx-accent-hover text-nx-deep transition-all duration-150 flex-shrink-0 ml-4 disabled:opacity-50"
          >
            {checking ? (
              <RefreshCw size={12} strokeWidth={1.5} className="animate-spin" />
            ) : (
              <RefreshCw size={12} strokeWidth={1.5} />
            )}
            {checking ? "Checking..." : "Check Now"}
          </button>
        </div>
      </section>

      {/* Empty state */}
      {availableUpdates.length === 0 && (
        <section className="bg-nx-surface rounded-[var(--radius-card)] border border-nx-border p-5">
          <div className="flex items-center gap-2">
            <Check size={14} strokeWidth={1.5} className="text-nx-success" />
            <p className="text-[12px] text-nx-text-ghost">
              Everything is up to date.
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
                <button
                  onClick={() => handleDismiss(update)}
                  disabled={isBusy}
                  className="flex items-center gap-1.5 px-3 py-1.5 text-[11px] font-medium rounded-[var(--radius-button)] bg-nx-overlay hover:bg-nx-wash text-nx-text-secondary transition-all duration-150 disabled:opacity-50"
                >
                  <X size={12} strokeWidth={1.5} />
                  Dismiss
                </button>
                {hasKeyChange ? (
                  <button
                    onClick={() => setKeyChangeUpdate(update)}
                    disabled={isBusy}
                    className="flex items-center gap-1.5 px-3 py-1.5 text-[11px] font-medium rounded-[var(--radius-button)] border border-nx-error text-nx-error hover:bg-nx-error-muted transition-all duration-150 disabled:opacity-50"
                  >
                    <ShieldX size={12} strokeWidth={1.5} />
                    Review Key Change
                  </button>
                ) : (
                  <button
                    onClick={() => handleUpdate(update)}
                    disabled={isBusy}
                    className="flex items-center gap-1.5 px-3 py-1.5 text-[11px] font-medium rounded-[var(--radius-button)] bg-nx-accent hover:bg-nx-accent-hover text-nx-deep transition-all duration-150 disabled:opacity-50"
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
                    {isBusy ? "Updating..." : "Update"}
                  </button>
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
