import { useEffect, useState } from "react"
import { Input } from "@heroui/react"
import { cn } from "../lib/utils"

interface SearchBarProps {
  /** Current search value (controlled) or initial value (uncontrolled) */
  value?: string
  /** Called with the debounced search query */
  onChange: (query: string) => void
  /** Placeholder text */
  placeholder?: string
  /** Debounce delay in ms (default 300) */
  delay?: number
  /** Optional icon element rendered as Input startContent */
  icon?: React.ReactNode
  className?: string
}

export function SearchBar({
  value: controlledValue,
  onChange,
  placeholder = "Search...",
  delay = 300,
  icon,
  className,
}: SearchBarProps) {
  const [internal, setInternal] = useState(controlledValue ?? "")

  // Sync controlled value
  useEffect(() => {
    if (controlledValue !== undefined) setInternal(controlledValue)
  }, [controlledValue])

  // Debounce
  useEffect(() => {
    const timer = setTimeout(() => onChange(internal), delay)
    return () => clearTimeout(timer)
  }, [internal, delay, onChange])

  return (
    <Input
      type="text"
      value={internal}
      onValueChange={setInternal}
      placeholder={placeholder}
      isClearable
      onClear={() => setInternal("")}
      startContent={icon}
      className={cn(className)}
    />
  )
}
