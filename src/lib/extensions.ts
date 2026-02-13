import { invoke } from "@tauri-apps/api/core";
import type {
  ExtensionManifest,
  ExtensionRegistryEntry,
  ExtensionStatus,
  InstalledExtension,
} from "../types/extension";

/** List all extensions (running + installed-but-disabled). */
export async function extensionList(): Promise<ExtensionStatus[]> {
  return invoke("extension_list");
}

/** Install an extension from a manifest URL. */
export async function extensionInstall(
  manifestUrl: string
): Promise<InstalledExtension> {
  return invoke("extension_install", { manifestUrl });
}

/** Enable an installed extension (spawns process). */
export async function extensionEnable(extId: string): Promise<void> {
  return invoke("extension_enable", { extId });
}

/** Disable an extension (stops process). */
export async function extensionDisable(extId: string): Promise<void> {
  return invoke("extension_disable", { extId });
}

/** Remove an extension (stop, delete files, revoke permissions). */
export async function extensionRemove(extId: string): Promise<void> {
  return invoke("extension_remove", { extId });
}

/** Install an extension from a local manifest (development). Binary resolved from manifest. */
export async function extensionInstallLocal(
  manifestPath: string
): Promise<InstalledExtension> {
  return invoke("extension_install_local", { manifestPath });
}

/** Preview an extension manifest without installing. */
export async function extensionPreview(
  manifestUrl: string
): Promise<ExtensionManifest> {
  return invoke("extension_preview", { manifestUrl });
}

/** Search the extension marketplace. */
export async function extensionMarketplaceSearch(
  query: string
): Promise<ExtensionRegistryEntry[]> {
  return invoke("extension_marketplace_search", { query });
}
