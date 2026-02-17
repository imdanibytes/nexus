use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::manifest::ExtensionManifest;
use super::ExtensionError;

/// Record of an installed extension, persisted to extensions.json.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledExtension {
    pub manifest: ExtensionManifest,
    pub enabled: bool,
    pub installed_at: chrono::DateTime<chrono::Utc>,
    /// Platform-specific binary filename (e.g. "extension" or "extension.exe")
    pub binary_name: String,
}

/// Persists the list of installed extensions to disk.
#[derive(Debug, Serialize, Deserialize)]
pub struct ExtensionStorage {
    extensions: Vec<InstalledExtension>,
    #[serde(skip)]
    path: PathBuf,
}

impl Default for ExtensionStorage {
    fn default() -> Self {
        Self {
            extensions: Vec::new(),
            path: PathBuf::new(),
        }
    }
}

impl ExtensionStorage {
    pub fn load(data_dir: &Path) -> Self {
        let path = data_dir.join("extensions.json");
        if path.exists() {
            if let Ok(data) = std::fs::read_to_string(&path) {
                if let Ok(mut store) = serde_json::from_str::<ExtensionStorage>(&data) {
                    store.path = path;
                    return store;
                }
            }
        }
        ExtensionStorage {
            extensions: Vec::new(),
            path,
        }
    }

    pub fn save(&self) -> Result<(), ExtensionError> {
        let data = serde_json::to_string_pretty(self)
            .map_err(|e| ExtensionError::Other(format!("Failed to serialize extension storage: {}", e)))?;
        crate::util::atomic_write(&self.path, data.as_bytes())?;
        Ok(())
    }

    /// Get all installed extensions.
    pub fn list(&self) -> &[InstalledExtension] {
        &self.extensions
    }

    /// Get a specific installed extension by ID.
    pub fn get(&self, ext_id: &str) -> Option<&InstalledExtension> {
        self.extensions.iter().find(|e| e.manifest.id == ext_id)
    }

    /// Get a mutable reference to an installed extension.
    pub fn get_mut(&mut self, ext_id: &str) -> Option<&mut InstalledExtension> {
        self.extensions.iter_mut().find(|e| e.manifest.id == ext_id)
    }

    /// Add a newly installed extension.
    pub fn add(&mut self, ext: InstalledExtension) -> Result<(), ExtensionError> {
        if self.extensions.iter().any(|e| e.manifest.id == ext.manifest.id) {
            return Err(ExtensionError::Other(format!(
                "Extension '{}' already installed",
                ext.manifest.id
            )));
        }
        self.extensions.push(ext);
        self.save()
    }

    /// Remove an extension by ID.
    pub fn remove(&mut self, ext_id: &str) -> Result<Option<InstalledExtension>, ExtensionError> {
        let idx = self.extensions.iter().position(|e| e.manifest.id == ext_id);
        let removed = idx.map(|i| self.extensions.remove(i));
        if removed.is_some() {
            self.save()?;
        }
        Ok(removed)
    }

    /// Update the enabled state of an extension.
    pub fn set_enabled(&mut self, ext_id: &str, enabled: bool) -> Result<(), ExtensionError> {
        let ext = self.extensions.iter_mut()
            .find(|e| e.manifest.id == ext_id)
            .ok_or_else(|| ExtensionError::Other(format!("Extension '{}' not found", ext_id)))?;
        ext.enabled = enabled;
        self.save()
    }
}
