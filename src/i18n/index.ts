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
} as const;

/** Available languages — native label shown in the switcher. */
export const LANGUAGES: { code: string; label: string }[] = [
  { code: "en", label: "English" },
  { code: "ja", label: "日本語" },
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
