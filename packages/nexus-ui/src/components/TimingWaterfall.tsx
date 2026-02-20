import { useMemo, useState } from "react";
import type { TimingSpan } from "../types";
import { cn } from "../lib/utils";

interface TreeNode {
  span: TimingSpan;
  children: TreeNode[];
  depth: number;
}

function buildTree(spans: TimingSpan[]): TreeNode[] {
  const byId = new Map<string, TreeNode>();
  const roots: TreeNode[] = [];

  for (const span of spans) {
    byId.set(span.id, { span, children: [], depth: 0 });
  }

  for (const span of spans) {
    const node = byId.get(span.id)!;
    if (span.parentId && byId.has(span.parentId)) {
      const parent = byId.get(span.parentId)!;
      node.depth = parent.depth + 1;
      parent.children.push(node);
    } else {
      roots.push(node);
    }
  }

  function sortChildren(node: TreeNode) {
    node.children.sort((a, b) => a.span.startMs - b.span.startMs);
    node.children.forEach(sortChildren);
  }
  roots.sort((a, b) => a.span.startMs - b.span.startMs);
  roots.forEach(sortChildren);

  return roots;
}

function flatten(nodes: TreeNode[]): TreeNode[] {
  const result: TreeNode[] = [];
  function walk(node: TreeNode) {
    result.push(node);
    node.children.forEach(walk);
  }
  nodes.forEach(walk);
  return result;
}

function getBarColor(name: string): string {
  if (name === "turn") return "bg-transparent";
  if (name.startsWith("round:")) return "bg-transparent";
  if (name === "setup") return "bg-slate-400/60 dark:bg-slate-500/50";
  if (name === "fetch_settings") return "bg-slate-400 dark:bg-slate-500";
  if (name === "build_tool_executor") return "bg-slate-400/80 dark:bg-slate-500/70";
  if (name === "fetch_mcp_tools") return "bg-amber-400 dark:bg-amber-500";
  if (name === "system_message") return "bg-violet-400/60 dark:bg-violet-500/50";
  if (name.startsWith("provider:")) return "bg-violet-400 dark:bg-violet-500";
  if (name === "llm_call") return "bg-blue-400 dark:bg-blue-500";
  if (name === "tool_execution") return "bg-transparent";
  if (name.startsWith("tool:")) return "bg-emerald-400 dark:bg-emerald-500";
  return "bg-default-400/40";
}

function getBarBorder(name: string): string {
  if (name === "turn") return "border border-default-200/50 rounded-sm";
  if (name.startsWith("round:")) return "border border-default-200/20 rounded-sm";
  if (name === "tool_execution") return "border border-dashed border-emerald-400/40 dark:border-emerald-500/30 rounded-sm";
  return "";
}

function formatDuration(ms: number): string {
  if (ms < 1) return `${(ms * 1000).toFixed(0)}us`;
  if (ms < 1000) return `${ms.toFixed(1)}ms`;
  return `${(ms / 1000).toFixed(2)}s`;
}

function formatName(name: string): string {
  if (name.startsWith("provider:")) return name.slice(9);
  if (name.startsWith("tool:")) return name.slice(5);
  if (name.startsWith("round:")) return `Round ${name.slice(6)}`;
  return name.replace(/_/g, " ");
}

function getGridlines(totalMs: number): number[] {
  if (totalMs <= 0) return [];
  const intervals = [10, 25, 50, 100, 250, 500, 1000, 2500, 5000, 10000];
  const targetCount = 6;
  const ideal = totalMs / targetCount;
  const interval = intervals.find((i) => i >= ideal) ?? intervals[intervals.length - 1];
  const lines: number[] = [];
  for (let t = interval; t < totalMs; t += interval) {
    lines.push(t);
  }
  return lines;
}

export function TimingWaterfall({ spans }: { spans: TimingSpan[] }) {
  const [selectedId, setSelectedId] = useState<string | null>(null);

  const { rows, totalMs, gridlines } = useMemo(() => {
    const tree = buildTree(spans);
    const flat = flatten(tree);
    const turnSpan = flat.find((n) => n.span.name === "turn");
    const total = turnSpan ? turnSpan.span.endMs : Math.max(...spans.map((s) => s.endMs), 1);
    const display = flat.filter((n) => n.span.name !== "turn");
    return {
      rows: display,
      totalMs: total,
      gridlines: getGridlines(total),
    };
  }, [spans]);

  if (rows.length === 0) return null;

  return (
    <div className="flex flex-col text-xs font-mono">
      {/* Header */}
      <div className="flex items-center border-b border-default-200/50 pb-1 mb-1 text-default-500">
        <div className="w-40 shrink-0 px-1 font-semibold">Span</div>
        <div className="flex-1 relative h-4">
          <span className="absolute left-0">0ms</span>
          {gridlines.map((t) => (
            <span
              key={t}
              className="absolute -translate-x-1/2"
              style={{ left: `${(t / totalMs) * 100}%` }}
            >
              {formatDuration(t)}
            </span>
          ))}
          <span className="absolute right-0">{formatDuration(totalMs)}</span>
        </div>
        <div className="w-20 shrink-0 px-1 text-right font-semibold">Duration</div>
      </div>

      {/* Rows */}
      <div className="relative">
        {rows.map((node) => {
          const { span } = node;
          const leftPct = (span.startMs / totalMs) * 100;
          const widthPct = Math.max((span.durationMs / totalMs) * 100, 0.3);
          const isContainer = span.name === "turn" || span.name.startsWith("round:") || span.name === "tool_execution" || span.name === "setup";
          const isSelected = selectedId === span.id;

          return (
            <div
              key={span.id}
              className={cn(
                "flex items-center h-6 group cursor-pointer",
                isSelected ? "bg-primary/10" : "hover:bg-default-100/20",
              )}
              onClick={() => setSelectedId(isSelected ? null : span.id)}
            >
              <div
                className="w-40 shrink-0 px-1 truncate"
                style={{ paddingLeft: `${node.depth * 12 + 4}px` }}
                title={span.name}
              >
                <span
                  className={cn(
                    "inline-block w-2 h-2 rounded-full mr-1.5 shrink-0",
                    getBarColor(span.name).replace("bg-transparent", "bg-default-200/50"),
                  )}
                />
                <span className={cn(isContainer && "text-default-500")}>
                  {formatName(span.name)}
                </span>
              </div>

              <div className="flex-1 relative h-4">
                {gridlines.map((t) => (
                  <div
                    key={t}
                    className="absolute top-0 bottom-0 w-px bg-default-200/20"
                    style={{ left: `${(t / totalMs) * 100}%` }}
                  />
                ))}

                <div
                  className={cn(
                    "absolute top-0.5 bottom-0.5 rounded-[2px] min-w-[2px]",
                    getBarColor(span.name),
                    getBarBorder(span.name),
                  )}
                  style={{
                    left: `${leftPct}%`,
                    width: `${widthPct}%`,
                  }}
                />

                {span.markers?.map((marker, i) => {
                  const markerPct = (marker.timeMs / totalMs) * 100;
                  return (
                    <div
                      key={`${span.id}-m-${i}`}
                      className="absolute top-0 bottom-0 flex items-center group/marker z-10"
                      style={{ left: `${markerPct}%` }}
                    >
                      <div className="w-2 h-2 rotate-45 bg-rose-500 dark:bg-rose-400 border border-rose-700 dark:border-rose-300 -translate-x-1 cursor-pointer" />
                      <div className="absolute bottom-full left-1/2 -translate-x-1/2 mb-1 hidden group-hover/marker:block pointer-events-none">
                        <div className="whitespace-nowrap rounded bg-default-50/80 backdrop-blur-xl border border-default-200/50 px-1.5 py-0.5 text-[10px] text-foreground shadow-md">
                          {marker.label} <span className="text-default-500">{formatDuration(marker.timeMs)}</span>
                        </div>
                      </div>
                    </div>
                  );
                })}
              </div>

              <div className="w-20 shrink-0 px-1 text-right tabular-nums text-default-500">
                {formatDuration(span.durationMs)}
              </div>
            </div>
          );
        })}
      </div>

      {/* Legend */}
      <div className="flex gap-3 mt-3 pt-2 border-t border-default-200/50 text-default-500 flex-wrap">
        {[
          { label: "Setup", color: "bg-slate-400 dark:bg-slate-500" },
          { label: "Network", color: "bg-amber-400 dark:bg-amber-500" },
          { label: "System msg", color: "bg-violet-400 dark:bg-violet-500" },
          { label: "LLM", color: "bg-blue-400 dark:bg-blue-500" },
          { label: "Tool", color: "bg-emerald-400 dark:bg-emerald-500" },
        ].map(({ label, color }) => (
          <div key={label} className="flex items-center gap-1">
            <div className={cn("w-2.5 h-2.5 rounded-[2px]", color)} />
            <span>{label}</span>
          </div>
        ))}
        <div className="flex items-center gap-1">
          <div className="w-2 h-2 rotate-45 bg-rose-500 dark:bg-rose-400" />
          <span>Marker</span>
        </div>
      </div>

      {/* Selected span detail */}
      {selectedId && (() => {
        const span = spans.find((s) => s.id === selectedId);
        if (!span) return null;
        const hasMetadata = span.metadata && Object.keys(span.metadata).length > 0;
        const hasMarkers = span.markers && span.markers.length > 0;
        if (!hasMetadata && !hasMarkers) return (
          <div className="mt-2 border-t border-default-200/50 pt-2 text-default-400 text-center">
            No metadata for {formatName(span.name)}
          </div>
        );
        return (
          <div className="mt-2 border-t border-default-200/50 pt-2 text-default-500 space-y-1">
            <div className="flex items-center justify-between mb-1">
              <span className="font-semibold text-foreground">{formatName(span.name)}</span>
              <span className="tabular-nums">{formatDuration(span.durationMs)}</span>
            </div>
            {hasMetadata && (
              <div className="flex flex-wrap gap-x-3 gap-y-0.5">
                {Object.entries(span.metadata!).map(([k, v]) => (
                  <span key={k}>
                    <span className="text-default-400">{k}:</span>{" "}
                    <span className="text-foreground">{String(v)}</span>
                  </span>
                ))}
              </div>
            )}
            {hasMarkers && (
              <div className="flex flex-wrap gap-x-3 gap-y-0.5 pt-1">
                {span.markers!.map((m, i) => (
                  <span key={i} className="inline-flex items-center gap-1">
                    <span className="w-1.5 h-1.5 rotate-45 bg-rose-500 dark:bg-rose-400 inline-block" />
                    <span className="text-foreground">{m.label}</span>
                    <span className="tabular-nums">{formatDuration(m.timeMs)}</span>
                  </span>
                ))}
              </div>
            )}
          </div>
        );
      })()}
    </div>
  );
}
