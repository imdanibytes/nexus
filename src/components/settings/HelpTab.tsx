import { HelpCircle, Monitor, Blocks } from "lucide-react";

const pluginIndicators = [
  {
    color: "bg-nx-success",
    animation: undefined,
    label: "Running (viewport loaded)",
    description: "Plugin is running and its UI is cached in memory. Switching to it is instant.",
  },
  {
    color: "bg-nx-success",
    animation: "pulse-status 2s ease-in-out infinite",
    label: "Running (viewport unloaded)",
    description:
      "Plugin is running but its UI is not in memory. Clicking it will load the interface, which may take a moment.",
  },
  {
    color: "bg-nx-text-muted",
    animation: undefined,
    label: "Stopped",
    description: "Plugin container is not running. Start it from the sidebar or the plugin viewport.",
  },
  {
    color: "bg-nx-error",
    animation: undefined,
    label: "Error",
    description: "Plugin container encountered an error. Check the logs for details.",
  },
  {
    color: "bg-nx-warning",
    animation: undefined,
    label: "Installing",
    description: "Plugin is being installed or its image is being pulled.",
  },
];

const extensionIndicators = [
  {
    color: "bg-nx-success",
    label: "Enabled",
    description: "Extension is running and available to plugins.",
  },
  {
    color: "bg-nx-text-muted",
    label: "Disabled",
    description: "Extension is installed but not running. Enable it from the sidebar or Settings.",
  },
];

function StatusDot({ color, animation }: { color: string; animation?: string }) {
  return (
    <span className="relative shrink-0 w-2.5 h-2.5 flex items-center justify-center">
      {animation && (
        <span
          className={`absolute inset-0 rounded-full ${color} opacity-30`}
          style={{ animation }}
        />
      )}
      <span
        className={`w-2 h-2 rounded-full ${color}`}
        style={animation ? { animation } : undefined}
      />
    </span>
  );
}

export function HelpTab() {
  return (
    <div className="space-y-6">
      {/* Plugin status indicators */}
      <section className="bg-nx-surface rounded-[var(--radius-card)] border border-nx-border p-5">
        <div className="flex items-center gap-2 mb-2">
          <Monitor size={15} strokeWidth={1.5} className="text-nx-text-muted" />
          <h3 className="text-[14px] font-semibold text-nx-text">
            Plugin Status Indicators
          </h3>
        </div>
        <p className="text-[11px] text-nx-text-ghost mb-4">
          The colored dot next to each plugin in the sidebar indicates its current state.
          Plugin viewports are cached for instant switching and automatically unloaded after
          5 minutes of inactivity to free memory.
        </p>
        <div className="space-y-1">
          {pluginIndicators.map((item) => (
            <div
              key={item.label}
              className="flex items-start gap-3 px-3 py-2.5 rounded-[var(--radius-button)] bg-nx-deep border border-nx-border-subtle"
            >
              <div className="pt-1">
                <StatusDot color={item.color} animation={item.animation} />
              </div>
              <div className="min-w-0">
                <p className="text-[12px] text-nx-text font-medium">
                  {item.label}
                </p>
                <p className="text-[11px] text-nx-text-ghost mt-0.5">
                  {item.description}
                </p>
              </div>
            </div>
          ))}
        </div>
      </section>

      {/* Extension status indicators */}
      <section className="bg-nx-surface rounded-[var(--radius-card)] border border-nx-border p-5">
        <div className="flex items-center gap-2 mb-2">
          <Blocks size={15} strokeWidth={1.5} className="text-nx-text-muted" />
          <h3 className="text-[14px] font-semibold text-nx-text">
            Extension Status Indicators
          </h3>
        </div>
        <p className="text-[11px] text-nx-text-ghost mb-4">
          Extensions are native host processes that provide operations to plugins.
          Clicking an extension in the sidebar opens its detail in Settings.
        </p>
        <div className="space-y-1">
          {extensionIndicators.map((item) => (
            <div
              key={item.label}
              className="flex items-start gap-3 px-3 py-2.5 rounded-[var(--radius-button)] bg-nx-deep border border-nx-border-subtle"
            >
              <div className="pt-1">
                <StatusDot color={item.color} />
              </div>
              <div className="min-w-0">
                <p className="text-[12px] text-nx-text font-medium">
                  {item.label}
                </p>
                <p className="text-[11px] text-nx-text-ghost mt-0.5">
                  {item.description}
                </p>
              </div>
            </div>
          ))}
        </div>
      </section>

      {/* Keyboard / tips */}
      <section className="bg-nx-surface rounded-[var(--radius-card)] border border-nx-border p-5">
        <div className="flex items-center gap-2 mb-2">
          <HelpCircle size={15} strokeWidth={1.5} className="text-nx-text-muted" />
          <h3 className="text-[14px] font-semibold text-nx-text">
            Tips
          </h3>
        </div>
        <div className="space-y-2 text-[12px] text-nx-text-secondary">
          <p>
            Hover over any plugin or extension in the sidebar to reveal a menu
            button with quick actions like start, stop, enable, disable, or
            remove.
          </p>
          <p>
            The sidebar remembers your last-viewed plugin and settings tab across app restarts.
          </p>
          <p>
            Plugin viewports stay cached in memory while you switch between tabs.
            After 5 minutes of inactivity, unused viewports are automatically unloaded.
          </p>
        </div>
      </section>
    </div>
  );
}
