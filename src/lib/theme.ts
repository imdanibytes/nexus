import { broadcastToPlugins } from "./pluginBridge";

const MODE_KEY = "nexus-color-mode";

export type ColorMode = "light" | "dark" | "system";

function resolveMode(mode: ColorMode): "light" | "dark" {
  if (mode !== "system") return mode;
  return window.matchMedia("(prefers-color-scheme: dark)").matches
    ? "dark"
    : "light";
}

function applyDarkClass(effective: "light" | "dark") {
  if (effective === "dark") {
    document.documentElement.classList.add("dark");
    document.documentElement.classList.remove("light");
  } else {
    document.documentElement.classList.remove("dark");
    document.documentElement.classList.add("light");
  }
}

export function getColorMode(): ColorMode {
  const stored = localStorage.getItem(MODE_KEY) as ColorMode | null;
  if (stored === "light" || stored === "dark" || stored === "system")
    return stored;
  return "dark";
}

export function applyColorMode(mode: ColorMode): void {
  localStorage.setItem(MODE_KEY, mode);
  applyDarkClass(resolveMode(mode));
  broadcastToPlugins("color_mode_changed", {
    mode,
    effective: resolveMode(mode),
  });
}

/** Call once at startup. Returns cleanup function. */
export function initColorMode(): () => void {
  const mode = getColorMode();
  applyDarkClass(resolveMode(mode));

  const mql = window.matchMedia("(prefers-color-scheme: dark)");
  const handler = () => {
    if (getColorMode() === "system") {
      applyDarkClass(resolveMode("system"));
    }
  };
  mql.addEventListener("change", handler);
  return () => mql.removeEventListener("change", handler);
}
