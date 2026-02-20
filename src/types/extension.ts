export interface ExtensionOperation {
  name: string;
  description: string;
  risk_level: "low" | "medium" | "high";
  scope_key: string | null;
  scope_description: string | null;
}

export interface ExtensionConsumer {
  plugin_id: string;
  plugin_name: string;
  granted: boolean;
}

/** Declared capability of a host extension (shown at install time). */
export type Capability =
  | { type: "process_exec"; scope: string[] }
  | { type: "file_read"; scope: string[] }
  | { type: "file_write"; scope: string[] }
  | { type: "network_http"; scope: string[] }
  | { type: "system_info" }
  | { type: "native_library"; scope: string[] }
  | { type: "custom"; name: string; description: string };

export interface ResourceListView {
  columns: string[];
  sort_by?: string;
  sort_order?: string;
}

export interface ResourceCapabilities {
  create: boolean;
  update: boolean;
  delete: boolean;
}

export interface ResourceTypeDef {
  label: string;
  description?: string;
  icon?: string;
  schema: Record<string, unknown>;
  list_view?: ResourceListView;
  capabilities?: ResourceCapabilities;
}

export interface ExtensionStatus {
  id: string;
  display_name: string;
  description: string;
  operations: ExtensionOperation[];
  capabilities: Capability[];
  consumers: ExtensionConsumer[];
  resources: Record<string, ResourceTypeDef>;
  installed: boolean;
  enabled: boolean;
}

/** Per-platform binary entry in an extension manifest. */
export interface BinaryEntry {
  url: string;
  signature: string;
  sha256: string;
}

/** Full extension manifest (returned by preview). */
export interface ExtensionManifest {
  id: string;
  display_name: string;
  version: string;
  description: string;
  author: string;
  license: string | null;
  homepage: string | null;
  operations: ExtensionOperation[];
  capabilities: Capability[];
  author_public_key: string;
  binaries: Record<string, BinaryEntry>;
}

/** Installed extension record. */
export interface InstalledExtension {
  manifest: ExtensionManifest;
  enabled: boolean;
  installed_at: string;
  binary_name: string;
}

/** Extension entry from the marketplace registry. */
export interface ExtensionRegistryEntry {
  id: string;
  name: string;
  version: string;
  description: string;
  author: string;
  author_url?: string;
  created_at?: string;
  author_public_key?: string;
  manifest_url: string;
  platforms?: string[];
  categories: string[];
  status?: string;
  source?: string;
}
