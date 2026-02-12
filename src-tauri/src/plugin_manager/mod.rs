pub mod docker;
pub mod health;
pub mod manifest;
pub mod registry;
pub mod storage;

use crate::error::{NexusError, NexusResult};
use crate::permissions::PermissionStore;
use manifest::PluginManifest;
use storage::{InstalledPlugin, PluginStatus, PluginStorage};

use std::collections::HashMap;
use std::path::PathBuf;

pub struct PluginManager {
    pub storage: PluginStorage,
    pub permissions: PermissionStore,
    pub registry_store: registry::RegistryStore,
    pub registry_cache: Vec<registry::RegistryEntry>,
    data_dir: PathBuf,
}

impl PluginManager {
    pub fn new(data_dir: PathBuf) -> Self {
        let storage = PluginStorage::load(&data_dir).unwrap_or_default();
        let permissions = PermissionStore::load(&data_dir).unwrap_or_default();
        let registry_store = registry::RegistryStore::load(&data_dir).unwrap_or_default();

        PluginManager {
            storage,
            permissions,
            registry_store,
            registry_cache: Vec::new(),
            data_dir,
        }
    }

    pub async fn install(&mut self, manifest: PluginManifest) -> NexusResult<InstalledPlugin> {
        manifest
            .validate()
            .map_err(NexusError::InvalidManifest)?;

        if self.storage.get(&manifest.id).is_some() {
            return Err(NexusError::PluginAlreadyExists(manifest.id.clone()));
        }

        // Pull the Docker image
        log::info!("Pulling image: {}", manifest.image);
        docker::pull_image(&manifest.image).await?;

        let port = self.storage.allocate_port();
        let token = uuid::Uuid::new_v4().to_string();

        // Build environment variables
        let mut env_vars: Vec<String> = manifest
            .env
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();
        env_vars.push(format!("NEXUS_TOKEN={}", token));
        // Browser-accessible URL — the iframe JS runs in the host browser, not inside the container
        env_vars.push(format!("NEXUS_API_URL=http://localhost:9600"));

        // Labels for tracking
        let mut labels = HashMap::new();
        labels.insert("nexus.plugin.id".to_string(), manifest.id.clone());
        labels.insert("nexus.plugin.version".to_string(), manifest.version.clone());

        let container_name = format!("nexus-{}", manifest.id.replace('.', "-"));

        let container_id = docker::create_container(
            &container_name,
            &manifest.image,
            port,
            manifest.ui.port,
            env_vars,
            labels,
        )
        .await?;

        let plugin = InstalledPlugin {
            manifest,
            container_id: Some(container_id),
            status: PluginStatus::Stopped,
            assigned_port: port,
            auth_token: token,
            installed_at: chrono::Utc::now(),
        };

        // Auto-grant permissions declared in the manifest
        for perm in &plugin.manifest.permissions {
            let _ = self.permissions.grant(&plugin.manifest.id, perm.clone(), None);
        }

        self.storage.add(plugin.clone())?;
        Ok(plugin)
    }

    pub async fn start(&mut self, plugin_id: &str) -> NexusResult<()> {
        let plugin = self
            .storage
            .get(plugin_id)
            .ok_or_else(|| NexusError::PluginNotFound(plugin_id.to_string()))?;

        let container_id = plugin
            .container_id
            .clone()
            .ok_or_else(|| NexusError::Other("No container ID".to_string()))?;

        docker::start_container(&container_id).await?;

        if let Some(plugin) = self.storage.get_mut(plugin_id) {
            plugin.status = PluginStatus::Running;
            self.storage.save()?;
        }

        Ok(())
    }

    pub async fn stop(&mut self, plugin_id: &str) -> NexusResult<()> {
        let plugin = self
            .storage
            .get(plugin_id)
            .ok_or_else(|| NexusError::PluginNotFound(plugin_id.to_string()))?;

        let container_id = plugin
            .container_id
            .clone()
            .ok_or_else(|| NexusError::Other("No container ID".to_string()))?;

        docker::stop_container(&container_id).await?;

        if let Some(plugin) = self.storage.get_mut(plugin_id) {
            plugin.status = PluginStatus::Stopped;
            self.storage.save()?;
        }

        Ok(())
    }

    pub async fn remove(&mut self, plugin_id: &str) -> NexusResult<()> {
        let plugin = self
            .storage
            .get(plugin_id)
            .ok_or_else(|| NexusError::PluginNotFound(plugin_id.to_string()))?;

        let image_name = plugin.manifest.image.clone();

        if let Some(container_id) = &plugin.container_id {
            // Stop first if running
            if plugin.status == PluginStatus::Running {
                let _ = docker::stop_container(container_id).await;
            }
            docker::remove_container(container_id).await?;
        }

        // Remove the Docker image (ignore failure — another container may reference it)
        if let Err(e) = docker::remove_image(&image_name).await {
            log::warn!("Could not remove image {}: {}", image_name, e);
        }

        self.storage.remove(plugin_id)?;
        self.permissions.revoke_all(plugin_id)?;

        Ok(())
    }

    pub async fn logs(&self, plugin_id: &str, tail: u32) -> NexusResult<Vec<String>> {
        let plugin = self
            .storage
            .get(plugin_id)
            .ok_or_else(|| NexusError::PluginNotFound(plugin_id.to_string()))?;

        let container_id = plugin
            .container_id
            .as_ref()
            .ok_or_else(|| NexusError::Other("No container ID".to_string()))?;

        docker::get_logs(container_id, tail).await
    }

    pub fn list(&self) -> Vec<&InstalledPlugin> {
        self.storage.list()
    }

    pub async fn refresh_registry(&mut self) -> NexusResult<()> {
        self.registry_cache = registry::fetch_all(&self.registry_store).await;
        Ok(())
    }

    pub fn search_marketplace(&self, query: &str) -> Vec<registry::RegistryEntry> {
        registry::search_entries(&self.registry_cache, query)
    }
}
