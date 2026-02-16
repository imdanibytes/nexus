import { broadcastToPlugins } from "./pluginBridge";
import { setTheme as persistTheme } from "./tauri";

const STORAGE_KEY = "nexus-theme";

export type ThemeId = "default" | "nebula";

export const THEMES: { id: ThemeId; labelKey: string; accent: string }[] = [
  { id: "default", labelKey: "general.themeDefault", accent: "#2DD4A8" },
  { id: "nebula", labelKey: "general.themeNebula", accent: "#8b8bf5" },
];

/** Read the persisted theme (falls back to "default"). */
export function getTheme(): ThemeId {
  const stored = localStorage.getItem(STORAGE_KEY);
  if (stored === "nebula") return "nebula";
  return "default";
}

/** Apply a theme: update DOM, persist to localStorage + backend, notify plugins. */
export function applyTheme(theme: ThemeId): void {
  // Apply to DOM
  if (theme === "default") {
    document.documentElement.removeAttribute("data-theme");
  } else {
    document.documentElement.setAttribute("data-theme", theme);
  }

  // Persist
  localStorage.setItem(STORAGE_KEY, theme);
  persistTheme(theme).catch(() => {});

  // Notify plugins
  broadcastToPlugins("theme_changed", { theme });
}

/** Apply the saved theme on startup (call once from main). */
export function initTheme(): void {
  const theme = getTheme();
  if (theme !== "default") {
    document.documentElement.setAttribute("data-theme", theme);
  }
}
