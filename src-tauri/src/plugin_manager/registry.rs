use crate::error::{NexusError, NexusResult};
use serde::{Deserialize, Serialize};

const DEFAULT_REGISTRY_URL: &str =
    "https://raw.githubusercontent.com/nexus-dashboard/registry/main/registry.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Registry {
    pub version: u32,
    pub updated_at: String,
    pub plugins: Vec<RegistryEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryEntry {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub image: String,
    pub manifest_url: String,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub downloads: u64,
}

pub async fn fetch_registry(registry_url: Option<&str>) -> NexusResult<Registry> {
    let url = registry_url.unwrap_or(DEFAULT_REGISTRY_URL);
    let response = reqwest::get(url)
        .await
        .map_err(NexusError::Http)?;

    if !response.status().is_success() {
        return Err(NexusError::Other(format!(
            "Registry returned status {}",
            response.status()
        )));
    }

    response
        .json::<Registry>()
        .await
        .map_err(NexusError::Http)
}

pub async fn fetch_manifest_from_url(url: &str) -> NexusResult<super::manifest::PluginManifest> {
    let response = reqwest::get(url)
        .await
        .map_err(NexusError::Http)?;

    if !response.status().is_success() {
        return Err(NexusError::Other(format!(
            "Manifest fetch returned status {}",
            response.status()
        )));
    }

    let manifest: super::manifest::PluginManifest =
        response.json().await.map_err(NexusError::Http)?;

    manifest
        .validate()
        .map_err(NexusError::InvalidManifest)?;

    Ok(manifest)
}

pub fn search_registry(registry: &Registry, query: &str) -> Vec<RegistryEntry> {
    let query_lower = query.to_lowercase();
    registry
        .plugins
        .iter()
        .filter(|p| {
            p.name.to_lowercase().contains(&query_lower)
                || p.description.to_lowercase().contains(&query_lower)
                || p.categories
                    .iter()
                    .any(|c| c.to_lowercase().contains(&query_lower))
        })
        .cloned()
        .collect()
}
