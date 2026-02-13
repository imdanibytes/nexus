pub mod docker;
pub mod health;
pub mod manifest;
pub mod registry;
pub mod storage;

use crate::error::{NexusError, NexusResult};
use crate::permissions::PermissionStore;
use manifest::PluginManifest;
use storage::{
    hash_token, InstalledPlugin, McpSettings, NexusSettings, PluginSettingsStore, PluginStatus,
    PluginStorage,
};

use std::collections::HashMap;
use std::path::PathBuf;

pub struct PluginManager {
    pub storage: PluginStorage,
    pub permissions: PermissionStore,
    pub registry_store: registry::RegistryStore,
    pub registry_cache: Vec<registry::RegistryEntry>,
    pub settings: NexusSettings,
    pub plugin_settings: PluginSettingsStore,
    pub mcp_settings: McpSettings,
    pub gateway_token_hash: String,
    pub data_dir: PathBuf,
}

impl PluginManager {
    pub fn new(data_dir: PathBuf) -> Self {
        let storage = PluginStorage::load(&data_dir).unwrap_or_default();
        let permissions = PermissionStore::load(&data_dir).unwrap_or_default();
        let registry_store = registry::RegistryStore::load(&data_dir).unwrap_or_default();
        let settings = NexusSettings::load(&data_dir).unwrap_or_default();
        let plugin_settings = PluginSettingsStore::load(&data_dir).unwrap_or_default();
        let mcp_settings = McpSettings::load(&data_dir).unwrap_or_default();

        // Generate or load the MCP gateway token
        let token_path = data_dir.join("mcp_gateway_token");
        let raw_token = if token_path.exists() {
            std::fs::read_to_string(&token_path).unwrap_or_else(|_| {
                let t = uuid::Uuid::new_v4().to_string();
                let _ = std::fs::write(&token_path, &t);
                t
            })
        } else {
            let t = uuid::Uuid::new_v4().to_string();
            let _ = std::fs::write(&token_path, &t);
            log::info!("Generated new MCP gateway token");
            t
        };
        let gateway_token_hash = hash_token(raw_token.trim());

        PluginManager {
            storage,
            permissions,
            registry_store,
            registry_cache: Vec::new(),
            settings,
            plugin_settings,
            mcp_settings,
            gateway_token_hash,
            data_dir,
        }
    }

    /// Verify a raw gateway token against the stored hash.
    pub fn verify_gateway_token(&self, raw: &str) -> bool {
        hash_token(raw) == self.gateway_token_hash
    }

    pub async fn install(
        &mut self,
        manifest: PluginManifest,
        approved_permissions: Vec<crate::permissions::Permission>,
    ) -> NexusResult<InstalledPlugin> {
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
        let token_hash = storage::hash_token(&token);

        // Build environment variables
        let mut env_vars: Vec<String> = manifest
            .env
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();
        env_vars.push(format!("NEXUS_TOKEN={}", token));
        // Browser-accessible URL — the iframe JS runs in the host browser, not inside the container
        env_vars.push(format!("NEXUS_API_URL=http://localhost:9600"));
        // Container-internal URL — for server-side code (MCP handlers etc.) that runs inside Docker
        env_vars.push(format!("NEXUS_HOST_URL=http://host.docker.internal:9600"));

        // Labels for tracking
        let mut labels = HashMap::new();
        labels.insert("nexus.plugin.id".to_string(), manifest.id.clone());
        labels.insert("nexus.plugin.version".to_string(), manifest.version.clone());

        let container_name = format!("nexus-{}", manifest.id.replace('.', "-"));

        let limits = docker::ResourceLimits {
            nano_cpus: self
                .settings
                .cpu_quota_percent
                .map(|pct| (pct / 100.0 * 1e9) as i64),
            memory_bytes: self
                .settings
                .memory_limit_mb
                .map(|mb| (mb * 1_048_576) as i64),
        };

        let container_id = docker::create_container(
            &container_name,
            &manifest.image,
            port,
            manifest.ui.port,
            env_vars,
            labels,
            limits,
        )
        .await?;

        let plugin = InstalledPlugin {
            manifest,
            container_id: Some(container_id),
            status: PluginStatus::Stopped,
            assigned_port: port,
            auth_token: token_hash,
            installed_at: chrono::Utc::now(),
        };

        // Grant only user-approved permissions.
        // Filesystem permissions default to an empty approved_paths list so that
        // every path access triggers a runtime approval prompt. Existing plugins
        // with `None` (unrestricted) are unaffected — this only applies at install time.
        for perm in &approved_permissions {
            let approved_paths = match perm {
                crate::permissions::Permission::FilesystemRead
                | crate::permissions::Permission::FilesystemWrite => Some(vec![]),
                _ => None,
            };
            let _ = self
                .permissions
                .grant(&plugin.manifest.id, perm.clone(), approved_paths);
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

        let port = plugin.assigned_port;
        let ready_path = plugin
            .manifest
            .health
            .as_ref()
            .map(|h| h.endpoint.clone())
            .unwrap_or_else(|| plugin.manifest.ui.path.clone());

        docker::start_container(&container_id).await?;

        // Wait for the plugin's HTTP server to be reachable before reporting success
        docker::wait_for_ready(port, &ready_path, std::time::Duration::from_secs(15)).await?;

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
