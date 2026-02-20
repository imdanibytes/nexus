import { cn } from "../lib/utils"

type Status = "running" | "stopped" | "error" | "busy" | "installing"

const COLOR: Record<Status, string> = {
  running: "bg-success",
  stopped: "bg-default-400",
  error: "bg-danger",
  busy: "bg-warning",
  installing: "bg-warning",
}

interface StatusDotProps {
  status: Status
  /** Show animated ping ring (default: true when running) */
  ping?: boolean
  className?: string
}

export function StatusDot({ status, ping, className }: StatusDotProps) {
  const showPing = ping ?? status === "running"

  return (
    <span className={cn("flex items-center justify-center w-4 shrink-0", className)}>
      <span className="relative flex h-2 w-2">
        {showPing && (
          <span
            className={cn(
              "absolute inline-flex h-full w-full rounded-full opacity-75 animate-ping",
              COLOR[status],
            )}
          />
        )}
        <span
          className={cn(
            "relative inline-flex rounded-full h-2 w-2",
            COLOR[status],
          )}
        />
      </span>
    </span>
  )
}
