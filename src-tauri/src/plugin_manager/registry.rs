use crate::error::{NexusError, NexusResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
            store.migrate_defaults();
            Ok(store)
        } else {
            let store = RegistryStore { path, ..Default::default() };
            store.save()?;
            Ok(store)
        }
    }

    /// Migrate the built-in "nexus-community" source if its URL is stale.
    /// This handles the case where DEFAULT_REGISTRY_URL changed between releases
    /// but the user's persisted registries.json still has the old value.
    fn migrate_defaults(&mut self) {
        let mut changed = false;
        if let Some(source) = self.sources.iter_mut().find(|s| s.id == "nexus-community") {
            if source.url != DEFAULT_REGISTRY_URL {
                log::info!(
                    "Migrating nexus-community registry URL: {} -> {}",
                    source.url,
                    DEFAULT_REGISTRY_URL
                );
                source.url = DEFAULT_REGISTRY_URL.to_string();
                changed = true;
            }
        }
        if changed {
            if let Err(e) = self.save() {
                log::warn!("Failed to save migrated registry store: {}", e);
            }
        }
    }

    pub fn save(&self) -> NexusResult<()> {
        let data = serde_json::to_string_pretty(self)?;
        crate::util::atomic_write(&self.path, data.as_bytes())?;
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
    /// Absolute path to a directory containing a Dockerfile.
    /// Set automatically for local registry entries that declare `build_context`.
    /// When present, Nexus can build the image from source instead of pulling.
    #[serde(default)]
    pub build_context: Option<String>,
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

    // Resolve relative paths to absolute file paths
    for entry in &mut registry.plugins {
        if !entry.manifest_url.starts_with("http://") && !entry.manifest_url.starts_with("https://") {
            let resolved = dir.join(&entry.manifest_url);
            entry.manifest_url = format!("file://{}", resolved.display());
        }
        // Resolve build_context to absolute path
        if let Some(ref ctx) = entry.build_context {
            if !ctx.starts_with('/') {
                let resolved = dir.join(ctx);
                entry.build_context = Some(resolved.display().to_string());
            }
        } else {
            // Auto-detect: if a Dockerfile sits next to the manifest, set build_context
            let manifest_path = if let Some(fp) = entry.manifest_url.strip_prefix("file://") {
                std::path::PathBuf::from(fp)
            } else {
                dir.join(&entry.manifest_url)
            };
            if let Some(manifest_dir) = manifest_path.parent() {
                if manifest_dir.join("Dockerfile").exists() {
                    entry.build_context = Some(manifest_dir.display().to_string());
                }
            }
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
#[derive(Serialize, Deserialize)]
pub struct FetchAllResult {
    pub plugins: Vec<RegistryEntry>,
    pub extensions: Vec<ExtensionRegistryEntry>,
}

// ---------------------------------------------------------------------------
// Local registry cache (Homebrew-style disk persistence + conditional GET)
// ---------------------------------------------------------------------------

/// Persisted registry cache on disk. Avoids network round-trips when the
/// registry hasn't changed (uses HTTP conditional GET with ETags per RFC 9111).
#[derive(Serialize, Deserialize, Default)]
pub struct RegistryCache {
    pub plugins: Vec<RegistryEntry>,
    pub extensions: Vec<ExtensionRegistryEntry>,
    /// ISO-8601 timestamp of last successful remote fetch.
    pub last_refreshed: String,
    /// Per-source ETags for conditional GET (source_id → etag).
    #[serde(default)]
    pub etags: HashMap<String, String>,
}

const CACHE_FILE: &str = "registry-cache.json";

/// Load the persisted registry cache from disk. Returns `None` if the file
/// doesn't exist or can't be parsed.
pub fn load_cache(data_dir: &Path) -> Option<RegistryCache> {
    let path = data_dir.join(CACHE_FILE);
    let data = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&data).ok()
}

/// Atomically persist the registry cache to disk.
pub fn save_cache(data_dir: &Path, cache: &RegistryCache) -> NexusResult<()> {
    let path = data_dir.join(CACHE_FILE);
    let data = serde_json::to_string_pretty(cache)?;
    crate::util::atomic_write(&path, data.as_bytes())?;
    Ok(())
}

/// Outcome of a conditional fetch against a single remote source.
pub enum FetchOutcome {
    /// 200 OK — new data + new ETag (if provided by server).
    Fresh(Registry, Option<String>),
    /// 304 Not Modified — cached data is still current.
    NotModified,
}

/// Fetch a remote registry with conditional GET (If-None-Match).
async fn fetch_remote_conditional(url: &str, etag: Option<&str>) -> NexusResult<FetchOutcome> {
    let client = http_client()?;
    let mut request = client.get(url);
    if let Some(etag_val) = etag {
        request = request.header("If-None-Match", etag_val);
    }

    let response = request.send().await.map_err(NexusError::Http)?;

    if response.status() == reqwest::StatusCode::NOT_MODIFIED {
        return Ok(FetchOutcome::NotModified);
    }

    if !response.status().is_success() {
        return Err(NexusError::Other(format!(
            "Registry returned status {}",
            response.status()
        )));
    }

    let new_etag = response
        .headers()
        .get("etag")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let text = fetch_text(response).await?;
    let registry: Registry = serde_json::from_str(&text)
        .map_err(|e| NexusError::Other(format!("Invalid registry JSON: {}", e)))?;

    Ok(FetchOutcome::Fresh(registry, new_etag))
}

/// Fetch all enabled registries using conditional GET for remote sources.
/// Reuses cached entries for sources that returned 304 Not Modified.
pub async fn fetch_all_conditional(
    store: &RegistryStore,
    existing_cache: &RegistryCache,
) -> (FetchAllResult, HashMap<String, String>) {
    let mut all_plugins = Vec::new();
    let mut all_extensions = Vec::new();
    let mut new_etags = existing_cache.etags.clone();

    for source in store.enabled_sources() {
        match source.kind {
            RegistryKind::Local => {
                // Local sources always read from disk — no caching needed.
                match fetch_local(&source.url) {
                    Ok(registry) => {
                        let trust_str = format!("{:?}", source.trust).to_lowercase();
                        for mut entry in registry.plugins {
                            entry.source = source.name.clone();
                            entry.source_trust = Some(trust_str.clone());
                            all_plugins.push(entry);
                        }
                        for mut entry in registry.extensions {
                            entry.source = source.name.clone();
                            all_extensions.push(entry);
                        }
                    }
                    Err(e) => {
                        log::warn!("Failed to fetch local registry '{}': {}", source.name, e);
                    }
                }
            }
            RegistryKind::Remote => {
                let cached_etag = existing_cache.etags.get(&source.id).map(|s| s.as_str());
                match fetch_remote_conditional(&source.url, cached_etag).await {
                    Ok(FetchOutcome::NotModified) => {
                        log::info!("Registry '{}': 304 Not Modified (cached)", source.name);
                        // Reuse entries from disk cache for this source.
                        for entry in &existing_cache.plugins {
                            if entry.source == source.name {
                                all_plugins.push(entry.clone());
                            }
                        }
                        for entry in &existing_cache.extensions {
                            if entry.source == source.name {
                                all_extensions.push(entry.clone());
                            }
                        }
                    }
                    Ok(FetchOutcome::Fresh(registry, new_etag)) => {
                        log::info!("Registry '{}': 200 OK (fresh data)", source.name);
                        if let Some(etag) = new_etag {
                            new_etags.insert(source.id.clone(), etag);
                        }
                        let trust_str = format!("{:?}", source.trust).to_lowercase();
                        for mut entry in registry.plugins {
                            entry.source = source.name.clone();
                            entry.source_trust = Some(trust_str.clone());
                            all_plugins.push(entry);
                        }
                        for mut entry in registry.extensions {
                            entry.source = source.name.clone();
                            all_extensions.push(entry);
                        }
                    }
                    Err(e) => {
                        log::warn!("Failed to fetch registry '{}': {}", source.name, e);
                        // Fall back to cached entries for this source.
                        for entry in &existing_cache.plugins {
                            if entry.source == source.name {
                                all_plugins.push(entry.clone());
                            }
                        }
                        for entry in &existing_cache.extensions {
                            if entry.source == source.name {
                                all_extensions.push(entry.clone());
                            }
                        }
                    }
                }
            }
        }
    }

    (
        FetchAllResult {
            plugins: all_plugins,
            extensions: all_extensions,
        },
        new_etags,
    )
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
    if let Some(file_path) = url.strip_prefix("file://") {
        let data = std::fs::read_to_string(file_path)?;
        let manifest: crate::extensions::manifest::ExtensionManifest = serde_json::from_str(&data)
            .map_err(|e| NexusError::Other(format!("Invalid extension manifest JSON: {}", e)))?;
        manifest.validate().map_err(NexusError::InvalidManifest)?;
        Ok(manifest)
    } else if url.starts_with("http://") || url.starts_with("https://") {
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
