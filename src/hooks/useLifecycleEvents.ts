import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { useAppStore } from "../stores/appStore";
import type { LifecycleEvent } from "../types/lifecycle";

/**
 * Single lifecycle event listener. Dispatches all backend state-change events
 * to the store. Mount once in App.tsx.
 *
 * To add a new operation:
 * 1. Add variants to LifecycleEvent (both Rust enum and TS type)
 * 2. Add cases to the switch below
 * 3. Backend emits via lifecycle_events::emit()
 */
export function useLifecycleEvents() {
  const {
    setBusy,
    setExtensionBusy,
    updatePlugin,
    removePlugin,
    updateExtension,
    removeExtension,
    setInstallStatus,
    addNotification,
  } = useAppStore();

  useEffect(() => {
    const unlisten = listen<LifecycleEvent>("nexus://lifecycle", (event) => {
      const e = event.payload;

      switch (e.kind) {
        // -- Plugin lifecycle --
        case "plugin:starting":
          setBusy(e.plugin_id, "starting");
          break;
        case "plugin:started":
          setBusy(e.plugin.manifest.id, null);
          updatePlugin(e.plugin);
          break;
        case "plugin:stopping":
          setBusy(e.plugin_id, "stopping");
          break;
        case "plugin:stopped":
          setBusy(e.plugin.manifest.id, null);
          updatePlugin(e.plugin);
          break;
        case "plugin:removing":
          setBusy(e.plugin_id, "removing");
          break;
        case "plugin:removed":
          setBusy(e.plugin_id, null);
          removePlugin(e.plugin_id);
          break;
        case "plugin:installing":
          setInstallStatus(e.message);
          break;
        case "plugin:installed":
          setInstallStatus(null);
          updatePlugin(e.plugin);
          break;
        case "plugin:error":
          setBusy(e.plugin_id, null);
          setInstallStatus(null);
          addNotification(`${e.action} failed: ${e.message}`, "error");
          break;

        // -- Plugin update stages --
        case "plugin:update_stage":
          switch (e.stage) {
            case "stopping":
              setBusy(e.plugin_id, "stopping");
              break;
            case "pulling":
              setBusy(e.plugin_id, "updating");
              break;
            case "starting":
              setBusy(e.plugin_id, "starting");
              break;
          }
          break;

        // -- Plugin dev rebuild --
        case "plugin:rebuild":
          switch (e.status) {
            case "started":
            case "building":
            case "restarting":
              setBusy(e.plugin_id, "rebuilding");
              break;
            case "complete":
              setBusy(e.plugin_id, null);
              addNotification("Dev rebuild complete", "success");
              break;
            case "error":
              setBusy(e.plugin_id, null);
              addNotification(`Dev rebuild failed: ${e.message}`, "error");
              break;
          }
          break;

        // -- Extension lifecycle --
        case "extension:enabling":
          setExtensionBusy(e.ext_id, "enabling");
          break;
        case "extension:enabled":
          setExtensionBusy(e.extension.id, null);
          updateExtension(e.extension);
          break;
        case "extension:disabling":
          setExtensionBusy(e.ext_id, "disabling");
          break;
        case "extension:disabled":
          setExtensionBusy(e.extension.id, null);
          updateExtension(e.extension);
          break;
        case "extension:removing":
          setExtensionBusy(e.ext_id, "removing");
          break;
        case "extension:removed":
          setExtensionBusy(e.ext_id, null);
          removeExtension(e.ext_id);
          break;
        case "extension:installing":
          setExtensionBusy(e.ext_id, "enabling");
          break;
        case "extension:installed":
          setExtensionBusy(e.extension.id, null);
          updateExtension(e.extension);
          break;
        case "extension:error":
          setExtensionBusy(e.ext_id, null);
          addNotification(`Extension ${e.action} failed: ${e.message}`, "error");
          break;
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [setBusy, setExtensionBusy, updatePlugin, removePlugin, updateExtension, removeExtension, setInstallStatus, addNotification]);
}
