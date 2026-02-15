/**
 * Plugin iframe bridge — broadcasts Nexus system events to all plugin iframes
 * via postMessage. Plugin SDK listens for these on the other side.
 *
 * Message protocol:
 *   { type: "nexus:system", event: string, data: unknown }
 */

export interface NexusHostEvent {
  type: "nexus:system";
  event: string;
  data: unknown;
}

/**
 * Post a system event to every mounted plugin iframe.
 * Iframes are identified by the `data-nexus-plugin` attribute.
 */
export function broadcastToPlugins(event: string, data: unknown): void {
  const message: NexusHostEvent = { type: "nexus:system", event, data };
  const iframes = document.querySelectorAll<HTMLIFrameElement>(
    "iframe[data-nexus-plugin]"
  );
  for (const iframe of iframes) {
    try {
      iframe.contentWindow?.postMessage(message, "*");
    } catch {
      // iframe might be cross-origin or unmounted — ignore
    }
  }
}
