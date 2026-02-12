import { useEffect, useState } from "react";
import { X } from "lucide-react";

interface Props {
  pluginId: string;
  getLogs: (pluginId: string, tail?: number) => Promise<string[]>;
  onClose: () => void;
}

export function PluginLogs({ pluginId, getLogs, onClose }: Props) {
  const [logs, setLogs] = useState<string[]>([]);
  const [loading, setLoading] = useState(true);

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

  return (
    <div className="fixed inset-0 z-40 flex items-end justify-center">
      <div
        className="absolute inset-0 bg-black/50 backdrop-blur-sm"
        onClick={onClose}
      />
      <div className="relative w-full max-w-4xl h-96 bg-nx-deep border border-nx-border rounded-t-[var(--radius-modal)] flex flex-col">
        <div className="flex items-center justify-between px-4 py-2.5 border-b border-nx-border-subtle">
          <h3 className="text-[12px] font-semibold text-nx-text-secondary">
            Logs &mdash; <span className="font-mono text-nx-text-muted">{pluginId}</span>
          </h3>
          <button
            onClick={onClose}
            className="text-nx-text-muted hover:text-nx-text transition-colors duration-150"
          >
            <X size={14} strokeWidth={1.5} />
          </button>
        </div>
        <div className="flex-1 overflow-y-auto p-4 font-mono text-[12px]">
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
