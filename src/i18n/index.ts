import i18n from "i18next";
import { initReactI18next } from "react-i18next";

import commonEn from "./locales/en/common.json";
import pluginsEn from "./locales/en/plugins.json";
import settingsEn from "./locales/en/settings.json";
import permissionsEn from "./locales/en/permissions.json";

export const defaultNS = "common";
export const resources = {
  en: {
    common: commonEn,
    plugins: pluginsEn,
    settings: settingsEn,
    permissions: permissionsEn,
  },
} as const;

i18n.use(initReactI18next).init({
  lng: "en",
  fallbackLng: "en",
  defaultNS,
  ns: ["common", "plugins", "settings", "permissions"],
  resources,
  interpolation: { escapeValue: false },
  returnNull: false,
});

export default i18n;
