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

// Types
export type { ToolCallStatus, TimingSpan, TimingSpanMarker } from "./types"

// Chat components
export { MarkdownText } from "./components/MarkdownText"
export { ToolFallback } from "./components/ToolFallback"
export { Composer } from "./components/Composer"
export type { ComposerProps } from "./components/Composer"

// Data visualization
export { TimingWaterfall } from "./components/TimingWaterfall"
export { ContextRing } from "./components/ContextRing"
export type { ContextRingProps } from "./components/ContextRing"

// Layout
export { SettingsShell } from "./components/SettingsShell"
export type { SettingsShellProps, SettingsTab } from "./components/SettingsShell"

// Atoms
export { TooltipIconButton } from "./components/TooltipIconButton"
export type { TooltipIconButtonProps } from "./components/TooltipIconButton"

// Hooks
export { useAutoScroll } from "./hooks/use-auto-scroll"
export { useScrollLock } from "./hooks/use-scroll-lock"
export { useIsMobile } from "./hooks/use-mobile"

// Utils
export { cn } from "./lib/utils"
export { formatToolDescription } from "./lib/tool-descriptions"
export { remarkHighlight, remarkSubSuperscript, remarkAbbreviations } from "./lib/remark-plugins"

// Toast — re-exported from sonner for convenience
export { toast, Toaster } from "sonner"
