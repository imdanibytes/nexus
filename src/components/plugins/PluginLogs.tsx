import { useCallback, useEffect, useRef, useState } from "react";
import { X } from "lucide-react";

interface Props {
  pluginId: string;
  getLogs: (pluginId: string, tail?: number) => Promise<string[]>;
  onClose: () => void;
}

export function PluginLogs({ pluginId, getLogs, onClose }: Props) {
  const [logs, setLogs] = useState<string[]>([]);
  const [loading, setLoading] = useState(true);
  const [visible, setVisible] = useState(false);
  const scrollRef = useRef<HTMLDivElement>(null);

  // Trigger enter animation on mount
  useEffect(() => {
    requestAnimationFrame(() => setVisible(true));
  }, []);

  const handleClose = useCallback(() => {
    setVisible(false);
    setTimeout(onClose, 200);
  }, [onClose]);

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
    <div className="fixed inset-0 z-40 flex items-end justify-center">
      {/* Backdrop */}
      <div
        className="absolute inset-0 bg-black/50 backdrop-blur-sm transition-opacity duration-200"
        style={{ opacity: visible ? 1 : 0 }}
        onClick={handleClose}
      />
      {/* Panel */}
      <div
        className="relative w-full max-w-4xl h-96 bg-nx-deep border border-nx-border rounded-t-[var(--radius-modal)] flex flex-col transition-transform duration-200 ease-out"
        style={{ transform: visible ? "translateY(0)" : "translateY(100%)" }}
      >
        <div className="flex items-center justify-between px-4 py-2.5 border-b border-nx-border-subtle">
          <h3 className="text-[12px] font-semibold text-nx-text-secondary">
            Logs &mdash; <span className="font-mono text-nx-text-muted">{pluginId}</span>
          </h3>
          <button
            onClick={handleClose}
            className="text-nx-text-muted hover:text-nx-text transition-colors duration-150"
          >
            <X size={14} strokeWidth={1.5} />
          </button>
        </div>
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
      </div>
    </div>
  );
}
