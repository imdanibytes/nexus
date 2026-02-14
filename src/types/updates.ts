export type UpdateSecurity =
  | "verified"
  | "key_match"
  | "key_changed"
  | "digest_available"
  | "no_digest"
  | "untrusted_source"
  | "manifest_domain_changed";

export type UpdateItemType = "plugin" | "extension";

export interface AvailableUpdate {
  item_id: string;
  item_type: UpdateItemType;
  item_name: string;
  installed_version: string;
  available_version: string;
  manifest_url: string;
  registry_source: string;
  security: UpdateSecurity[];
  new_image_digest: string | null;
  author_public_key: string | null;
}
