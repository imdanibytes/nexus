import { useEffect, useState } from "react";

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
        className="absolute inset-0 bg-black/50"
        onClick={onClose}
      />
      <div className="relative w-full max-w-4xl h-96 bg-slate-900 border border-slate-700 rounded-t-xl flex flex-col">
        <div className="flex items-center justify-between px-4 py-2 border-b border-slate-700">
          <h3 className="text-sm font-semibold text-slate-300">
            Logs &mdash; {pluginId}
          </h3>
          <button
            onClick={onClose}
            className="text-slate-400 hover:text-white transition-colors"
          >
            <svg
              className="w-4 h-4"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M6 18L18 6M6 6l12 12"
              />
            </svg>
          </button>
        </div>
        <div className="flex-1 overflow-y-auto p-4 font-mono text-xs">
          {loading && logs.length === 0 ? (
            <p className="text-slate-500">Loading logs...</p>
          ) : logs.length === 0 ? (
            <p className="text-slate-500">No logs available</p>
          ) : (
            logs.map((line, i) => (
              <div key={i} className="text-slate-300 whitespace-pre-wrap leading-5">
                {line}
              </div>
            ))
          )}
        </div>
      </div>
    </div>
  );
}
