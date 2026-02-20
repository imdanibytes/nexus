/**
 * Ambient gradient background — uses HeroUI theme tokens for colors.
 *
 * Blobs are radial-gradient divs (no CSS `filter: blur()` — GPU-safe).
 * Layer-promoted via `will-change: transform`, animated exclusively with
 * `transform: translate3d()` so the GPU composites cached textures at new
 * positions each frame — no Gaussian blur recomputation.
 *
 * Drift keyframes (`drift-1` through `drift-4`) are defined in base.css.
 */

import { cn } from "../lib/utils"

export interface BlobConfig {
  /** Blob diameter in px */
  size: number
  /** CSS left position (e.g. "10%", "200px") */
  x: string
  /** CSS top position */
  y: string
  /** HeroUI color token name — "primary", "secondary", "success", etc. */
  color: string
  /** Opacity for the radial gradient (0–1) */
  opacity: number
  /** Keyframe name from base.css — "drift-1", "drift-2", etc. */
  drift: string
  /** Animation duration (e.g. "25s") */
  duration: string
}

const DEFAULT_BLOBS: BlobConfig[] = [
  { size: 700, x: "10%", y: "5%", color: "primary", opacity: 0.08, drift: "drift-1", duration: "25s" },
  { size: 600, x: "70%", y: "15%", color: "secondary", opacity: 0.06, drift: "drift-2", duration: "30s" },
  { size: 550, x: "30%", y: "65%", color: "primary", opacity: 0.05, drift: "drift-4", duration: "22s" },
]

interface GradientBackgroundProps {
  /** Custom blob configurations. Defaults to a 3-blob teal/violet/blue set. */
  blobs?: BlobConfig[]
  className?: string
}

export function GradientBackground({ blobs = DEFAULT_BLOBS, className }: GradientBackgroundProps) {
  return (
    <div className={cn("fixed inset-0 -z-10 overflow-hidden bg-background", className)}>
      {blobs.map((blob, i) => (
        <div
          key={i}
          style={{
            position: "absolute",
            left: blob.x,
            top: blob.y,
            width: blob.size,
            height: blob.size,
            borderRadius: "50%",
            background: `radial-gradient(circle, hsl(var(--heroui-${blob.color}) / ${blob.opacity}) 0%, transparent 70%)`,
            willChange: "transform",
            animation: `${blob.drift} ${blob.duration} ease-in-out infinite alternate`,
            transform: "translate3d(0,0,0)",
          }}
        />
      ))}
    </div>
  )
}
