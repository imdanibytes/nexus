use crate::error::{NexusError, NexusResult};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const DEFAULT_REGISTRY_URL: &str =
    "https://raw.githubusercontent.com/imdanibytes/registry/main/index.json";

// ---------------------------------------------------------------------------
// Registry source — where plugins come from
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RegistryKind {
    Remote,
    Local,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum RegistryTrust {
    #[default]
    Official,
    Community,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrySource {
    pub id: String,
    pub name: String,
    pub kind: RegistryKind,
    /// URL for Remote registries, filesystem path for Local registries
    pub url: String,
    pub enabled: bool,
    #[serde(default)]
    pub trust: RegistryTrust,
}

// ---------------------------------------------------------------------------
// Registry store — persists configured registries to disk
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
pub struct RegistryStore {
    sources: Vec<RegistrySource>,
    #[serde(skip)]
    path: PathBuf,
}

impl Default for RegistryStore {
    fn default() -> Self {
        RegistryStore {
            sources: vec![RegistrySource {
                id: "nexus-community".to_string(),
                name: "Nexus Community".to_string(),
                kind: RegistryKind::Remote,
                url: DEFAULT_REGISTRY_URL.to_string(),
                enabled: true,
                trust: RegistryTrust::Official,
            }],
            path: PathBuf::new(),
        }
    }
}

impl RegistryStore {
    pub fn load(data_dir: &Path) -> NexusResult<Self> {
        let path = data_dir.join("registries.json");
        if path.exists() {
            let data = std::fs::read_to_string(&path)?;
            let mut store: RegistryStore = serde_json::from_str(&data)?;
            store.path = path;
            Ok(store)
        } else {
            let store = RegistryStore { path, ..Default::default() };
            store.save()?;
            Ok(store)
        }
    }

    pub fn save(&self) -> NexusResult<()> {
        let data = serde_json::to_string_pretty(self)?;
        std::fs::write(&self.path, data)?;
        Ok(())
    }

    pub fn list(&self) -> &[RegistrySource] {
        &self.sources
    }

    pub fn add(&mut self, source: RegistrySource) -> NexusResult<()> {
        if self.sources.iter().any(|s| s.id == source.id) {
            return Err(NexusError::Other(format!(
                "Registry '{}' already exists",
                source.id
            )));
        }
        self.sources.push(source);
        self.save()
    }

    pub fn remove(&mut self, id: &str) -> NexusResult<()> {
        let before = self.sources.len();
        self.sources.retain(|s| s.id != id);
        if self.sources.len() == before {
            return Err(NexusError::Other(format!("Registry '{}' not found", id)));
        }
        self.save()
    }

    pub fn toggle(&mut self, id: &str, enabled: bool) -> NexusResult<()> {
        let source = self
            .sources
            .iter_mut()
            .find(|s| s.id == id)
            .ok_or_else(|| NexusError::Other(format!("Registry '{}' not found", id)))?;
        source.enabled = enabled;
        self.save()
    }

    pub fn enabled_sources(&self) -> Vec<&RegistrySource> {
        self.sources.iter().filter(|s| s.enabled).collect()
    }

    /// Look up the trust level for a registry by its name.
    /// Returns `Community` if the source is not found.
    pub fn source_trust(&self, source_name: &str) -> RegistryTrust {
        self.sources
            .iter()
            .find(|s| s.name == source_name)
            .map(|s| s.trust.clone())
            .unwrap_or(RegistryTrust::Community)
    }
}

// ---------------------------------------------------------------------------
// Registry data types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RegistryMeta {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub homepage: Option<String>,
    #[serde(default)]
    pub maintainer: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Registry {
    pub version: u32,
    pub updated_at: String,
    pub plugins: Vec<RegistryEntry>,
    /// Host extensions available in this registry (optional — old registries don't have this).
    #[serde(default)]
    pub extensions: Vec<ExtensionRegistryEntry>,
    /// Registry-level metadata (v2+).
    #[serde(default)]
    pub registry: Option<RegistryMeta>,
}

/// A host extension listed in a registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionRegistryEntry {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub manifest_url: String,
    #[serde(default)]
    pub manifest_sha256: Option<String>,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub source: String,
    /// Author public key (base64-encoded Ed25519), used for key consistency checks
    #[serde(default)]
    pub author_public_key: Option<String>,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub author_url: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub platforms: Vec<String>,
    #[serde(default)]
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryEntry {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub image: String,
    /// SHA-256 digest of the Docker image (e.g. "sha256:a1b2c3...").
    /// Displayed in the marketplace and used for integrity verification.
    #[serde(default)]
    pub image_digest: Option<String>,
    pub manifest_url: String,
    #[serde(default)]
    pub manifest_sha256: Option<String>,
    #[serde(default)]
    pub categories: Vec<String>,
    /// Which registry this entry came from (populated at fetch time)
    #[serde(default)]
    pub source: String,
    /// Trust level of the registry this entry came from
    #[serde(default)]
    pub source_trust: Option<String>,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub author_url: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub license: Option<String>,
    #[serde(default)]
    pub homepage: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
}

// ---------------------------------------------------------------------------
// Fetching
// ---------------------------------------------------------------------------

/// Maximum response body size for registry/manifest fetches (10 MB).
const MAX_FETCH_BYTES: usize = 10 * 1024 * 1024;

/// Build a hardened HTTP client for registry operations.
fn http_client() -> NexusResult<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
        .map_err(|e| NexusError::Other(format!("HTTP client error: {}", e)))
}

/// Fetch a response body as text with a size limit.
async fn fetch_text(response: reqwest::Response) -> NexusResult<String> {
    if let Some(len) = response.content_length() {
        if len > MAX_FETCH_BYTES as u64 {
            return Err(NexusError::Other("Response too large".to_string()));
        }
    }

    let bytes = response.bytes().await.map_err(NexusError::Http)?;
    if bytes.len() > MAX_FETCH_BYTES {
        return Err(NexusError::Other("Response too large".to_string()));
    }

    String::from_utf8(bytes.to_vec())
        .map_err(|_| NexusError::Other("Response is not valid UTF-8".to_string()))
}

/// Fetch a registry from any source type.
pub async fn fetch_from_source(source: &RegistrySource) -> NexusResult<Registry> {
    match source.kind {
        RegistryKind::Remote => fetch_remote(&source.url).await,
        RegistryKind::Local => fetch_local(&source.url),
    }
}

async fn fetch_remote(url: &str) -> NexusResult<Registry> {
    let client = http_client()?;
    let response = client.get(url).send().await.map_err(NexusError::Http)?;

    if !response.status().is_success() {
        return Err(NexusError::Other(format!(
            "Registry returned status {}",
            response.status()
        )));
    }

    let text = fetch_text(response).await?;
    serde_json::from_str(&text).map_err(|e| NexusError::Other(format!("Invalid registry JSON: {}", e)))
}

fn fetch_local(path_str: &str) -> NexusResult<Registry> {
    let dir = PathBuf::from(path_str);

    // Try index.json first (v2 format), then legacy registry.json
    let index_file = dir.join("index.json");
    let legacy_file = dir.join("registry.json");

    let mut registry = if index_file.exists() {
        let data = std::fs::read_to_string(&index_file)?;
        serde_json::from_str::<Registry>(&data)?
    } else if legacy_file.exists() {
        let data = std::fs::read_to_string(&legacy_file)?;
        serde_json::from_str::<Registry>(&data)?
    } else {
        // Scan YAML files as a fallback
        scan_yaml_registry(&dir)?
    };

    // Resolve relative manifest_url paths to absolute file paths
    for entry in &mut registry.plugins {
        if !entry.manifest_url.starts_with("http://") && !entry.manifest_url.starts_with("https://") {
            let resolved = dir.join(&entry.manifest_url);
            entry.manifest_url = format!("file://{}", resolved.display());
        }
    }

    for entry in &mut registry.extensions {
        if !entry.manifest_url.starts_with("http://") && !entry.manifest_url.starts_with("https://") {
            let resolved = dir.join(&entry.manifest_url);
            entry.manifest_url = format!("file://{}", resolved.display());
        }
    }

    Ok(registry)
}

/// Scan a local directory for YAML-based registry entries.
/// Expects: registry.yaml (metadata), plugins/*.yaml, extensions/*.yaml
fn scan_yaml_registry(dir: &Path) -> NexusResult<Registry> {
    let meta_file = dir.join("registry.yaml");
    let registry_meta = if meta_file.exists() {
        let data = std::fs::read_to_string(&meta_file)?;
        let meta: RegistryMeta = serde_yaml::from_str(&data)
            .map_err(|e| NexusError::Other(format!("Invalid registry.yaml: {}", e)))?;
        Some(meta)
    } else {
        None
    };

    let mut plugins = Vec::new();
    let plugins_dir = dir.join("plugins");
    if plugins_dir.is_dir() {
        for entry in std::fs::read_dir(&plugins_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("yaml")
                || path.extension().and_then(|e| e.to_str()) == Some("yml")
            {
                let data = std::fs::read_to_string(&path)?;
                let plugin: RegistryEntry = serde_yaml::from_str(&data)
                    .map_err(|e| NexusError::Other(format!(
                        "Invalid plugin YAML {}: {}", path.display(), e
                    )))?;
                plugins.push(plugin);
            }
        }
    }

    let mut extensions = Vec::new();
    let extensions_dir = dir.join("extensions");
    if extensions_dir.is_dir() {
        for entry in std::fs::read_dir(&extensions_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("yaml")
                || path.extension().and_then(|e| e.to_str()) == Some("yml")
            {
                let data = std::fs::read_to_string(&path)?;
                let ext: ExtensionRegistryEntry = serde_yaml::from_str(&data)
                    .map_err(|e| NexusError::Other(format!(
                        "Invalid extension YAML {}: {}", path.display(), e
                    )))?;
                extensions.push(ext);
            }
        }
    }

    Ok(Registry {
        version: 2,
        updated_at: chrono::Utc::now().to_rfc3339(),
        plugins,
        extensions,
        registry: registry_meta,
    })
}

/// Result of fetching all registries — both plugin and extension entries.
pub struct FetchAllResult {
    pub plugins: Vec<RegistryEntry>,
    pub extensions: Vec<ExtensionRegistryEntry>,
}

/// Fetch all enabled registries and return merged plugin + extension entries.
pub async fn fetch_all(store: &RegistryStore) -> FetchAllResult {
    let mut all_plugins = Vec::new();
    let mut all_extensions = Vec::new();

    for source in store.enabled_sources() {
        match fetch_from_source(source).await {
            Ok(registry) => {
                for mut entry in registry.plugins {
                    entry.source = source.name.clone();
                    all_plugins.push(entry);
                }
                for mut entry in registry.extensions {
                    entry.source = source.name.clone();
                    all_extensions.push(entry);
                }
            }
            Err(e) => {
                log::warn!("Failed to fetch registry '{}': {}", source.name, e);
            }
        }
    }

    FetchAllResult {
        plugins: all_plugins,
        extensions: all_extensions,
    }
}

/// Fetch a manifest from a URL or file:// path.
///
/// `file://` is only accepted for local registry sources. Remote manifests
/// must use `http://` or `https://`.
pub async fn fetch_manifest(url: &str) -> NexusResult<super::manifest::PluginManifest> {
    if let Some(file_path) = url.strip_prefix("file://") {
        let data = std::fs::read_to_string(file_path)?;
        let manifest: super::manifest::PluginManifest = serde_json::from_str(&data)?;
        manifest.validate().map_err(NexusError::InvalidManifest)?;
        Ok(manifest)
    } else if url.starts_with("http://") || url.starts_with("https://") {
        let client = http_client()?;
        let response = client.get(url).send().await.map_err(NexusError::Http)?;

        if !response.status().is_success() {
            return Err(NexusError::Other(format!(
                "Manifest fetch returned status {}",
                response.status()
            )));
        }

        let text = fetch_text(response).await?;
        let manifest: super::manifest::PluginManifest = serde_json::from_str(&text)
            .map_err(|e| NexusError::Other(format!("Invalid manifest JSON: {}", e)))?;

        manifest.validate().map_err(NexusError::InvalidManifest)?;
        Ok(manifest)
    } else {
        Err(NexusError::Other(format!(
            "Unsupported URL scheme: {}",
            url.split(':').next().unwrap_or("unknown")
        )))
    }
}

/// Fetch an extension manifest from a URL.
pub async fn fetch_extension_manifest(url: &str) -> NexusResult<crate::extensions::manifest::ExtensionManifest> {
    if url.starts_with("http://") || url.starts_with("https://") {
        let client = http_client()?;
        let response = client.get(url).send().await.map_err(NexusError::Http)?;

        if !response.status().is_success() {
            return Err(NexusError::Other(format!(
                "Extension manifest fetch returned status {}",
                response.status()
            )));
        }

        let text = fetch_text(response).await?;
        let manifest: crate::extensions::manifest::ExtensionManifest = serde_json::from_str(&text)
            .map_err(|e| NexusError::Other(format!("Invalid extension manifest JSON: {}", e)))?;

        manifest
            .validate()
            .map_err(NexusError::InvalidManifest)?;
        Ok(manifest)
    } else {
        Err(NexusError::Other(format!(
            "Unsupported URL scheme for extension manifest: {}",
            url.split(':').next().unwrap_or("unknown")
        )))
    }
}

pub fn search_entries(entries: &[RegistryEntry], query: &str) -> Vec<RegistryEntry> {
    if query.is_empty() {
        return entries.to_vec();
    }

    let query_lower = query.to_lowercase();
    entries
        .iter()
        .filter(|p| {
            p.name.to_lowercase().contains(&query_lower)
                || p.description.to_lowercase().contains(&query_lower)
                || p.categories
                    .iter()
                    .any(|c| c.to_lowercase().contains(&query_lower))
                || p.source.to_lowercase().contains(&query_lower)
                || p.author.as_deref().unwrap_or("").to_lowercase().contains(&query_lower)
        })
        .cloned()
        .collect()
}

pub fn search_extension_entries(entries: &[ExtensionRegistryEntry], query: &str) -> Vec<ExtensionRegistryEntry> {
    if query.is_empty() {
        return entries.to_vec();
    }

    let query_lower = query.to_lowercase();
    entries
        .iter()
        .filter(|e| {
            e.name.to_lowercase().contains(&query_lower)
                || e.description.to_lowercase().contains(&query_lower)
                || e.categories
                    .iter()
                    .any(|c| c.to_lowercase().contains(&query_lower))
                || e.source.to_lowercase().contains(&query_lower)
                || e.author.as_deref().unwrap_or("").to_lowercase().contains(&query_lower)
        })
        .cloned()
        .collect()
}
