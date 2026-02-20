/**
 * Plugin surface registry — decouples message routing from rendering backend.
 *
 * Each "surface" is a message sender function. The registry doesn't care
 * whether it's backed by an iframe postMessage or a Tauri webview emit.
 * PluginViewport registers on mount, unregisters on unmount.
 */

export type MessageSender = (event: string, data: unknown) => void;

const surfaces = new Map<string, MessageSender>();

/** Register a plugin's rendering surface. Called on mount. */
export function registerSurface(
  pluginId: string,
  sender: MessageSender,
): void {
  surfaces.set(pluginId, sender);
}

/** Unregister a plugin's surface. Called on unmount or stop. */
export function unregisterSurface(pluginId: string): void {
  surfaces.delete(pluginId);
}

/** Send an event to a specific plugin. */
export function sendToSurface(
  pluginId: string,
  event: string,
  data: unknown,
): void {
  surfaces.get(pluginId)?.(event, data);
}

/** Broadcast an event to all active surfaces. */
export function broadcastToSurfaces(event: string, data: unknown): void {
  for (const sender of surfaces.values()) {
    try {
      sender(event, data);
    } catch {
      // surface may be torn down
    }
  }
}

/** Build the URL for a plugin's UI surface. */
export function buildPluginUrl(
  port: number,
  uiPath: string,
  theme: string,
): string {
  const sep = uiPath.includes("?") ? "&" : "?";
  return `http://localhost:${port}${uiPath}${sep}nexus_theme=${theme}`;
}
