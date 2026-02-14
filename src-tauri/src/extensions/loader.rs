use std::path::{Path, PathBuf};

use super::manifest::ExtensionManifest;
use super::process::ProcessExtension;
use super::registry::ExtensionRegistry;
use super::signing::{self, KeyConsistency, TrustedKeyStore};
use super::storage::{ExtensionStorage, InstalledExtension};
use super::ExtensionError;

/// Fetch binary data from a URL. Supports `file://` (absolute or relative to manifest)
/// and `http(s)://` URLs.
async fn fetch_binary(
    binary_url: &str,
    manifest_url: Option<&str>,
    ext_id: &str,
) -> Result<Vec<u8>, ExtensionError> {
    if let Some(file_path) = binary_url.strip_prefix("file://") {
        let source = if Path::new(file_path).is_absolute() {
            PathBuf::from(file_path)
        } else if let Some(url) = manifest_url {
            if let Some(manifest_file) = url.strip_prefix("file://") {
                Path::new(manifest_file)
                    .parent()
                    .unwrap_or(Path::new("."))
                    .join(file_path)
            } else {
                PathBuf::from(file_path)
            }
        } else {
            PathBuf::from(file_path)
        };

        log::info!(
            "Copying extension binary for '{}' from {}",
            ext_id,
            source.display()
        );

        if !source.exists() {
            return Err(ExtensionError::Other(format!(
                "Binary not found: {}",
                source.display()
            )));
        }

        Ok(std::fs::read(&source)?)
    } else {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .redirect(reqwest::redirect::Policy::limited(5))
            .build()
            .map_err(|e| ExtensionError::Other(format!("HTTP client error: {}", e)))?;

        log::info!(
            "Downloading extension binary for '{}' from {}",
            ext_id,
            binary_url
        );

        let response = client
            .get(binary_url)
            .send()
            .await
            .map_err(|e| ExtensionError::Other(format!("Download failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(ExtensionError::Other(format!(
                "Binary download returned status {}",
                response.status()
            )));
        }

        let data = response
            .bytes()
            .await
            .map_err(|e| ExtensionError::Other(format!("Failed to read binary: {}", e)))?;
        Ok(data.to_vec())
    }
}

/// Manages the lifecycle of host extensions: install, enable, disable, remove.
pub struct ExtensionLoader {
    /// Root directory for extension packages: ~/.nexus/extensions/
    extensions_dir: PathBuf,
    /// Persistence for installed extension metadata
    pub storage: ExtensionStorage,
    /// Trusted author public keys
    pub trusted_keys: TrustedKeyStore,
}

impl ExtensionLoader {
    pub fn new(data_dir: &Path) -> Self {
        let extensions_dir = data_dir.join("extensions");
        std::fs::create_dir_all(&extensions_dir).ok();

        let storage = ExtensionStorage::load(data_dir);
        let trusted_keys = TrustedKeyStore::load(data_dir);

        Self {
            extensions_dir,
            storage,
            trusted_keys,
        }
    }

    /// Load all enabled extensions into the registry (called at startup).
    pub fn load_enabled(&self, registry: &mut ExtensionRegistry) {
        for installed in self.storage.list() {
            if !installed.enabled {
                continue;
            }

            let binary_path = self
                .extensions_dir
                .join(&installed.manifest.id)
                .join(&installed.binary_name);

            if !binary_path.exists() {
                log::warn!(
                    "Extension '{}' binary not found at {}, skipping",
                    installed.manifest.id,
                    binary_path.display()
                );
                continue;
            }

            let ext = ProcessExtension::new(installed.manifest.clone(), binary_path);
            match ext.start() {
                Ok(()) => {
                    log::info!("Loaded extension: {}", installed.manifest.id);
                    registry.register(Box::new(ext));
                }
                Err(e) => {
                    log::error!(
                        "Failed to start extension '{}': {}",
                        installed.manifest.id,
                        e
                    );
                }
            }
        }
    }

    /// Install an extension from a manifest. Downloads and verifies the binary.
    /// `manifest_url` is used to resolve relative `file://` binary paths.
    pub async fn install(
        &mut self,
        manifest: ExtensionManifest,
        manifest_url: Option<&str>,
    ) -> Result<InstalledExtension, ExtensionError> {
        // Validate manifest
        manifest
            .validate()
            .map_err(|e| ExtensionError::Other(format!("Invalid manifest: {}", e)))?;

        // Check for duplicate
        if self.storage.get(&manifest.id).is_some() {
            return Err(ExtensionError::Other(format!(
                "Extension '{}' is already installed",
                manifest.id
            )));
        }

        // Get binary for current platform
        let binary_entry = manifest
            .binary_for_current_platform()
            .ok_or_else(|| {
                ExtensionError::Other(format!(
                    "No binary available for platform '{}'",
                    ExtensionManifest::current_platform()
                ))
            })?;

        let binary_data = fetch_binary(&binary_entry.url, manifest_url, &manifest.id).await?;

        // Skip signature verification for local file:// binaries (dev workflow)
        if binary_entry.url.starts_with("file://") {
            log::info!(
                "Skipping signature verification for local binary '{}'",
                manifest.id
            );
        } else {
            signing::verify_binary(
                &manifest.author_public_key,
                &binary_data,
                &binary_entry.signature,
                &binary_entry.sha256,
            )?;

            log::info!(
                "Signature verified for extension '{}'",
                manifest.id
            );
        }

        // Check author key consistency (TOFU)
        match self
            .trusted_keys
            .check_key_consistency(&manifest.author, &manifest.author_public_key)
        {
            KeyConsistency::NewAuthor => {
                log::info!(
                    "New author '{}' for extension '{}', trusting key (fingerprint: {})",
                    manifest.author,
                    manifest.id,
                    signing::key_fingerprint(&manifest.author_public_key)
                );
                self.trusted_keys
                    .trust(&manifest.author, &manifest.author_public_key)?;
            }
            KeyConsistency::Matches => {
                log::debug!("Author key matches trusted store for '{}'", manifest.author);
            }
            KeyConsistency::Changed => {
                log::warn!(
                    "SECURITY WARNING: Author key changed for '{}' in extension '{}'. \
                     Old fingerprint does not match new key. Possible supply chain attack.",
                    manifest.author,
                    manifest.id
                );
                // Still install but log the warning — the UI should show this to the user
            }
        }

        // Write binary to disk
        let ext_dir = self.extensions_dir.join(&manifest.id);
        std::fs::create_dir_all(&ext_dir)?;

        let binary_name = if cfg!(target_os = "windows") {
            "extension.exe".to_string()
        } else {
            "extension".to_string()
        };

        let binary_path = ext_dir.join(&binary_name);
        std::fs::write(&binary_path, &binary_data)?;

        // Set executable permission on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&binary_path)?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&binary_path, perms)?;
        }

        // Write manifest
        let manifest_path = ext_dir.join("manifest.json");
        let manifest_json = serde_json::to_string_pretty(&manifest)
            .map_err(|e| ExtensionError::Other(format!("Failed to serialize manifest: {}", e)))?;
        std::fs::write(&manifest_path, manifest_json)?;

        // Save to storage
        let installed = InstalledExtension {
            manifest,
            enabled: false, // Not enabled by default — user must explicitly enable
            installed_at: chrono::Utc::now(),
            binary_name,
        };

        self.storage.add(installed.clone())?;

        log::info!("Installed extension: {}", installed.manifest.id);
        Ok(installed)
    }

    /// Enable an extension: spawn its process and register in the registry.
    pub fn enable(
        &mut self,
        ext_id: &str,
        registry: &mut ExtensionRegistry,
    ) -> Result<(), ExtensionError> {
        let installed = self
            .storage
            .get(ext_id)
            .ok_or_else(|| ExtensionError::Other(format!("Extension '{}' not found", ext_id)))?;

        if installed.enabled {
            return Ok(()); // Already enabled
        }

        let binary_path = self
            .extensions_dir
            .join(ext_id)
            .join(&installed.binary_name);

        if !binary_path.exists() {
            return Err(ExtensionError::Other(format!(
                "Binary not found: {}",
                binary_path.display()
            )));
        }

        let ext = ProcessExtension::new(installed.manifest.clone(), binary_path);
        ext.start()?;
        registry.register(Box::new(ext));

        self.storage.set_enabled(ext_id, true)?;
        log::info!("Enabled extension: {}", ext_id);
        Ok(())
    }

    /// Disable an extension: stop its process and unregister from the registry.
    pub fn disable(
        &mut self,
        ext_id: &str,
        registry: &mut ExtensionRegistry,
    ) -> Result<(), ExtensionError> {
        // Unregister from registry (this drops the ProcessExtension, which calls stop via Drop)
        registry.unregister(ext_id);

        self.storage.set_enabled(ext_id, false)?;
        log::info!("Disabled extension: {}", ext_id);
        Ok(())
    }

    /// Remove an extension: stop, delete files, unregister.
    pub fn remove(
        &mut self,
        ext_id: &str,
        registry: &mut ExtensionRegistry,
    ) -> Result<(), ExtensionError> {
        // Unregister first (stops the process)
        registry.unregister(ext_id);

        // Remove from storage
        self.storage.remove(ext_id)?;

        // Delete extension directory
        let ext_dir = self.extensions_dir.join(ext_id);
        if ext_dir.exists() {
            std::fs::remove_dir_all(&ext_dir)?;
        }

        log::info!("Removed extension: {}", ext_id);
        Ok(())
    }

    /// Update an installed extension to a new version.
    /// If `force_key` is true and the author key changed, the key is rotated.
    /// Otherwise, a key change is treated as an error.
    pub async fn update(
        &mut self,
        manifest: ExtensionManifest,
        registry: &mut ExtensionRegistry,
        force_key: bool,
        manifest_url: Option<&str>,
    ) -> Result<InstalledExtension, ExtensionError> {
        manifest
            .validate()
            .map_err(|e| ExtensionError::Other(format!("Invalid manifest: {}", e)))?;

        let installed = self
            .storage
            .get(&manifest.id)
            .ok_or_else(|| ExtensionError::Other(format!("Extension '{}' not found", manifest.id)))?;

        let was_enabled = installed.enabled;
        let ext_id = manifest.id.clone();

        // Key consistency check
        match self
            .trusted_keys
            .check_key_consistency(&manifest.author, &manifest.author_public_key)
        {
            KeyConsistency::NewAuthor => {
                log::info!(
                    "New author '{}' for extension '{}', trusting key",
                    manifest.author,
                    manifest.id
                );
                self.trusted_keys
                    .trust(&manifest.author, &manifest.author_public_key)?;
            }
            KeyConsistency::Matches => {
                log::debug!("Author key matches for '{}'", manifest.author);
            }
            KeyConsistency::Changed => {
                if force_key {
                    self.trusted_keys
                        .rotate_key(&manifest.author, &manifest.author_public_key)?;
                } else {
                    return Err(ExtensionError::Other(format!(
                        "Author key changed for '{}'. This could indicate a supply chain attack. \
                         Use force_key to accept the new key.",
                        manifest.author
                    )));
                }
            }
        }

        // Disable if running
        if was_enabled {
            registry.unregister(&ext_id);
            self.storage.set_enabled(&ext_id, false)?;
        }

        // Get binary for current platform
        let binary_entry = manifest
            .binary_for_current_platform()
            .ok_or_else(|| {
                ExtensionError::Other(format!(
                    "No binary available for platform '{}'",
                    ExtensionManifest::current_platform()
                ))
            })?;

        let binary_data = fetch_binary(&binary_entry.url, manifest_url, &manifest.id).await?;

        if binary_entry.url.starts_with("file://") {
            log::info!(
                "Skipping signature verification for local binary '{}'",
                manifest.id
            );
        } else {
            signing::verify_binary(
                &manifest.author_public_key,
                &binary_data,
                &binary_entry.signature,
                &binary_entry.sha256,
            )?;

            log::info!("Signature verified for updated extension '{}'", manifest.id);
        }

        // Write binary to disk
        let ext_dir = self.extensions_dir.join(&manifest.id);
        std::fs::create_dir_all(&ext_dir)?;

        let binary_name = if cfg!(target_os = "windows") {
            "extension.exe".to_string()
        } else {
            "extension".to_string()
        };

        let binary_path = ext_dir.join(&binary_name);
        std::fs::write(&binary_path, &binary_data)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&binary_path)?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&binary_path, perms)?;
        }

        // Write manifest
        let manifest_path = ext_dir.join("manifest.json");
        let manifest_json = serde_json::to_string_pretty(&manifest)
            .map_err(|e| ExtensionError::Other(format!("Failed to serialize manifest: {}", e)))?;
        std::fs::write(&manifest_path, manifest_json)?;

        // Update storage
        let updated = InstalledExtension {
            manifest,
            enabled: false,
            installed_at: chrono::Utc::now(),
            binary_name,
        };

        if let Some(existing) = self.storage.get_mut(&ext_id) {
            *existing = updated.clone();
        }
        self.storage.save()?;

        // Re-enable if it was enabled
        if was_enabled {
            self.enable(&ext_id, registry)?;
        }

        log::info!("Updated extension: {}", ext_id);
        Ok(self.storage.get(&ext_id).cloned().unwrap_or(updated))
    }

    /// Install from a local manifest file (for development/testing).
    /// Resolves the binary path from the manifest's `binaries` field.
    pub fn install_local(
        &mut self,
        manifest_path: &Path,
    ) -> Result<InstalledExtension, ExtensionError> {
        let manifest_data = std::fs::read_to_string(manifest_path)?;
        let manifest: ExtensionManifest = serde_json::from_str(&manifest_data)
            .map_err(|e| ExtensionError::Other(format!("Invalid manifest JSON: {}", e)))?;

        manifest
            .validate()
            .map_err(|e| ExtensionError::Other(format!("Invalid manifest: {}", e)))?;

        if self.storage.get(&manifest.id).is_some() {
            return Err(ExtensionError::Other(format!(
                "Extension '{}' is already installed",
                manifest.id
            )));
        }

        // Resolve binary from manifest
        let binary_entry = manifest
            .binary_for_current_platform()
            .ok_or_else(|| {
                ExtensionError::Other(format!(
                    "No binary for platform '{}' in manifest",
                    ExtensionManifest::current_platform()
                ))
            })?;

        let binary_source = if let Some(path) = binary_entry.url.strip_prefix("file://") {
            PathBuf::from(path)
        } else {
            // Treat as a path relative to the manifest directory
            let manifest_dir = manifest_path.parent().unwrap_or(Path::new("."));
            manifest_dir.join(&binary_entry.url)
        };

        if !binary_source.exists() {
            return Err(ExtensionError::Other(format!(
                "Binary not found: {}",
                binary_source.display()
            )));
        }

        // Copy binary
        let ext_dir = self.extensions_dir.join(&manifest.id);
        std::fs::create_dir_all(&ext_dir)?;

        let binary_name = if cfg!(target_os = "windows") {
            "extension.exe".to_string()
        } else {
            "extension".to_string()
        };

        let dest_path = ext_dir.join(&binary_name);
        std::fs::copy(&binary_source, &dest_path)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&dest_path)?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&dest_path, perms)?;
        }

        // Copy manifest
        let manifest_dest = ext_dir.join("manifest.json");
        std::fs::copy(manifest_path, &manifest_dest)?;

        let installed = InstalledExtension {
            manifest,
            enabled: false,
            installed_at: chrono::Utc::now(),
            binary_name,
        };

        self.storage.add(installed.clone())?;
        Ok(installed)
    }
}
