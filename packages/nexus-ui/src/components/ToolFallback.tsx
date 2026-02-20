import { createContext, memo, useCallback, useContext, useRef, useState, type FC } from "react";
import { CheckIcon, ChevronDownIcon, CopyIcon } from "lucide-react";
import { motion, AnimatePresence } from "framer-motion";
import { cn } from "../lib/utils";
import { useScrollLock } from "../hooks/use-scroll-lock";
import { formatToolDescription } from "../lib/tool-descriptions";
import { MarkdownText } from "./MarkdownText";
import type { ToolCallStatus } from "../types";

const ANIMATION_DURATION = 200;

// ── Root ──

export type ToolFallbackRootProps = React.ComponentProps<"div"> & {
  open?: boolean;
  onOpenChange?: (open: boolean) => void;
  defaultOpen?: boolean;
};

function ToolFallbackRoot({
  className,
  open: controlledOpen,
  onOpenChange: controlledOnOpenChange,
  defaultOpen = false,
  children,
  ...props
}: ToolFallbackRootProps) {
  const collapsibleRef = useRef<HTMLDivElement>(null);
  const [uncontrolledOpen, setUncontrolledOpen] = useState(defaultOpen);
  const lockScroll = useScrollLock(collapsibleRef, ANIMATION_DURATION);

  const isControlled = controlledOpen !== undefined;
  const isOpen = isControlled ? controlledOpen : uncontrolledOpen;

  const handleToggle = useCallback(() => {
    const next = !isOpen;
    if (!next) {
      lockScroll();
    }
    if (!isControlled) {
      setUncontrolledOpen(next);
    }
    controlledOnOpenChange?.(next);
  }, [isOpen, lockScroll, isControlled, controlledOnOpenChange]);

  return (
    <ToolFallbackContext.Provider value={{ isOpen, toggle: handleToggle }}>
      <div
        ref={collapsibleRef}
        data-slot="tool-fallback-root"
        data-state={isOpen ? "open" : "closed"}
        className={cn(
          "aui-tool-fallback-root group/tool-fallback-root w-full cursor-pointer select-none nx-glass py-3",
          className,
        )}
        onClick={handleToggle}
        style={
          {
            "--animation-duration": `${ANIMATION_DURATION}ms`,
          } as React.CSSProperties
        }
        {...props}
      >
        {children}
      </div>
    </ToolFallbackContext.Provider>
  );
}

// Context for open state
const ToolFallbackContext = createContext<{ isOpen: boolean; toggle: () => void }>({
  isOpen: false,
  toggle: () => {},
});

// ── Trigger ──

type ToolStatusType = NonNullable<ToolCallStatus>["type"];

const Throbber = () => (
  <span className="aui-tool-throbber relative flex size-4 shrink-0 items-center justify-center">
    <span className="absolute size-2 rounded-full bg-primary/80 animate-ping" />
    <span className="relative size-2 rounded-full bg-primary" />
  </span>
);

const StatusDot = ({ color }: { color: string }) => (
  <span className={cn("size-2 rounded-full shrink-0", color)} />
);

const statusIconMap: Record<ToolStatusType, React.ElementType> = {
  running: Throbber,
  complete: () => <StatusDot color="bg-success" />,
  incomplete: () => <StatusDot color="bg-danger" />,
};

function ToolFallbackTrigger({
  toolName,
  argsText,
  status,
  className,
  ...props
}: React.ComponentProps<"button"> & {
  toolName: string;
  argsText?: string;
  status?: ToolCallStatus;
}) {
  const { isOpen } = useContext(ToolFallbackContext);
  const statusType = status?.type ?? "complete";
  const isRunning = statusType === "running";
  const isCancelled =
    status?.type === "incomplete" && status.reason === "cancelled";

  const Icon = statusIconMap[statusType];

  // Human-readable description
  const description = formatToolDescription(toolName, argsText);

  return (
    <button
      type="button"
      data-slot="tool-fallback-trigger"
      data-state={isOpen ? "open" : "closed"}
      className={cn(
        "aui-tool-fallback-trigger group/trigger flex w-full cursor-pointer items-center gap-2 px-4 text-sm transition-colors",
        className,
      )}
      {...props}
    >
      <Icon
        data-slot="tool-fallback-trigger-icon"
        className={cn(
          "aui-tool-fallback-trigger-icon size-4 shrink-0",
          isCancelled && "text-default-400",
        )}
      />
      <span
        data-slot="tool-fallback-trigger-label"
        className={cn(
          "aui-tool-fallback-trigger-label grow text-left leading-none",
          isCancelled && "text-default-400 line-through",
          isRunning && "tool-pulse motion-reduce:animate-none",
        )}
      >
        {description}
      </span>
      <ChevronDownIcon
        data-slot="tool-fallback-trigger-chevron"
        className={cn(
          "aui-tool-fallback-trigger-chevron size-4 shrink-0 text-default-400",
          "transition-transform duration-200 ease-out",
          isOpen ? "rotate-0" : "-rotate-90",
        )}
      />
    </button>
  );
}

// ── Content ──

function ToolFallbackContent({
  className,
  children,
}: {
  className?: string;
  children?: React.ReactNode;
}) {
  const { isOpen } = useContext(ToolFallbackContext);

  return (
    <AnimatePresence initial={false}>
      {isOpen && (
        <motion.div
          data-slot="tool-fallback-content"
          data-state="open"
          initial={{ height: 0, opacity: 0 }}
          animate={{ height: "auto", opacity: 1 }}
          exit={{ height: 0, opacity: 0 }}
          transition={{ duration: 0.2, ease: "easeOut" }}
          className={cn(
            "aui-tool-fallback-content overflow-hidden text-sm outline-none",
            className,
          )}
        >
          <div className="mt-3 flex flex-col gap-2 border-t border-default-200/50 pt-2">{children}</div>
        </motion.div>
      )}
    </AnimatePresence>
  );
}

// ── Args / Result / Error ──

// Parse args JSON into key-value pairs for display
function parseArgs(argsText: string): Record<string, unknown> | null {
  try {
    const parsed = JSON.parse(argsText);
    if (parsed && typeof parsed === "object" && !Array.isArray(parsed)) return parsed;
  } catch { /* fall through */ }
  return null;
}

function formatValue(value: unknown): string {
  if (typeof value === "string") return value;
  if (typeof value === "number" || typeof value === "boolean") return String(value);
  return JSON.stringify(value, null, 2);
}

function CopyButton({ text }: { text: string }) {
  const [copied, setCopied] = useState(false);
  return (
    <button
      type="button"
      onClick={(e) => {
        e.stopPropagation();
        navigator.clipboard.writeText(text).then(() => {
          setCopied(true);
          setTimeout(() => setCopied(false), 2000);
        });
      }}
      className="shrink-0 rounded-md p-1 text-default-400 transition-colors hover:bg-default-200/40 hover:text-default-600"
      aria-label="Copy"
    >
      {copied
        ? <CheckIcon className="size-3 text-success" />
        : <CopyIcon className="size-3" />}
    </button>
  );
}

function ToolFallbackArgs({
  argsText,
  className,
  ...props
}: React.ComponentProps<"div"> & { argsText?: string }) {
  if (!argsText) return null;

  const parsed = parseArgs(argsText);

  if (parsed) {
    const entries = Object.entries(parsed);
    return (
      <div
        data-slot="tool-fallback-args"
        className={cn("aui-tool-fallback-args px-4", className)}
        {...props}
      >
        <div className="mb-1 flex items-center justify-between">
          <p className="text-xs font-medium text-default-400">Request</p>
          <CopyButton text={argsText} />
        </div>
        <div className="flex flex-col gap-1">
          {entries.map(([key, value]) => (
            <div key={key} className="flex gap-2 text-xs">
              <span className="shrink-0 text-default-400 font-mono">{key}</span>
              <pre className="whitespace-pre-wrap break-all text-default-600 font-mono">
                {formatValue(value)}
              </pre>
            </div>
          ))}
        </div>
      </div>
    );
  }

  return (
    <div
      data-slot="tool-fallback-args"
      className={cn("aui-tool-fallback-args px-4", className)}
      {...props}
    >
      <div className="mb-1 flex items-center justify-between">
        <p className="text-xs font-medium text-default-400">Request</p>
        <CopyButton text={argsText} />
      </div>
      <pre className="aui-tool-fallback-args-value whitespace-pre-wrap text-xs text-default-500 font-mono">
        {argsText}
      </pre>
    </div>
  );
}

// Detect content type for smart rendering
const MD_PATTERN = /(?:^#{1,6}\s|^\s*[-*]\s|\*\*|__|\[.+\]\(.+\)|^```)/m;

function detectContentType(text: string): "json" | "markdown" | "plain" {
  const trimmed = text.trim();
  if (
    (trimmed.startsWith("{") && trimmed.endsWith("}")) ||
    (trimmed.startsWith("[") && trimmed.endsWith("]"))
  ) {
    try { JSON.parse(trimmed); return "json"; } catch { /* not json */ }
  }
  if (MD_PATTERN.test(trimmed)) return "markdown";
  return "plain";
}

const MAX_LINES = 20;

function truncateToLines(text: string, max: number): { truncated: string; isTruncated: boolean } {
  const lines = text.split("\n");
  if (lines.length <= max) return { truncated: text, isTruncated: false };
  return { truncated: lines.slice(0, max).join("\n"), isTruncated: true };
}

function ResultContent({ text }: { text: string }) {
  const [expanded, setExpanded] = useState(false);
  const [collapsedH, setCollapsedH] = useState<number | null>(null);
  const clampRef = useRef<HTMLDivElement>(null);
  const type = detectContentType(text);

  const displayText = type === "json"
    ? JSON.stringify(JSON.parse(text.trim()), null, 2)
    : text;

  const { isTruncated } = truncateToLines(displayText, MAX_LINES);

  // Measure the clamped height (MAX_LINES worth) on mount
  const measureRef = useCallback((node: HTMLDivElement | null) => {
    clampRef.current = node;
    if (node && collapsedH === null) {
      // Temporarily clamp to MAX_LINES via line-clamp, measure, then remove
      const style = node.style;
      const prev = {
        display: style.display,
        webkitLineClamp: style.webkitLineClamp,
        webkitBoxOrient: (style as unknown as Record<string, string>).webkitBoxOrient,
        overflow: style.overflow,
      };
      style.display = "-webkit-box";
      style.webkitLineClamp = String(MAX_LINES);
      (style as unknown as Record<string, string>).webkitBoxOrient = "vertical";
      style.overflow = "hidden";
      setCollapsedH(node.offsetHeight);
      style.display = prev.display;
      style.webkitLineClamp = prev.webkitLineClamp;
      (style as unknown as Record<string, string>).webkitBoxOrient = prev.webkitBoxOrient;
      style.overflow = prev.overflow;
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const fadeMask = isTruncated && !expanded
    ? { maskImage: "linear-gradient(to bottom, black 70%, transparent 100%)", WebkitMaskImage: "linear-gradient(to bottom, black 70%, transparent 100%)" }
    : undefined;

  const content = type === "markdown" ? (
    <div className="text-xs [&_.aui-md-p]:my-1 [&_.aui-md-p]:text-xs">
      <MarkdownText text={displayText} />
    </div>
  ) : (
    <pre className="whitespace-pre-wrap text-xs text-default-600 font-mono">
      {displayText}
    </pre>
  );

  const targetHeight = !isTruncated ? "auto"
    : expanded ? "auto"
    : (collapsedH ?? "auto");

  return (
    <div>
      <motion.div
        initial={false}
        animate={{ height: targetHeight }}
        transition={{ duration: 0.3, ease: [0.25, 0.1, 0.25, 1] }}
        className="overflow-hidden"
        style={fadeMask}
      >
        <div ref={measureRef}>
          {content}
        </div>
      </motion.div>
      {isTruncated && (
        <motion.button
          type="button"
          onClick={(e: React.MouseEvent) => { e.stopPropagation(); setExpanded(!expanded); }}
          className="mt-1.5 text-xs text-primary hover:text-primary/80 font-medium"
          whileTap={{ scale: 0.97 }}
        >
          {expanded ? "Show less" : "Show more"}
        </motion.button>
      )}
    </div>
  );
}

function ToolFallbackResult({
  result,
  className,
  ...props
}: React.ComponentProps<"div"> & { result?: unknown }) {
  if (result === undefined) return null;
  const text = typeof result === "string" ? result : JSON.stringify(result, null, 2);
  return (
    <div
      data-slot="tool-fallback-result"
      className={cn(
        "aui-tool-fallback-result border-t border-dashed border-default-200/50 px-4 pt-2",
        className,
      )}
      {...props}
    >
      <div className="mb-1 flex items-center justify-between">
        <p className="aui-tool-fallback-result-header text-xs font-medium text-default-400">Result</p>
        <CopyButton text={text} />
      </div>
      <ResultContent text={text} />
    </div>
  );
}

function ToolFallbackError({
  status,
  className,
  ...props
}: React.ComponentProps<"div"> & { status?: ToolCallStatus }) {
  if (status?.type !== "incomplete") return null;
  const error = status.error;
  const errorText = error
    ? typeof error === "string"
      ? error
      : JSON.stringify(error)
    : null;
  if (!errorText) return null;

  const isCancelled = status.reason === "cancelled";
  const headerText = isCancelled ? "Cancelled reason:" : "Error:";

  return (
    <div
      data-slot="tool-fallback-error"
      className={cn("aui-tool-fallback-error px-4", className)}
      {...props}
    >
      <p className="aui-tool-fallback-error-header font-semibold text-default-500">
        {headerText}
      </p>
      <p className="aui-tool-fallback-error-reason text-default-500">
        {errorText}
      </p>
    </div>
  );
}

// ── Composed ToolFallback ──

interface ToolFallbackProps {
  toolName: string;
  argsText?: string;
  result?: unknown;
  status?: ToolCallStatus;
}

const ToolFallbackImpl: FC<ToolFallbackProps> = ({
  toolName,
  argsText,
  result,
  status,
}) => {
  const isCancelled =
    status?.type === "incomplete" && status.reason === "cancelled";

  return (
    <ToolFallbackRoot
      className={cn(isCancelled && "border-default-200/20 bg-default-50/20")}
    >
      <ToolFallbackTrigger toolName={toolName} argsText={argsText} status={status} />
      <ToolFallbackContent>
        <ToolFallbackError status={status} />
        <ToolFallbackArgs
          argsText={argsText}
          className={cn(isCancelled && "opacity-60")}
        />
        {!isCancelled && <ToolFallbackResult result={result} />}
      </ToolFallbackContent>
    </ToolFallbackRoot>
  );
};

export const ToolFallback = memo(ToolFallbackImpl);
