import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { appVersion, type AppVersionInfo } from "../../lib/tauri";
import { RegistrySettings } from "./RegistrySettings";
import { UpdateCheck } from "./UpdateCheck";
import { Info, Bug, Bell, BellOff, Globe, Check, Sun, Moon, Monitor } from "lucide-react";
import { Switch, Autocomplete, AutocompleteItem, Button, Card, CardBody, Divider, Tabs, Tab } from "@heroui/react";
import {
  notificationsEnabled,
  setNotificationsEnabled,
} from "../../hooks/useOsNotification";
import { LANGUAGES } from "../../i18n";
import { cn } from "../../lib/utils";
import { getColorMode, applyColorMode, type ColorMode } from "../../lib/theme";

const COLOR_MODES: { id: ColorMode; icon: typeof Sun; labelKey: string }[] = [
  { id: "light", icon: Sun, labelKey: "general.modeLight" },
  { id: "dark", icon: Moon, labelKey: "general.modeDark" },
  { id: "system", icon: Monitor, labelKey: "general.modeSystem" },
];

export function GeneralTab() {
  const { t, i18n } = useTranslation("settings");
  const [version, setVersion] = useState<AppVersionInfo | null>(null);
  const [notifEnabled, setNotifEnabled] = useState(notificationsEnabled);
  const [colorMode, setColorMode] = useState<ColorMode>(getColorMode);

  useEffect(() => {
    appVersion().then(setVersion).catch(() => {});
  }, []);

  function handleNotifToggle(checked: boolean) {
    setNotifEnabled(checked);
    setNotificationsEnabled(checked);
  }

  function handleColorMode(mode: ColorMode) {
    setColorMode(mode);
    applyColorMode(mode);
  }

  const handleColorModeSelectionChange = useCallback(
    (key: React.Key) => handleColorMode(key as ColorMode),
    [],
  );

  const handleLanguageSelectionChange = useCallback(
    (key: React.Key | null) => {
      if (key) i18n.changeLanguage(String(key));
    },
    [i18n],
  );

  return (
    <div className="space-y-6">
      {/* Appearance */}
      <Card>
        <CardBody>
          <div className="flex items-center gap-2 mb-4">
            <Sun size={16} className="text-default-500" />
            <h3 className="text-sm font-semibold">{t("general.appearance")}</h3>
          </div>
          <Tabs
            selectedKey={colorMode}
            onSelectionChange={handleColorModeSelectionChange}
          >
            {COLOR_MODES.map((mode) => {
              const Icon = mode.icon;
              return (
                <Tab
                  key={mode.id}
                  title={
                    <div className="flex items-center gap-2">
                      <Icon size={14} />
                      {t(mode.labelKey)}
                    </div>
                  }
                />
              );
            })}
          </Tabs>
        </CardBody>
      </Card>

      {/* About */}
      <Card>
        <CardBody>
          <div className="flex items-center gap-2 mb-4">
            <Info size={16} className="text-default-500" />
            <h3 className="text-sm font-semibold">{t("general.about")}</h3>
          </div>
          <div className="space-y-3">
            <div className="flex justify-between items-center">
              <span className="text-sm text-default-500">{t("general.version")}</span>
              <span className="text-sm font-mono">
                {version?.version ?? "..."}
              </span>
            </div>
            <div className="flex justify-between items-center">
              <span className="text-sm text-default-500">{t("general.app")}</span>
              <span className="text-sm">
                {version?.name ?? "Nexus"}
              </span>
            </div>
            {version?.commit && (
              <div className="flex justify-between items-center">
                <span className="text-sm text-default-500">{t("general.build")}</span>
                <span className="text-sm font-mono">{version.commit}</span>
              </div>
            )}
          </div>
          <Divider className="my-4" />
          <UpdateCheck />
          <Divider className="my-4" />
          <div className="flex items-center justify-between">
            <span className="text-sm text-default-500">{t("general.bugPrompt")}</span>
            <Button
              as="a"
              href={`https://github.com/imdanibytes/nexus/issues/new?template=bug_report.md&labels=bug&title=&body=${encodeURIComponent(`**Nexus Version:** ${version?.version ?? "unknown"}\n**OS:** ${navigator.platform}\n\n**Describe the bug**\n\n\n**Steps to reproduce**\n1. \n2. \n3. \n\n**Expected behavior**\n\n\n**Screenshots**\n`)}`}
              target="_blank"
              rel="noopener noreferrer"
              startContent={<Bug size={14} />}
            >
              {t("general.reportBug")}
            </Button>
          </div>
        </CardBody>
      </Card>

      {/* Notifications */}
      <Card>
        <CardBody>
          <div className="flex items-center gap-2 mb-4">
            <Bell size={16} className="text-default-500" />
            <h3 className="text-sm font-semibold">{t("general.notifications")}</h3>
          </div>
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm">{t("general.showNative")}</p>
              <p className="text-xs text-default-400 mt-1">{t("general.nativeHint")}</p>
            </div>
            <Switch isSelected={notifEnabled} onValueChange={handleNotifToggle} />
          </div>
          {!notifEnabled && (
            <>
              <Divider className="my-4" />
              <div className="flex items-center gap-2">
                <BellOff size={14} className="text-default-400" />
                <p className="text-xs text-default-400">{t("general.notifDisabled")}</p>
              </div>
            </>
          )}
        </CardBody>
      </Card>

      {/* Language */}
      <Card>
        <CardBody>
          <div className="flex items-center gap-2 mb-4">
            <Globe size={16} className="text-default-500" />
            <h3 className="text-sm font-semibold">{t("general.language")}</h3>
          </div>
          <div className="flex items-center justify-between">
            <p className="text-sm">{t("general.languageHint")}</p>
            <Autocomplete
              defaultSelectedKey={i18n.language}
              onSelectionChange={handleLanguageSelectionChange}
              placeholder={t("general.searchLanguage")}
              className="w-[200px]"
            >
              {LANGUAGES.map((lang) => (
                <AutocompleteItem key={lang.code} textValue={lang.label}>
                  <div className="flex items-center gap-2">
                    <Check
                      className={cn(
                        "size-4",
                        i18n.language === lang.code ? "opacity-100" : "opacity-0",
                      )}
                    />
                    {lang.label}
                  </div>
                </AutocompleteItem>
              ))}
            </Autocomplete>
          </div>
        </CardBody>
      </Card>

      {/* Registries */}
      <RegistrySettings />
    </div>
  );
}
