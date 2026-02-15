import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { useAppStore } from "../stores/appStore";

interface DevRebuildEvent {
  plugin_id: string;
  status: "started" | "building" | "restarting" | "complete" | "error";
  message: string;
}

export function useDevRebuild() {
  const { setBusy, addNotification } = useAppStore();

  useEffect(() => {
    const unlisten = listen<DevRebuildEvent>("nexus://dev-rebuild", (event) => {
      const { plugin_id, status, message } = event.payload;

      switch (status) {
        case "started":
        case "building":
        case "restarting":
          setBusy(plugin_id, "rebuilding");
          break;
        case "complete":
          setBusy(plugin_id, null);
          addNotification("Dev rebuild complete", "success");
          break;
        case "error":
          setBusy(plugin_id, null);
          addNotification(`Dev rebuild failed: ${message}`, "error");
          break;
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [setBusy, addNotification]);
}
