/**
 * Plugin iframe bridge — broadcasts Nexus system events to all plugin surfaces.
 * Plugin SDK listens for these on the other side.
 *
 * Message protocol:
 *   { type: "nexus:system", event: string, data: unknown }
 */

import { broadcastToSurfaces } from "./pluginSurface";

export interface NexusHostEvent {
  type: "nexus:system";
  event: string;
  data: unknown;
}

/**
 * Post a system event to every active plugin surface.
 * Delegates to the surface registry — no DOM queries needed.
 */
export function broadcastToPlugins(event: string, data: unknown): void {
  broadcastToSurfaces(event, data);
}
