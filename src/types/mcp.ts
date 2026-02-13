export interface McpToolDef {
  name: string;
  description: string;
  permissions: string[];
  input_schema: Record<string, unknown>;
}

export interface McpConfig {
  tools: McpToolDef[];
}

export interface McpPluginSettings {
  enabled: boolean;
  disabled_tools: string[];
}

export interface McpSettings {
  enabled: boolean;
  plugins: Record<string, McpPluginSettings>;
}

export interface McpToolStatus {
  name: string;
  description: string;
  input_schema: Record<string, unknown>;
  plugin_id: string;
  plugin_name: string;
  plugin_running: boolean;
  mcp_global_enabled: boolean;
  mcp_plugin_enabled: boolean;
  tool_enabled: boolean;
  required_permissions: string[];
  permissions_granted: boolean;
}
