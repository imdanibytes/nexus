import type { InstalledPlugin } from "../../types/plugin";
import { PluginControls } from "./PluginControls";

interface Props {
  plugin: InstalledPlugin;
  onStart: () => void;
  onStop: () => void;
  onRemove: () => void;
  onShowLogs: () => void;
}

export function PluginViewport({
  plugin,
  onStart,
  onStop,
  onRemove,
  onShowLogs,
}: Props) {
  const isRunning = plugin.status === "running";
  const iframeSrc = `http://localhost:${plugin.assigned_port}${plugin.manifest.ui.path}`;

  return (
    <div className="flex flex-col h-full">
      {/* Plugin header */}
      <div className="flex items-center justify-between px-5 py-3 bg-slate-800/30 border-b border-slate-700">
        <div>
          <h3 className="text-sm font-semibold text-white">
            {plugin.manifest.name}
          </h3>
          <p className="text-xs text-slate-400">
            {plugin.manifest.author} &middot; v{plugin.manifest.version}
          </p>
        </div>
        <PluginControls
          status={plugin.status}
          onStart={onStart}
          onStop={onStop}
          onRemove={onRemove}
          onShowLogs={onShowLogs}
        />
      </div>

      {/* Plugin content */}
      <div className="flex-1 relative">
        {isRunning ? (
          <iframe
            src={iframeSrc}
            className="w-full h-full border-0"
            title={plugin.manifest.name}
            sandbox="allow-scripts allow-same-origin allow-forms allow-popups"
            allow="clipboard-read; clipboard-write"
          />
        ) : (
          <div className="flex flex-col items-center justify-center h-full text-center">
            <div className="w-16 h-16 rounded-2xl bg-slate-800 flex items-center justify-center mb-4">
              <svg
                className="w-8 h-8 text-slate-500"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={1.5}
                  d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
                />
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={1.5}
                  d="M9 9.563C9 9.252 9.252 9 9.563 9h4.874c.311 0 .563.252.563.563v4.874c0 .311-.252.563-.563.563H9.564A.562.562 0 019 14.437V9.564z"
                />
              </svg>
            </div>
            <p className="text-slate-400 text-sm mb-3">
              {plugin.status === "error"
                ? "Plugin encountered an error"
                : "Plugin is stopped"}
            </p>
            <button
              onClick={onStart}
              className="px-4 py-2 bg-indigo-500 hover:bg-indigo-600 text-white text-sm rounded-lg transition-colors"
            >
              Start Plugin
            </button>
          </div>
        )}
      </div>
    </div>
  );
}
