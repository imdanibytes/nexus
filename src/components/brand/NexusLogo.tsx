/**
 * Nexus wordmark — typographic logo using Geist font.
 * Uses currentColor so it inherits from its container.
 * When collapsed, shows just "n." as a compact mark.
 */
export function NexusLogo({
  className,
  collapsed,
}: {
  className?: string;
  collapsed?: boolean;
}) {
  return (
    <div className={className}>
      <span className="text-lg font-bold tracking-tight select-none">
        {collapsed ? "n" : "nexus"}
      </span>
      <span className="text-lg font-bold text-primary select-none">.</span>
    </div>
  );
}
