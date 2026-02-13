import { useRef } from "react";
import { invoke } from "@tauri-apps/api/core";

const STORAGE_KEY = "nexus:notifications-enabled";

/** Minimum interval between OS notifications (ms). */
const THROTTLE_MS = 15_000;

/** Send a native OS notification via our cross-platform Rust command. */
export async function nativeNotify(title: string, body: string) {
  await invoke("send_notification", { title, body });
}

/** Check whether the user has disabled notifications in app settings. */
export function notificationsEnabled(): boolean {
  return localStorage.getItem(STORAGE_KEY) !== "false";
}

/** Toggle the app-level notification preference. */
export function setNotificationsEnabled(enabled: boolean) {
  localStorage.setItem(STORAGE_KEY, String(enabled));
}

export function useOsNotification() {
  const lastAt = useRef(0);

  function notify(title: string, body: string, queueSize: number) {
    if (document.hasFocus() || !notificationsEnabled()) return;

    const now = Date.now();
    const isFirst = queueSize <= 1;
    const throttleOk = now - lastAt.current >= THROTTLE_MS;

    if (isFirst || throttleOk) {
      lastAt.current = now;
      nativeNotify(title, body);
    }
  }

  return { notify };
}
