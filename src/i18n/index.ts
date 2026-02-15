import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import { broadcastToPlugins } from "../lib/pluginBridge";
import { setLanguage } from "../lib/tauri";

import commonEn from "./locales/en/common.json";
import pluginsEn from "./locales/en/plugins.json";
import settingsEn from "./locales/en/settings.json";
import permissionsEn from "./locales/en/permissions.json";

import commonJa from "./locales/ja/common.json";
import pluginsJa from "./locales/ja/plugins.json";
import settingsJa from "./locales/ja/settings.json";
import permissionsJa from "./locales/ja/permissions.json";

import commonEs from "./locales/es/common.json";
import pluginsEs from "./locales/es/plugins.json";
import settingsEs from "./locales/es/settings.json";
import permissionsEs from "./locales/es/permissions.json";

import commonKo from "./locales/ko/common.json";
import pluginsKo from "./locales/ko/plugins.json";
import settingsKo from "./locales/ko/settings.json";
import permissionsKo from "./locales/ko/permissions.json";

import commonZh from "./locales/zh/common.json";
import pluginsZh from "./locales/zh/plugins.json";
import settingsZh from "./locales/zh/settings.json";
import permissionsZh from "./locales/zh/permissions.json";

import commonDe from "./locales/de/common.json";
import pluginsDe from "./locales/de/plugins.json";
import settingsDe from "./locales/de/settings.json";
import permissionsDe from "./locales/de/permissions.json";

export const defaultNS = "common";
export const resources = {
  en: {
    common: commonEn,
    plugins: pluginsEn,
    settings: settingsEn,
    permissions: permissionsEn,
  },
  ja: {
    common: commonJa,
    plugins: pluginsJa,
    settings: settingsJa,
    permissions: permissionsJa,
  },
  es: {
    common: commonEs,
    plugins: pluginsEs,
    settings: settingsEs,
    permissions: permissionsEs,
  },
  ko: {
    common: commonKo,
    plugins: pluginsKo,
    settings: settingsKo,
    permissions: permissionsKo,
  },
  zh: {
    common: commonZh,
    plugins: pluginsZh,
    settings: settingsZh,
    permissions: permissionsZh,
  },
  de: {
    common: commonDe,
    plugins: pluginsDe,
    settings: settingsDe,
    permissions: permissionsDe,
  },
} as const;

/** Available languages — native label shown in the switcher. */
export const LANGUAGES: { code: string; label: string }[] = [
  { code: "en", label: "English" },
  { code: "ja", label: "日本語" },
  { code: "es", label: "Español" },
  { code: "ko", label: "한국어" },
  { code: "zh", label: "中文" },
  { code: "de", label: "Deutsch" },
];

const STORAGE_KEY = "nexus-language";

i18n.use(initReactI18next).init({
  lng: localStorage.getItem(STORAGE_KEY) ?? "en",
  fallbackLng: "en",
  defaultNS,
  ns: ["common", "plugins", "settings", "permissions"],
  resources,
  interpolation: { escapeValue: false },
  returnNull: false,
});

i18n.on("languageChanged", (lng) => {
  localStorage.setItem(STORAGE_KEY, lng);
  // Persist to backend so plugin containers get NEXUS_LANGUAGE on next start
  setLanguage(lng).catch(() => {});
  // Notify running plugin iframes immediately
  broadcastToPlugins("language_changed", { language: lng });
});

export default i18n;
