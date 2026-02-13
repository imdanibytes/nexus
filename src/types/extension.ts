export interface ExtensionOperation {
  name: string;
  description: string;
  risk_level: "low" | "medium" | "high";
}

export interface ExtensionConsumer {
  plugin_id: string;
  plugin_name: string;
  granted: boolean;
}

export interface ExtensionStatus {
  id: string;
  display_name: string;
  description: string;
  operations: ExtensionOperation[];
  consumers: ExtensionConsumer[];
}
