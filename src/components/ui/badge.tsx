import * as React from "react"
import { cva, type VariantProps } from "class-variance-authority"
import { Slot } from "radix-ui"

import { cn } from "@/lib/utils"

const badgeVariants = cva(
  "inline-flex items-center justify-center rounded-[var(--radius-tag)] border border-transparent px-1.5 py-0.5 text-[10px] font-semibold w-fit whitespace-nowrap shrink-0 [&>svg]:size-3 gap-1 [&>svg]:pointer-events-none transition-[color,box-shadow] overflow-hidden uppercase tracking-wide",
  {
    variants: {
      variant: {
        default: "bg-primary text-primary-foreground",
        secondary: "bg-nx-overlay text-nx-text-ghost",
        destructive: "bg-destructive/15 text-destructive",
        outline: "border-border text-foreground",
        success: "bg-nx-success-muted text-nx-success",
        warning: "bg-nx-warning-muted text-nx-warning",
        error: "bg-nx-error-muted text-nx-error",
        info: "bg-nx-info-muted text-nx-info",
        accent: "bg-nx-accent-muted text-nx-accent",
        highlight: "bg-nx-highlight-muted text-nx-highlight",
      },
    },
    defaultVariants: {
      variant: "default",
    },
  }
)

function Badge({
  className,
  variant = "default",
  asChild = false,
  ...props
}: React.ComponentProps<"span"> &
  VariantProps<typeof badgeVariants> & { asChild?: boolean }) {
  const Comp = asChild ? Slot.Root : "span"

  return (
    <Comp
      data-slot="badge"
      data-variant={variant}
      className={cn(badgeVariants({ variant }), className)}
      {...props}
    />
  )
}

// eslint-disable-next-line react-refresh/only-export-components
export { Badge, badgeVariants }
