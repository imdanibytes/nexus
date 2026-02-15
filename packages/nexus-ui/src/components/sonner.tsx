import {
  CircleCheckIcon,
  InfoIcon,
  Loader2Icon,
  OctagonXIcon,
  TriangleAlertIcon,
} from "lucide-react"
import { toast, Toaster as Sonner, type ToasterProps } from "sonner"

function Toaster({ ...props }: ToasterProps) {
  return (
    <Sonner
      theme="dark"
      className="toaster group"
      icons={{
        success: <CircleCheckIcon className="size-4" />,
        info: <InfoIcon className="size-4" />,
        warning: <TriangleAlertIcon className="size-4" />,
        error: <OctagonXIcon className="size-4" />,
        loading: <Loader2Icon className="size-4 animate-spin" />,
      }}
      style={
        {
          "--normal-bg": "var(--color-nx-surface)",
          "--normal-text": "var(--color-nx-text)",
          "--normal-border": "var(--color-nx-border)",
          "--success-bg": "var(--color-nx-success-muted)",
          "--success-text": "var(--color-nx-success)",
          "--success-border": "var(--color-nx-success)",
          "--error-bg": "var(--color-nx-error-muted)",
          "--error-text": "var(--color-nx-error)",
          "--error-border": "var(--color-nx-error)",
          "--warning-bg": "var(--color-nx-warning-muted)",
          "--warning-text": "var(--color-nx-warning)",
          "--warning-border": "var(--color-nx-warning)",
          "--border-radius": "var(--radius-card)",
        } as React.CSSProperties
      }
      {...props}
    />
  )
}

export { toast, Toaster }
