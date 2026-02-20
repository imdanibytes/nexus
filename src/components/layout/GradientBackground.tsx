import { GradientBackground as NxGradientBackground } from "@imdanibytes/nexus-ui"
import type { BlobConfig } from "@imdanibytes/nexus-ui"

const BLOBS: BlobConfig[] = [
  { size: 800, x: "5%",  y: "10%", color: "primary",   opacity: 0.10, drift: "drift-1", duration: "30s" },
  { size: 700, x: "75%", y: "5%",  color: "secondary", opacity: 0.08, drift: "drift-2", duration: "35s" },
  { size: 750, x: "60%", y: "60%", color: "primary",   opacity: 0.06, drift: "drift-3", duration: "28s" },
  { size: 650, x: "20%", y: "70%", color: "secondary", opacity: 0.05, drift: "drift-4", duration: "40s" },
  { size: 600, x: "45%", y: "30%", color: "primary",   opacity: 0.04, drift: "drift-1", duration: "33s" },
]

export function GradientBackground() {
  return <NxGradientBackground blobs={BLOBS} />
}
