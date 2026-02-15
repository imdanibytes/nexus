import { useEffect, useRef, useState } from "react";
import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";

interface Props {
  pluginId: string;
  getLogs: (pluginId: string, tail?: number) => Promise<string[]>;
  onClose: () => void;
}

export function PluginLogs({ pluginId, getLogs, onClose }: Props) {
  const [logs, setLogs] = useState<string[]>([]);
  const [loading, setLoading] = useState(true);
  const scrollRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    let active = true;

    async function fetchLogs() {
      setLoading(true);
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
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [logs]);

  return (
    <Sheet open onOpenChange={(open) => { if (!open) onClose(); }}>
      <SheetContent side="bottom" className="h-96 max-w-4xl mx-auto rounded-t-[var(--radius-modal)] p-0">
        <SheetHeader className="px-4 py-2.5 border-b border-nx-border-subtle">
          <SheetTitle className="text-[12px] font-semibold text-nx-text-secondary">
            Logs &mdash; <span className="font-mono text-nx-text-muted">{pluginId}</span>
          </SheetTitle>
        </SheetHeader>
        <div ref={scrollRef} className="flex-1 overflow-y-auto p-4 font-mono text-[12px]">
          {loading && logs.length === 0 ? (
            <p className="text-nx-text-ghost">Loading logs...</p>
          ) : logs.length === 0 ? (
            <p className="text-nx-text-ghost">No logs available</p>
          ) : (
            logs.map((line, i) => (
              <div key={i} className="text-nx-text-secondary whitespace-pre-wrap leading-5">
                {line}
              </div>
            ))
          )}
        </div>
      </SheetContent>
    </Sheet>
  );
}
