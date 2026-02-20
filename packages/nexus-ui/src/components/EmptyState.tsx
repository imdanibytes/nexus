import type { ReactNode, ElementType } from "react"
import { cn } from "../lib/utils"

interface EmptyStateProps {
  /** Lucide icon component or any React element type */
  icon: ElementType
  /** Primary message */
  title: string
  /** Secondary description */
  description?: string
  /** Optional action slot (button, link, etc.) */
  action?: ReactNode
  className?: string
}

export function EmptyState({ icon: Icon, title, description, action, className }: EmptyStateProps) {
  return (
    <div className={cn("text-center py-12 rounded-xl border border-dashed border-default-200/50", className)}>
      <Icon size={28} className="mx-auto mb-3 text-default-400" />
      <p className="text-sm text-default-500">{title}</p>
      {description && (
        <p className="text-[11px] text-default-400 mt-1">{description}</p>
      )}
      {action && <div className="mt-3">{action}</div>}
    </div>
  )
}
