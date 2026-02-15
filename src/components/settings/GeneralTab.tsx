import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { appVersion, type AppVersionInfo } from "../../lib/tauri";
import { RegistrySettings } from "./RegistrySettings";
import { UpdateCheck } from "./UpdateCheck";
import { Info, Bug, Bell, BellOff, Globe, Check, ChevronsUpDown } from "lucide-react";
import { Switch } from "@/components/ui/switch";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from "@/components/ui/command";
import { Button } from "@/components/ui/button";
import {
  notificationsEnabled,
  setNotificationsEnabled,
} from "../../hooks/useOsNotification";
import { LANGUAGES } from "../../i18n";
import { cn } from "../../lib/utils";

export function GeneralTab() {
  const { t, i18n } = useTranslation("settings");
  const [version, setVersion] = useState<AppVersionInfo | null>(null);
  const [notifEnabled, setNotifEnabled] = useState(notificationsEnabled);
  const [langOpen, setLangOpen] = useState(false);

  useEffect(() => {
    appVersion().then(setVersion).catch(() => {});
  }, []);

  function handleNotifToggle(checked: boolean) {
    setNotifEnabled(checked);
    setNotificationsEnabled(checked);
  }

  return (
    <div className="space-y-6">
      {/* About */}
      <section className="bg-nx-surface rounded-[var(--radius-card)] border border-nx-border p-5">
        <div className="flex items-center gap-2 mb-4">
          <Info size={15} strokeWidth={1.5} className="text-nx-text-muted" />
          <h3 className="text-[14px] font-semibold text-nx-text">{t("general.about")}</h3>
        </div>
        <div className="space-y-2.5">
          <div className="flex justify-between items-center">
            <span className="text-[12px] text-nx-text-muted">{t("general.version")}</span>
            <span className="text-[13px] text-nx-text font-mono">
              {version?.version ?? "..."}
            </span>
          </div>
          <div className="flex justify-between items-center">
            <span className="text-[12px] text-nx-text-muted">{t("general.app")}</span>
            <span className="text-[13px] text-nx-text">
              {version?.name ?? "Nexus"}
            </span>
          </div>
          {version?.commit && (
            <div className="flex justify-between items-center">
              <span className="text-[12px] text-nx-text-muted">{t("general.build")}</span>
              <span className="text-[13px] text-nx-text font-mono">
                {version.commit}
              </span>
            </div>
          )}
        </div>
        <div className="mt-4 pt-4 border-t border-nx-border-subtle">
          <UpdateCheck />
        </div>
        <div className="mt-4 pt-4 border-t border-nx-border-subtle flex items-center justify-between">
          <div>
            <span className="text-[12px] text-nx-text-muted">{t("general.bugPrompt")}</span>
          </div>
          <a
            href={`https://github.com/imdanibytes/nexus/issues/new?template=bug_report.md&labels=bug&title=&body=${encodeURIComponent(`**Nexus Version:** ${version?.version ?? "unknown"}\n**OS:** ${navigator.platform}\n\n**Describe the bug**\n\n\n**Steps to reproduce**\n1. \n2. \n3. \n\n**Expected behavior**\n\n\n**Screenshots**\n`)}`}
            target="_blank"
            rel="noopener noreferrer"
            className="flex items-center gap-1.5 px-3 py-1.5 text-[12px] font-medium text-nx-text-muted hover:text-nx-text rounded-[var(--radius-button)] border border-nx-border hover:bg-nx-wash/40 transition-all duration-150"
          >
            <Bug size={13} strokeWidth={1.5} />
            {t("general.reportBug")}
          </a>
        </div>
      </section>

      {/* Notifications */}
      <section className="bg-nx-surface rounded-[var(--radius-card)] border border-nx-border p-5">
        <div className="flex items-center gap-2 mb-4">
          <Bell size={15} strokeWidth={1.5} className="text-nx-text-muted" />
          <h3 className="text-[14px] font-semibold text-nx-text">
            {t("general.notifications")}
          </h3>
        </div>
        <div className="flex items-center justify-between">
          <div>
            <p className="text-[13px] text-nx-text">
              {t("general.showNative")}
            </p>
            <p className="text-[11px] text-nx-text-ghost mt-0.5">
              {t("general.nativeHint")}
            </p>
          </div>
          <Switch checked={notifEnabled} onCheckedChange={handleNotifToggle} />
        </div>
        {!notifEnabled && (
          <div className="flex items-center gap-2 mt-4 pt-4 border-t border-nx-border-subtle">
            <BellOff size={13} strokeWidth={1.5} className="text-nx-text-ghost" />
            <p className="text-[11px] text-nx-text-ghost">
              {t("general.notifDisabled")}
            </p>
          </div>
        )}
      </section>

      {/* Language */}
      <section className="bg-nx-surface rounded-[var(--radius-card)] border border-nx-border p-5">
        <div className="flex items-center gap-2 mb-4">
          <Globe size={15} strokeWidth={1.5} className="text-nx-text-muted" />
          <h3 className="text-[14px] font-semibold text-nx-text">
            {t("general.language")}
          </h3>
        </div>
        <div className="flex items-center justify-between">
          <div>
            <p className="text-[13px] text-nx-text">
              {t("general.languageHint")}
            </p>
          </div>
          <Popover open={langOpen} onOpenChange={setLangOpen}>
            <PopoverTrigger asChild>
              <Button
                variant="outline"
                role="combobox"
                aria-expanded={langOpen}
                className="w-[180px] justify-between text-[12px] font-medium bg-nx-overlay border-nx-border hover:bg-nx-wash"
              >
                {LANGUAGES.find((l) => l.code === i18n.language)?.label ?? i18n.language}
                <ChevronsUpDown className="ml-2 size-3.5 shrink-0 opacity-50" />
              </Button>
            </PopoverTrigger>
            <PopoverContent
              className="w-[180px] p-0 bg-nx-raised border-nx-border-strong shadow-[var(--shadow-dropdown)] rounded-[var(--radius-input)]"
              align="end"
            >
              <Command className="bg-transparent">
                <CommandInput
                  placeholder={t("general.searchLanguage")}
                  className="text-[12px] h-8"
                />
                <CommandList>
                  <CommandEmpty className="text-[12px] text-nx-text-ghost py-4">
                    {t("general.noLanguageFound")}
                  </CommandEmpty>
                  <CommandGroup>
                    {LANGUAGES.map((lang) => (
                      <CommandItem
                        key={lang.code}
                        value={lang.label}
                        onSelect={() => {
                          i18n.changeLanguage(lang.code);
                          setLangOpen(false);
                        }}
                        className="text-[12px] rounded-[var(--radius-tag)]"
                      >
                        <Check
                          className={cn(
                            "mr-2 size-3.5",
                            i18n.language === lang.code ? "opacity-100" : "opacity-0"
                          )}
                        />
                        {lang.label}
                      </CommandItem>
                    ))}
                  </CommandGroup>
                </CommandList>
              </Command>
            </PopoverContent>
          </Popover>
        </div>
      </section>

      {/* Registries */}
      <RegistrySettings />
    </div>
  );
}
