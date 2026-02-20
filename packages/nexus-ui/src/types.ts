export type ToolCallStatus =
  | { type: "running" }
  | { type: "complete" }
  | { type: "incomplete"; reason: string; error?: unknown };

export interface TimingSpanMarker {
  label: string;
  timeMs: number;
}

export interface TimingSpan {
  id: string;
  name: string;
  parentId: string | null;
  startMs: number;
  endMs: number;
  durationMs: number;
  metadata?: Record<string, unknown>;
  markers?: TimingSpanMarker[];
}
