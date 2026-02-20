// Provider
export { NexusProvider, useNexus } from "./provider/nexus-provider"

// Components
export { GradientBackground } from "./components/GradientBackground"
export type { BlobConfig } from "./components/GradientBackground"
export { Surface } from "./components/Surface"
export { SearchBar } from "./components/SearchBar"
export { StatusDot } from "./components/StatusDot"
export { CodeBlock } from "./components/CodeBlock"
export { EmptyState } from "./components/EmptyState"

// Utilities
export { cn } from "./lib/utils"

// Toast — re-exported from sonner for convenience
export { toast, Toaster } from "sonner"

// Hooks
export { useIsMobile } from "./hooks/use-mobile"
