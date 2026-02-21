/** CE Subscriptions API filter dialects. */
export type Filter =
  | { exact: Record<string, string> }
  | { prefix: Record<string, string> }
  | { suffix: Record<string, string> }
  | { all: Filter[] }
  | { any: Filter[] }
  | { not: Filter };

/** What happens when a routing rule matches an event. */
export type RouteAction =
  | {
      action: "invoke_plugin_tool";
      plugin_id: string;
      tool_name: string;
      args_template?: Record<string, unknown>;
    }
  | {
      action: "call_extension";
      extension_id: string;
      operation: string;
      args_template?: Record<string, unknown>;
    }
  | { action: "emit_frontend"; channel: string };

/** A routing rule that matches events by CE filters and triggers an action. */
export interface RoutingRule {
  id: string;
  name?: string;
  filters: Filter[];
  action: RouteAction;
  enabled: boolean;
  created_by: string;
}

/** Event log entry returned from the event log query command. */
export interface EventLogEntry {
  id: string;
  source: string;
  type: string;
  time: string;
  subject?: string;
  data: unknown;
}
