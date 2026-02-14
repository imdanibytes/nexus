export interface ClassifiedTool {
  name: string;
  description: string;
  input_schema: Record<string, unknown>;
  permissions: string[];
  requires_approval: boolean;
  high_risk: boolean;
}

export interface PluginMetadata {
  id: string;
  name: string;
  description: string;
  author: string;
}
