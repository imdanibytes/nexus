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

export function NexusProvider({
  children,
  apiUrl = "http://localhost:9600",
  toaster = true,
}: NexusProviderProps) {
  React.useEffect(() => {
    const id = "nexus-theme-css"
    if (document.getElementById(id)) return

    const link = document.createElement("link")
    link.id = id
    link.rel = "stylesheet"
    link.href = `${apiUrl}/api/v1/theme.css`
    link.onerror = () => {
      // Host not running — build-time tokens still provide all values
    }
    document.head.appendChild(link)

    return () => {
      document.getElementById(id)?.remove()
    }
  }, [apiUrl])

  return (
    <NexusContext.Provider value={{ apiUrl }}>
      <TooltipProvider>
        {children}
        {toaster && <Toaster />}
      </TooltipProvider>
    </NexusContext.Provider>
  )
}
