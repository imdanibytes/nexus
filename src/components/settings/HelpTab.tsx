import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { HelpCircle, Monitor, Blocks } from "lucide-react";
import { Card, CardBody } from "@heroui/react";

function StatusDot({ color, animation }: { color: string; animation?: string }) {
  const outerStyle = useMemo(
    () => (animation ? { animation } : undefined),
    [animation],
  );
  const innerStyle = useMemo(
    () => (animation ? { animation } : undefined),
    [animation],
  );
  return (
    <span className="relative shrink-0 w-2.5 h-2.5 flex items-center justify-center">
      {animation && (
        <span
          className={`absolute inset-0 rounded-full ${color} opacity-30`}
          style={outerStyle}
        />
      )}
      <span
        className={`w-2 h-2 rounded-full ${color}`}
        style={innerStyle}
      />
    </span>
  );
}

export function HelpTab() {
  const { t } = useTranslation("settings");

  const pluginIndicators = [
    {
      color: "bg-success",
      animation: undefined,
      label: t("help.runningLoaded"),
      description: t("help.runningLoadedDesc"),
    },
    {
      color: "bg-success",
      animation: "pulse-status 2s ease-in-out infinite",
      label: t("help.runningUnloaded"),
      description: t("help.runningUnloadedDesc"),
    },
    {
      color: "bg-default-400",
      animation: undefined,
      label: t("help.stoppedLabel"),
      description: t("help.stoppedDesc"),
    },
    {
      color: "bg-danger",
      animation: undefined,
      label: t("help.errorLabel"),
      description: t("help.errorDesc"),
    },
    {
      color: "bg-warning",
      animation: undefined,
      label: t("help.installingLabel"),
      description: t("help.installingDesc"),
    },
  ];

  const extensionIndicators = [
    {
      color: "bg-success",
      label: t("help.enabledLabel"),
      description: t("help.enabledDesc"),
    },
    {
      color: "bg-default-400",
      label: t("help.disabledLabel"),
      description: t("help.disabledDesc"),
    },
  ];

  return (
    <div className="space-y-6">
      {/* Plugin status indicators */}
      <Card><CardBody className="p-5">
        <div className="flex items-center gap-2 mb-2">
          <Monitor size={15} strokeWidth={1.5} className="text-default-500" />
          <h3 className="text-[14px] font-semibold">
            {t("help.pluginStatusIndicators")}
          </h3>
        </div>
        <p className="text-[11px] text-default-400 mb-4">
          {t("help.pluginStatusDesc")}
        </p>
        <div className="space-y-1">
          {pluginIndicators.map((item) => (
            <div
              key={item.label}
              className="flex items-start gap-3 px-3 py-2.5"
            >
              <div className="pt-1">
                <StatusDot color={item.color} animation={item.animation} />
              </div>
              <div className="min-w-0">
                <p className="text-[12px] font-medium">
                  {item.label}
                </p>
                <p className="text-[11px] text-default-400 mt-0.5">
                  {item.description}
                </p>
              </div>
            </div>
          ))}
        </div>
      </CardBody></Card>

      {/* Extension status indicators */}
      <Card><CardBody className="p-5">
        <div className="flex items-center gap-2 mb-2">
          <Blocks size={15} strokeWidth={1.5} className="text-default-500" />
          <h3 className="text-[14px] font-semibold">
            {t("help.extensionStatusIndicators")}
          </h3>
        </div>
        <p className="text-[11px] text-default-400 mb-4">
          {t("help.extensionStatusDesc")}
        </p>
        <div className="space-y-1">
          {extensionIndicators.map((item) => (
            <div
              key={item.label}
              className="flex items-start gap-3 px-3 py-2.5"
            >
              <div className="pt-1">
                <StatusDot color={item.color} />
              </div>
              <div className="min-w-0">
                <p className="text-[12px] font-medium">
                  {item.label}
                </p>
                <p className="text-[11px] text-default-400 mt-0.5">
                  {item.description}
                </p>
              </div>
            </div>
          ))}
        </div>
      </CardBody></Card>

      {/* Keyboard / tips */}
      <Card><CardBody className="p-5">
        <div className="flex items-center gap-2 mb-2">
          <HelpCircle size={15} strokeWidth={1.5} className="text-default-500" />
          <h3 className="text-[14px] font-semibold">
            {t("help.tips")}
          </h3>
        </div>
        <div className="space-y-2 text-[12px] text-default-500">
          <p>{t("help.tip1")}</p>
          <p>{t("help.tip2")}</p>
          <p>{t("help.tip3")}</p>
        </div>
      </CardBody></Card>
    </div>
  );
}
