import type { defaultNS } from "./index";

declare module "i18next" {
  interface CustomTypeOptions {
    defaultNS: typeof defaultNS;
    // Resources intentionally omitted: strict key typing rejects cross-namespace
    // references like t("common:action.save") from useTranslation("settings").
    // i18next's type system only supports prefixed keys when useTranslation
    // receives an array of namespaces. Re-enable once all callsites are migrated.
  }
}
