import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { useAppStore } from "../stores/appStore";

interface PluginUpdateEvent {
  plugin_id: string;
  stage: "stopping" | "pulling" | "starting";
}

/**
 * Listens for granular update-stage events from the backend and
 * cycles the busy overlay through stopping → updating → starting.
 */
export function useUpdateEvents() {
  const { setBusy } = useAppStore();

  useEffect(() => {
    const unlisten = listen<PluginUpdateEvent>("nexus://plugin-update", (event) => {
      const { plugin_id, stage } = event.payload;

      switch (stage) {
        case "stopping":
          setBusy(plugin_id, "stopping");
          break;
        case "pulling":
          setBusy(plugin_id, "updating");
          break;
        case "starting":
          setBusy(plugin_id, "starting");
          break;
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [setBusy]);
}
