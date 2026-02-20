import { useState } from "react"
import { Button } from "@heroui/react"
import { cn } from "../lib/utils"

interface CodeBlockProps {
  /** The text content to display */
  text: string
  /** Label for the copy button (accessibility) */
  copyLabel?: string
  className?: string
}

function CopyButton({ text, label }: { text: string; label?: string }) {
  const [copied, setCopied] = useState(false)

  async function copy() {
    await navigator.clipboard.writeText(text)
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }

  return (
    <Button
      isIconOnly
      size="sm"
      variant="light"
      onPress={copy}
      className="absolute top-2 right-2"
      title={label ?? "Copy to clipboard"}
    >
      {copied ? (
        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" className="text-success">
          <polyline points="20 6 9 17 4 12" />
        </svg>
      ) : (
        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" className="text-default-400">
          <rect width="14" height="14" x="8" y="8" rx="2" ry="2" />
          <path d="M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2" />
        </svg>
      )}
    </Button>
  )
}

export function CodeBlock({ text, copyLabel, className }: CodeBlockProps) {
  return (
    <div className={cn("relative", className)}>
      <pre className="bg-background border border-default-100 rounded-[8px] p-3 text-[11px] text-default-500 font-mono overflow-x-auto leading-relaxed whitespace-pre-wrap break-all">
        {text}
      </pre>
      <CopyButton text={text} label={copyLabel} />
    </div>
  )
}
