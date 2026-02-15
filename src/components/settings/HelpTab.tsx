import { useTranslation } from "react-i18next";
import { HelpCircle, Monitor, Blocks } from "lucide-react";

function StatusDot({ color, animation }: { color: string; animation?: string }) {
  return (
    <span className="relative shrink-0 w-2.5 h-2.5 flex items-center justify-center">
      {animation && (
        <span
          className={`absolute inset-0 rounded-full ${color} opacity-30`}
          style={{ animation }}
        />
      )}
      <span
        className={`w-2 h-2 rounded-full ${color}`}
        style={animation ? { animation } : undefined}
      />
    </span>
  );
}

export function HelpTab() {
  const { t } = useTranslation("settings");

  const pluginIndicators = [
    {
      color: "bg-nx-success",
      animation: undefined,
      label: t("help.runningLoaded"),
      description: t("help.runningLoadedDesc"),
    },
    {
      color: "bg-nx-success",
      animation: "pulse-status 2s ease-in-out infinite",
      label: t("help.runningUnloaded"),
      description: t("help.runningUnloadedDesc"),
    },
    {
      color: "bg-nx-text-muted",
      animation: undefined,
      label: t("help.stoppedLabel"),
      description: t("help.stoppedDesc"),
    },
    {
      color: "bg-nx-error",
      animation: undefined,
      label: t("help.errorLabel"),
      description: t("help.errorDesc"),
    },
    {
      color: "bg-nx-warning",
      animation: undefined,
      label: t("help.installingLabel"),
      description: t("help.installingDesc"),
    },
  ];

  const extensionIndicators = [
    {
      color: "bg-nx-success",
      label: t("help.enabledLabel"),
      description: t("help.enabledDesc"),
    },
    {
      color: "bg-nx-text-muted",
      label: t("help.disabledLabel"),
      description: t("help.disabledDesc"),
    },
  ];

  return (
    <div className="space-y-6">
      {/* Plugin status indicators */}
      <section className="bg-nx-surface rounded-[var(--radius-card)] border border-nx-border p-5">
        <div className="flex items-center gap-2 mb-2">
          <Monitor size={15} strokeWidth={1.5} className="text-nx-text-muted" />
          <h3 className="text-[14px] font-semibold text-nx-text">
            {t("help.pluginStatusIndicators")}
          </h3>
        </div>
        <p className="text-[11px] text-nx-text-ghost mb-4">
          {t("help.pluginStatusDesc")}
        </p>
        <div className="space-y-1">
          {pluginIndicators.map((item) => (
            <div
              key={item.label}
              className="flex items-start gap-3 px-3 py-2.5 rounded-[var(--radius-button)] bg-nx-deep border border-nx-border-subtle"
            >
              <div className="pt-1">
                <StatusDot color={item.color} animation={item.animation} />
              </div>
              <div className="min-w-0">
                <p className="text-[12px] text-nx-text font-medium">
                  {item.label}
                </p>
                <p className="text-[11px] text-nx-text-ghost mt-0.5">
                  {item.description}
                </p>
              </div>
            </div>
          ))}
        </div>
      </section>

      {/* Extension status indicators */}
      <section className="bg-nx-surface rounded-[var(--radius-card)] border border-nx-border p-5">
        <div className="flex items-center gap-2 mb-2">
          <Blocks size={15} strokeWidth={1.5} className="text-nx-text-muted" />
          <h3 className="text-[14px] font-semibold text-nx-text">
            {t("help.extensionStatusIndicators")}
          </h3>
        </div>
        <p className="text-[11px] text-nx-text-ghost mb-4">
          {t("help.extensionStatusDesc")}
        </p>
        <div className="space-y-1">
          {extensionIndicators.map((item) => (
            <div
              key={item.label}
              className="flex items-start gap-3 px-3 py-2.5 rounded-[var(--radius-button)] bg-nx-deep border border-nx-border-subtle"
            >
              <div className="pt-1">
                <StatusDot color={item.color} />
              </div>
              <div className="min-w-0">
                <p className="text-[12px] text-nx-text font-medium">
                  {item.label}
                </p>
                <p className="text-[11px] text-nx-text-ghost mt-0.5">
                  {item.description}
                </p>
              </div>
            </div>
          ))}
        </div>
      </section>

      {/* Keyboard / tips */}
      <section className="bg-nx-surface rounded-[var(--radius-card)] border border-nx-border p-5">
        <div className="flex items-center gap-2 mb-2">
          <HelpCircle size={15} strokeWidth={1.5} className="text-nx-text-muted" />
          <h3 className="text-[14px] font-semibold text-nx-text">
            {t("help.tips")}
          </h3>
        </div>
        <div className="space-y-2 text-[12px] text-nx-text-secondary">
          <p>{t("help.tip1")}</p>
          <p>{t("help.tip2")}</p>
          <p>{t("help.tip3")}</p>
        </div>
      </section>
    </div>
  );
}
