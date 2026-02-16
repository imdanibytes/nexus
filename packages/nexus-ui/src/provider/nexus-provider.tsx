import * as React from "react"
import { TooltipProvider } from "../components/tooltip"
import { Toaster } from "../components/sonner"

interface NexusContextValue {
  apiUrl: string
}

const NexusContext = React.createContext<NexusContextValue>({
  apiUrl: "http://localhost:9600",
})

export function useNexus() {
  return React.useContext(NexusContext)
}

interface NexusProviderProps {
  children: React.ReactNode
  apiUrl?: string
  toaster?: boolean
}

function applyThemeAttr(theme?: string) {
  if (theme && theme !== "default") {
    document.documentElement.setAttribute("data-theme", theme)
  } else {
    document.documentElement.removeAttribute("data-theme")
  }
}

/** Read nexus_theme from the iframe URL query string (set by the host). */
function getInitialTheme(): string | undefined {
  try {
    const params = new URLSearchParams(window.location.search)
    return params.get("nexus_theme") ?? undefined
  } catch {
    return undefined
  }
}

export function NexusProvider({
  children,
  apiUrl = "http://localhost:9600",
  toaster = true,
}: NexusProviderProps) {
  // Apply initial theme synchronously from URL param (fastest, no network)
  const [themeApplied] = React.useState(() => {
    const initial = getInitialTheme()
    if (initial) applyThemeAttr(initial)
    return !!initial
  })

  // Fetch the active theme from the host API if URL param wasn't set
  React.useEffect(() => {
    if (themeApplied) return
    fetch(`${apiUrl}/api/v1/theme`)
      .then((r) => r.json())
      .then((data: { theme?: string }) => {
        applyThemeAttr(data.theme)
      })
      .catch(() => {
        // Host not running — stay on default
      })
  }, [apiUrl, themeApplied])

  // Listen for host system events (theme, language, etc.)
  React.useEffect(() => {
    function onMessage(e: MessageEvent) {
      const msg = e.data
      if (msg?.type !== "nexus:system") return

      if (msg.event === "theme_changed") {
        const theme = (msg.data as { theme?: string })?.theme
        applyThemeAttr(theme)
      }
    }
    window.addEventListener("message", onMessage)
    return () => window.removeEventListener("message", onMessage)
  }, [])

  return (
    <NexusContext.Provider value={{ apiUrl }}>
      <TooltipProvider>
        {children}
        {toaster && <Toaster />}
      </TooltipProvider>
    </NexusContext.Provider>
  )
}
