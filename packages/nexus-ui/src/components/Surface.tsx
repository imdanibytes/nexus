import * as React from "react"
import { cn } from "../lib/utils"

interface SurfaceProps extends React.HTMLAttributes<HTMLDivElement> {
  /** "default" = standard glass card, "strong" = structural chrome (no rounding, stronger blur) */
  variant?: "default" | "strong"
}

export const Surface = React.forwardRef<HTMLDivElement, SurfaceProps>(
  ({ variant = "default", className, children, ...props }, ref) => {
    return (
      <div
        ref={ref}
        className={cn(
          variant === "default" ? "nx-glass" : "nx-glass-strong",
          className,
        )}
        {...props}
      >
        {children}
      </div>
    )
  },
)

Surface.displayName = "Surface"
