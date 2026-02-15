import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Copy, Check, Loader2, X } from "lucide-react";
import {
  Sheet,
  SheetClose,
  SheetContent,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";

interface Props {
  pluginId: string;
  getLogs: (pluginId: string, tail?: number) => Promise<string[]>;
  onClose: () => void;
}

export function PluginLogs({ pluginId, getLogs, onClose }: Props) {
  const { t } = useTranslation("plugins");
  const [logs, setLogs] = useState<string[]>([]);
  const [loading, setLoading] = useState(true);
  const [copied, setCopied] = useState(false);
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    let active = true;

    async function fetchLogs() {
      const lines = await getLogs(pluginId, 200);
      if (active) {
        setLogs(lines);
        setLoading(false);
      }
    }

    fetchLogs();
    const interval = setInterval(fetchLogs, 3000);

    return () => {
      active = false;
      clearInterval(interval);
    };
  }, [pluginId, getLogs]);

  // Auto-scroll to bottom when new logs arrive
  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [logs]);

  function handleCopy() {
    navigator.clipboard.writeText(logs.join("\n")).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    });
  }

  return (
    <Sheet open onOpenChange={(open) => { if (!open) onClose(); }}>
      <SheetContent side="bottom" showCloseButton={false} className="h-96 max-w-4xl mx-auto rounded-t-[var(--radius-modal)] p-0 flex flex-col">
        <SheetHeader className="px-4 py-2.5 border-b border-nx-border-subtle flex-row items-center gap-2 shrink-0">
          <SheetTitle className="text-[12px] font-semibold text-nx-text-secondary flex items-center gap-2 flex-1">
            {t("logs.title")}
            <Badge variant="secondary" className="text-[10px] font-mono">
              {pluginId}
            </Badge>
          </SheetTitle>
          <Button
            variant="ghost"
            size="xs"
            onClick={handleCopy}
            disabled={logs.length === 0}
            className="text-nx-text-muted"
          >
            {copied ? (
              <Check size={12} strokeWidth={1.5} className="text-nx-success" />
            ) : (
              <Copy size={12} strokeWidth={1.5} />
            )}
            {copied ? t("common:action.copied") : t("common:action.copy")}
          </Button>
          <SheetClose asChild>
            <Button variant="ghost" size="xs" className="text-nx-text-muted">
              <X size={14} strokeWidth={1.5} />
            </Button>
          </SheetClose>
        </SheetHeader>

        <ScrollArea className="flex-1 min-h-0">
          <div className="p-4 font-mono text-[11px] leading-5">
            {loading && logs.length === 0 ? (
              <div className="flex items-center gap-2 text-nx-text-ghost">
                <Loader2 size={14} strokeWidth={1.5} className="animate-spin" />
                {t("logs.loadingLogs")}
              </div>
            ) : logs.length === 0 ? (
              <p className="text-nx-text-ghost">{t("logs.noLogs")}</p>
            ) : (
              logs.map((line, i) => (
                <div key={i} className="text-nx-text-secondary whitespace-pre-wrap hover:bg-nx-surface/40 px-1 -mx-1 rounded-sm">
                  {line}
                </div>
              ))
            )}
            <div ref={bottomRef} />
          </div>
        </ScrollArea>
      </SheetContent>
    </Sheet>
  );
}
