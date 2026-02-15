pub mod dev_watcher;
pub mod health;
pub mod manifest;
pub mod registry;
pub mod storage;

use crate::error::{NexusError, NexusResult};
use crate::extensions::ipc::AppIpcRouter;
use crate::extensions::loader::ExtensionLoader;
use crate::extensions::registry::ExtensionRegistry;
use crate::host_api::mcp_client::McpClientManager;
use crate::permissions::service::PermissionService;
use crate::runtime::{ContainerConfig, ContainerRuntime, ResourceLimits, SecurityConfig};
use crate::update_checker::UpdateCheckState;
use crate::AppState;
use manifest::PluginManifest;
use storage::{
    hash_token, InstalledPlugin, McpSettings, NexusSettings, PluginSettingsStore, PluginStatus,
    PluginStorage,
};

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Generate a Docker volume name for a plugin's persistent data.
fn data_volume_name(plugin_id: &str) -> String {
    format!("nexus-data-{}", plugin_id.replace('.', "-"))
}

/// Reject install/update if the plugin requires a newer Nexus version.
fn check_min_nexus_version(manifest: &PluginManifest) -> NexusResult<()> {
    if let Some(ref required) = manifest.min_nexus_version {
        let current = semver::Version::parse(env!("CARGO_PKG_VERSION"))
            .expect("CARGO_PKG_VERSION is valid semver");
        let minimum = semver::Version::parse(required).map_err(|e| {
            NexusError::InvalidManifest(format!("invalid min_nexus_version \"{required}\": {e}"))
        })?;
        if current < minimum {
            return Err(NexusError::Other(format!(
                "Plugin \"{}\" requires Nexus >= {minimum}, but this is Nexus {current}. \
                 Please update Nexus first.",
                manifest.id,
            )));
        }
    }
    Ok(())
}

pub struct PluginManager {
    pub runtime: Arc<dyn ContainerRuntime>,
    pub storage: PluginStorage,
    pub permissions: Arc<dyn PermissionService>,
    pub extensions: ExtensionRegistry,
    pub extension_loader: ExtensionLoader,
    pub registry_store: registry::RegistryStore,
    pub registry_cache: Vec<registry::RegistryEntry>,
    pub extension_registry_cache: Vec<registry::ExtensionRegistryEntry>,
    pub settings: NexusSettings,
    pub plugin_settings: PluginSettingsStore,
    pub mcp_settings: McpSettings,
    pub update_state: UpdateCheckState,
    pub gateway_token_hash: String,
    pub data_dir: PathBuf,
    tool_version: AtomicU64,
    tool_version_tx: tokio::sync::watch::Sender<u64>,
    pub tool_version_rx: tokio::sync::watch::Receiver<u64>,
    /// Native MCP client connections to plugin servers.
    pub mcp_clients: McpClientManager,
}

impl PluginManager {
    pub fn new(
        data_dir: PathBuf,
        runtime: Arc<dyn ContainerRuntime>,
        permissions: Arc<dyn PermissionService>,
    ) -> Self {
        let storage = PluginStorage::load(&data_dir).unwrap_or_default();
        let mut registry_store = registry::RegistryStore::load(&data_dir).unwrap_or_default();
        let settings = NexusSettings::load(&data_dir).unwrap_or_default();
        let plugin_settings = PluginSettingsStore::load(&data_dir).unwrap_or_default();
        let mcp_settings = McpSettings::load(&data_dir).unwrap_or_default();
        let update_state = crate::update_checker::load_update_state(&data_dir);

        // Auto-register local registry for MCP-wrapped plugins
        let mcp_plugins_dir = data_dir.join("mcp-plugins");
        std::fs::create_dir_all(&mcp_plugins_dir).ok();
        if !registry_store.list().iter().any(|s| s.id == "nexus-mcp-local") {
            let _ = registry_store.add(registry::RegistrySource {
                id: "nexus-mcp-local".to_string(),
                name: "MCP Wrapped Plugins".to_string(),
                kind: registry::RegistryKind::Local,
                url: mcp_plugins_dir.display().to_string(),
                enabled: true,
                trust: registry::RegistryTrust::Community,
            });
        }

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

        let (tool_version_tx, tool_version_rx) = tokio::sync::watch::channel(0u64);

        let extension_loader = ExtensionLoader::new(&data_dir);
        let mut extensions = ExtensionRegistry::new();

        // Load all enabled extensions at startup
        extension_loader.load_enabled(&mut extensions);

        PluginManager {
            runtime,
            storage,
            permissions,
            extensions,
            extension_loader,
            registry_store,
            registry_cache: Vec::new(),
            extension_registry_cache: Vec::new(),
            settings,
            plugin_settings,
            mcp_settings,
            update_state,
            gateway_token_hash,
            data_dir,
            tool_version: AtomicU64::new(0),
            tool_version_tx,
            tool_version_rx,
            mcp_clients: McpClientManager::new(),
        }
    }

    /// Build resource limits from current settings.
    fn resource_limits(&self) -> ResourceLimits {
        ResourceLimits {
            nano_cpus: self
                .settings
                .cpu_quota_percent
                .map(|pct| (pct / 100.0 * 1e9) as i64),
            memory_bytes: self
                .settings
                .memory_limit_mb
                .map(|mb| (mb * 1_048_576) as i64),
        }
    }

    /// Verify a raw gateway token against the stored hash.
    pub fn verify_gateway_token(&self, raw: &str) -> bool {
        hash_token(raw) == self.gateway_token_hash
    }

    /// Bump the tool version counter and notify SSE subscribers.
    /// Call this after any change that affects the MCP tool list.
    pub fn notify_tools_changed(&self) {
        let v = self.tool_version.fetch_add(1, Ordering::Relaxed) + 1;
        let _ = self.tool_version_tx.send(v);
        log::debug!("Tool list changed (version {})", v);
    }

    pub async fn install(
        &mut self,
        manifest: PluginManifest,
        approved_permissions: Vec<crate::permissions::Permission>,
        deferred_permissions: Vec<crate::permissions::Permission>,
        manifest_url: Option<&str>,
        local_manifest_path: Option<String>,
    ) -> NexusResult<InstalledPlugin> {
        manifest
            .validate()
            .map_err(NexusError::InvalidManifest)?;

        check_min_nexus_version(&manifest)?;

        // Preserve dev_mode across local-to-local reinstalls only.
        // When switching sources (local→registry or registry→local), reset dev_mode
        // and use the new local_manifest_path as-is (don't carry over the old one).
        let prev_dev_mode = if let Some(existing) = self.storage.get(&manifest.id) {
            let dm = if local_manifest_path.is_some() { existing.dev_mode } else { false };

            log::info!("Reinstalling plugin '{}' (replacing existing)", manifest.id);

            // Stop and remove old container, but keep volume (data) and permissions.
            // Also remove by name as fallback (container name survives Docker restarts).
            if let Some(container_id) = &existing.container_id {
                if existing.status == PluginStatus::Running {
                    let _ = self.runtime.stop_container(container_id).await;
                }
                let _ = self.runtime.remove_container(container_id).await;
            }
            let name = format!("nexus-{}", manifest.id.replace('.', "-"));
            let _ = self.runtime.remove_container(&name).await;

            self.storage.remove(&manifest.id)?;
            dm
        } else {
            false
        };

        // Pull the Docker image (skip if already present — e.g. locally built)
        let image_exists = self.runtime.image_exists(&manifest.image).await.unwrap_or(false);
        if image_exists {
            log::info!("Image already exists locally: {}", manifest.image);
        } else {
            log::info!("Pulling image: {}", manifest.image);
            self.runtime.pull_image(&manifest.image).await?;
        }

        // Verify image digest if declared in manifest
        if let Some(ref expected_digest) = manifest.image_digest {
            match self.runtime.get_image_digest(&manifest.image).await? {
                Some(actual_digest) => {
                    if &actual_digest != expected_digest {
                        return Err(NexusError::Other(format!(
                            "Image digest mismatch for {}. Expected: {}, Got: {}. \
                             The image may have been tampered with.",
                            manifest.image, expected_digest, actual_digest
                        )));
                    }
                    log::info!(
                        "Image digest verified: {} = {}",
                        manifest.image, actual_digest
                    );
                }
                None => {
                    log::warn!(
                        "Image {} has no registry digest (locally built?). \
                         Skipping digest verification.",
                        manifest.image
                    );
                }
            }
        } else {
            log::warn!(
                "Plugin {} has no image_digest — skipping content verification",
                manifest.id
            );
        }

        let port = self.storage.allocate_port();
        let token = uuid::Uuid::new_v4().to_string();
        let token_hash = storage::hash_token(&token);

        // Build environment variables
        let mut env_vars: Vec<String> = manifest
            .env
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();
        env_vars.push(format!("NEXUS_PLUGIN_SECRET={}", token));
        // Browser-accessible URL — the iframe JS runs in the host browser, not inside the container
        env_vars.push("NEXUS_API_URL=http://localhost:9600".to_string());
        // Container-internal URL — for server-side code (MCP handlers etc.) that runs inside Docker
        env_vars.push("NEXUS_HOST_URL=http://host.docker.internal:9600".to_string());
        // Persistent data directory inside the container
        env_vars.push("NEXUS_DATA_DIR=/data".to_string());

        // Labels for tracking
        let mut labels = HashMap::new();
        labels.insert("nexus.plugin.id".to_string(), manifest.id.clone());
        labels.insert("nexus.plugin.version".to_string(), manifest.version.clone());

        let container_name = format!("nexus-{}", manifest.id.replace('.', "-"));
        let volume_name = data_volume_name(&manifest.id);

        let container_port = manifest.ui.as_ref().map(|u| u.port).unwrap_or(80);
        let container_id = self.runtime.create_container(ContainerConfig {
            name: container_name,
            image: manifest.image.clone(),
            host_port: port,
            container_port,
            env_vars,
            labels,
            limits: self.resource_limits(),
            data_volume: Some(volume_name),
            network: "nexus-bridge".to_string(),
            security: SecurityConfig::default(),
        })
        .await?;

        let plugin = InstalledPlugin {
            manifest,
            container_id: Some(container_id),
            status: PluginStatus::Stopped,
            assigned_port: port,
            auth_token: token_hash,
            installed_at: chrono::Utc::now(),
            manifest_url_origin: manifest_url.and_then(storage::extract_url_host),
            dev_mode: prev_dev_mode,
            local_manifest_path,
        };

        // Grant only user-approved permissions.
        // Filesystem permissions default to an empty approved_scopes list so that
        // every path access triggers a runtime approval prompt. Extension permissions
        // with scope_key also default to empty scopes. Existing plugins with `None`
        // (unrestricted) are unaffected — this only applies at install time.
        for perm in &approved_permissions {
            let approved_scopes = match perm {
                crate::permissions::Permission::FilesystemRead
                | crate::permissions::Permission::FilesystemWrite => Some(vec![]),
                crate::permissions::Permission::Extension(_) => Some(vec![]),
                _ => None,
            };
            let _ = self
                .permissions
                .grant(&plugin.manifest.id, perm.clone(), approved_scopes);
        }

        // Deferred permissions: user skipped these at install time.
        // They'll trigger a JIT approval dialog on first use.
        for perm in &deferred_permissions {
            let approved_scopes = match perm {
                crate::permissions::Permission::FilesystemRead
                | crate::permissions::Permission::FilesystemWrite => Some(vec![]),
                crate::permissions::Permission::Extension(_) => Some(vec![]),
                _ => None,
            };
            let _ = self
                .permissions
                .defer(&plugin.manifest.id, perm.clone(), approved_scopes);
        }

        self.storage.add(plugin.clone())?;
        Ok(plugin)
    }

    /// Start a plugin. Recreates the container with a fresh auth token every
    /// time — tokens are ephemeral to the container lifecycle. If a token leaks,
    /// restarting the plugin invalidates it.
    pub async fn start(&mut self, plugin_id: &str) -> NexusResult<()> {
        let plugin = self
            .storage
            .get(plugin_id)
            .ok_or_else(|| NexusError::PluginNotFound(plugin_id.to_string()))?;

        let manifest = plugin.manifest.clone();
        let port = plugin.assigned_port;
        let old_container_id = plugin.container_id.clone();

        let ready_path = manifest
            .health
            .as_ref()
            .map(|h| h.endpoint.clone())
            .unwrap_or_else(|| {
                manifest.ui.as_ref()
                    .map(|u| u.path.clone())
                    .unwrap_or_else(|| "/health".to_string())
            });

        // Remove the old container (if any).
        // After a Docker engine restart, the container ID may be stale but the
        // name is still claimed — so we also force-remove by name as a fallback.
        let container_name = format!("nexus-{}", manifest.id.replace('.', "-"));
        if let Some(ref cid) = old_container_id {
            let _ = self.runtime.stop_container(cid).await;
            let _ = self.runtime.remove_container(cid).await;
        }
        let _ = self.runtime.remove_container(&container_name).await;

        // Fresh token for every start
        let new_token = uuid::Uuid::new_v4().to_string();
        let new_hash = storage::hash_token(&new_token);

        let mut env_vars: Vec<String> = manifest
            .env
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();
        env_vars.push(format!("NEXUS_PLUGIN_SECRET={}", new_token));
        env_vars.push("NEXUS_API_URL=http://localhost:9600".to_string());
        env_vars.push("NEXUS_HOST_URL=http://host.docker.internal:9600".to_string());
        env_vars.push("NEXUS_DATA_DIR=/data".to_string());

        let mut labels = HashMap::new();
        labels.insert("nexus.plugin.id".to_string(), manifest.id.clone());
        labels.insert("nexus.plugin.version".to_string(), manifest.version.clone());

        let container_name = format!("nexus-{}", manifest.id.replace('.', "-"));
        let volume_name = data_volume_name(plugin_id);

        let container_port = manifest.ui.as_ref().map(|u| u.port).unwrap_or(80);
        let new_container_id = self.runtime.create_container(ContainerConfig {
            name: container_name,
            image: manifest.image.clone(),
            host_port: port,
            container_port,
            env_vars,
            labels,
            limits: self.resource_limits(),
            data_volume: Some(volume_name),
            network: "nexus-bridge".to_string(),
            security: SecurityConfig::default(),
        })
        .await?;

        self.runtime.start_container(&new_container_id).await?;
        self.runtime.wait_for_ready(port, &ready_path, std::time::Duration::from_secs(15)).await?;

        if let Some(plugin) = self.storage.get_mut(plugin_id) {
            plugin.auth_token = new_hash;
            plugin.container_id = Some(new_container_id);
            plugin.status = PluginStatus::Running;
        }
        self.storage.save()?;

        // Connect to native MCP server if the plugin declares one
        if let Some(ref mcp_config) = manifest.mcp {
            if let Some(ref server_config) = mcp_config.server {
                match self
                    .mcp_clients
                    .connect(plugin_id, port, &server_config.path)
                    .await
                {
                    Ok(()) => {
                        log::info!(
                            "Connected to native MCP server for plugin '{}'",
                            plugin_id
                        );
                    }
                    Err(e) => {
                        log::warn!(
                            "Failed to connect to native MCP server for plugin '{}': {}. \
                             Plugin is running but native MCP features unavailable.",
                            plugin_id,
                            e
                        );
                    }
                }
            } else if !mcp_config.tools.is_empty() {
                log::warn!(
                    "DEPRECATED: Plugin '{}' uses mcp.tools without mcp.server. \
                     Migrate to a native MCP server for full MCP support.",
                    plugin_id
                );
            }
        }

        log::info!("Started plugin={} with fresh auth token", plugin_id);
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

        // Disconnect native MCP client before stopping the container
        self.mcp_clients.disconnect(plugin_id);

        self.runtime.stop_container(&container_id).await?;

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

        // Disconnect native MCP client
        self.mcp_clients.disconnect(plugin_id);

        if let Some(container_id) = &plugin.container_id {
            // Stop first if running
            if plugin.status == PluginStatus::Running {
                let _ = self.runtime.stop_container(container_id).await;
            }
            self.runtime.remove_container(container_id).await?;
        }

        // Remove the Docker image (ignore failure — another container may reference it)
        if let Err(e) = self.runtime.remove_image(&image_name).await {
            log::warn!("Could not remove image {}: {}", image_name, e);
        }

        // Remove persistent data: Docker volume + KV storage
        let volume_name = data_volume_name(plugin_id);
        if let Err(e) = self.runtime.remove_volume(&volume_name).await {
            log::warn!("Could not remove volume {}: {}", volume_name, e);
        }
        crate::host_api::storage::remove_plugin_storage(&self.data_dir, plugin_id);

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

        Ok(self.runtime.get_logs(container_id, tail).await?)
    }

    pub fn list(&self) -> Vec<&InstalledPlugin> {
        self.storage.list()
    }

    pub async fn refresh_registry(&mut self) -> NexusResult<()> {
        let result = registry::fetch_all(&self.registry_store).await;
        self.registry_cache = result.plugins;
        self.extension_registry_cache = result.extensions;
        Ok(())
    }

    pub fn search_marketplace(&self, query: &str) -> Vec<registry::RegistryEntry> {
        registry::search_entries(&self.registry_cache, query)
    }

    /// Update an installed plugin to a new version from a manifest URL.
    /// Preserves assigned_port, auth_token, and permissions.
    pub async fn update_plugin(
        &mut self,
        manifest: PluginManifest,
        expected_digest: Option<String>,
    ) -> NexusResult<InstalledPlugin> {
        manifest
            .validate()
            .map_err(NexusError::InvalidManifest)?;

        check_min_nexus_version(&manifest)?;

        let plugin = self
            .storage
            .get(&manifest.id)
            .ok_or_else(|| NexusError::PluginNotFound(manifest.id.clone()))?;

        // Security: block digest downgrade
        if plugin.manifest.image_digest.is_some() && manifest.image_digest.is_none() {
            return Err(NexusError::Other(
                "Digest downgrade blocked: installed plugin has an image digest but the update does not".to_string(),
            ));
        }

        // Security: verify expected digest matches manifest
        if let Some(ref expected) = expected_digest {
            if let Some(ref manifest_digest) = manifest.image_digest {
                if expected != manifest_digest {
                    return Err(NexusError::Other(format!(
                        "Expected digest {} does not match manifest digest {}",
                        expected, manifest_digest
                    )));
                }
            }
        }

        let was_running = plugin.status == PluginStatus::Running;
        let port = plugin.assigned_port;
        let old_container_id = plugin.container_id.clone();
        let preserved_origin = plugin.manifest_url_origin.clone();
        let preserved_dev_mode = plugin.dev_mode;
        let preserved_local_path = plugin.local_manifest_path.clone();

        // Stop old container (also remove by name as fallback for Docker restarts)
        if let Some(ref cid) = old_container_id {
            if was_running {
                let _ = self.runtime.stop_container(cid).await;
            }
            let _ = self.runtime.remove_container(cid).await;
        }
        let container_name = format!("nexus-{}", manifest.id.replace('.', "-"));
        let _ = self.runtime.remove_container(&container_name).await;

        // Pull new image
        log::info!("Pulling updated image: {}", manifest.image);
        self.runtime.pull_image(&manifest.image).await?;

        // Verify digest if present
        if let Some(ref expected_digest) = manifest.image_digest {
            match self.runtime.get_image_digest(&manifest.image).await? {
                Some(actual_digest) => {
                    if &actual_digest != expected_digest {
                        return Err(NexusError::Other(format!(
                            "Image digest mismatch for {}. Expected: {}, Got: {}",
                            manifest.image, expected_digest, actual_digest
                        )));
                    }
                    log::info!(
                        "Image digest verified: {} = {}",
                        manifest.image, actual_digest
                    );
                }
                None => {
                    log::warn!(
                        "Image {} has no registry digest, skipping digest verification",
                        manifest.image
                    );
                }
            }
        }

        // Create new container
        let mut env_vars: Vec<String> = manifest
            .env
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();
        let new_token = uuid::Uuid::new_v4().to_string();
        let new_hash = storage::hash_token(&new_token);
        env_vars.push(format!("NEXUS_PLUGIN_SECRET={}", new_token));
        env_vars.push("NEXUS_API_URL=http://localhost:9600".to_string());
        env_vars.push("NEXUS_HOST_URL=http://host.docker.internal:9600".to_string());
        env_vars.push("NEXUS_DATA_DIR=/data".to_string());

        let mut labels = HashMap::new();
        labels.insert("nexus.plugin.id".to_string(), manifest.id.clone());
        labels.insert("nexus.plugin.version".to_string(), manifest.version.clone());

        let container_name = format!("nexus-{}", manifest.id.replace('.', "-"));
        let volume_name = data_volume_name(&manifest.id);

        let container_port = manifest.ui.as_ref().map(|u| u.port).unwrap_or(80);
        let new_container_id = self.runtime.create_container(ContainerConfig {
            name: container_name,
            image: manifest.image.clone(),
            host_port: port,
            container_port,
            env_vars,
            labels,
            limits: self.resource_limits(),
            data_volume: Some(volume_name),
            network: "nexus-bridge".to_string(),
            security: SecurityConfig::default(),
        })
        .await?;

        let updated_plugin = InstalledPlugin {
            manifest,
            container_id: Some(new_container_id.clone()),
            status: PluginStatus::Stopped,
            assigned_port: port,
            auth_token: new_hash,
            installed_at: chrono::Utc::now(),
            manifest_url_origin: preserved_origin,
            dev_mode: preserved_dev_mode,
            local_manifest_path: preserved_local_path,
        };

        // Update storage
        if let Some(existing) = self.storage.get_mut(&updated_plugin.manifest.id) {
            *existing = updated_plugin.clone();
        }

        // Restart if it was running
        if was_running {
            let ready_path = updated_plugin
                .manifest
                .health
                .as_ref()
                .map(|h| h.endpoint.clone())
                .unwrap_or_else(|| {
                    updated_plugin.manifest.ui.as_ref()
                        .map(|u| u.path.clone())
                        .unwrap_or_else(|| "/health".to_string())
                });

            self.runtime.start_container(&new_container_id).await?;
            self.runtime.wait_for_ready(port, &ready_path, std::time::Duration::from_secs(15)).await?;

            if let Some(plugin) = self.storage.get_mut(&updated_plugin.manifest.id) {
                plugin.status = PluginStatus::Running;
            }
        }

        self.storage.save()?;
        log::info!(
            "Updated plugin {} to version {}",
            updated_plugin.manifest.id,
            updated_plugin.manifest.version
        );

        Ok(self.storage.get(&updated_plugin.manifest.id).cloned().unwrap())
    }

    pub fn search_extension_marketplace(&self, query: &str) -> Vec<registry::ExtensionRegistryEntry> {
        registry::search_extension_entries(&self.extension_registry_cache, query)
    }

    /// Enable a host extension (spawns process, registers in runtime).
    /// Uses split borrows to satisfy the borrow checker.
    pub fn enable_extension(&mut self, ext_id: &str) -> Result<(), crate::extensions::ExtensionError> {
        self.extension_loader.enable(ext_id, &mut self.extensions)
    }

    /// Disable a host extension (stops process, unregisters).
    pub fn disable_extension(&mut self, ext_id: &str) -> Result<(), crate::extensions::ExtensionError> {
        self.extension_loader.disable(ext_id, &mut self.extensions)
    }

    /// Update a host extension to a new version.
    pub async fn update_extension(
        &mut self,
        manifest: crate::extensions::manifest::ExtensionManifest,
        force_key: bool,
        manifest_url: Option<&str>,
    ) -> Result<crate::extensions::storage::InstalledExtension, crate::extensions::ExtensionError> {
        self.extension_loader
            .update(manifest, &mut self.extensions, force_key, manifest_url)
            .await
    }

    /// Remove a host extension (stop, delete files, unregister).
    pub fn remove_extension(&mut self, ext_id: &str) -> Result<(), crate::extensions::ExtensionError> {
        self.extension_loader.remove(ext_id, &mut self.extensions)
    }

    /// Install (or reinstall) an extension from a local manifest.
    /// Idempotent: if already installed, hot-swaps the binary in place.
    pub fn install_extension_local(
        &mut self,
        manifest_path: &std::path::Path,
    ) -> Result<crate::extensions::storage::InstalledExtension, crate::extensions::ExtensionError> {
        self.extension_loader.install_local(manifest_path, &mut self.extensions)
    }

    /// Create the IPC router and inject it into all registered extensions.
    /// Must be called after the AppState Arc is constructed (needs the Arc for the router).
    pub fn wire_extension_ipc(state: &AppState) {
        let router = Arc::new(AppIpcRouter::new(state.clone()));
        // Called during setup before the state is shared — no contention, no runtime needed
        let mut mgr = state.try_write().expect("state not yet shared, try_write must succeed");
        mgr.extensions.set_ipc_router(router);
        log::info!("Extension IPC router wired");
    }
}

// ---------------------------------------------------------------------------
// Tests — PluginManager integration tests using MockRuntime
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::mock::{MockRuntime, RuntimeCall};

    fn test_manifest(id: &str) -> manifest::PluginManifest {
        manifest::PluginManifest {
            id: id.into(),
            name: format!("Test Plugin {}", id),
            version: "1.0.0".into(),
            description: "A test plugin".into(),
            author: "Test".into(),
            license: None,
            homepage: None,
            icon: None,
            image: format!("test-{}:latest", id.replace('.', "-")),
            image_digest: None,
            ui: Some(manifest::UiConfig {
                port: 80,
                path: "/".into(),
            }),
            permissions: vec![],
            health: None,
            env: HashMap::new(),
            min_nexus_version: None,
            settings: vec![],
            mcp: None,
            extensions: HashMap::new(),
        }
    }

    fn test_manager(dir: &std::path::Path, mock: Arc<MockRuntime>) -> PluginManager {
        let store = crate::permissions::PermissionStore::load(dir).unwrap_or_default();
        let permissions: Arc<dyn crate::permissions::service::PermissionService> =
            Arc::new(crate::permissions::DefaultPermissionService::new(store));
        PluginManager::new(dir.to_path_buf(), mock, permissions)
    }

    // -- install --

    #[tokio::test]
    async fn install_pulls_image_and_creates_container() {
        let tmp = tempfile::tempdir().unwrap();
        let mock = Arc::new(MockRuntime::new());
        let mock_ref = Arc::clone(&mock);
        let mut mgr = test_manager(tmp.path(), mock);

        let m = test_manifest("com.test.alpha");
        let plugin = mgr
            .install(m.clone(), vec![], vec![], None, None)
            .await
            .unwrap();

        assert_eq!(plugin.manifest.id, "com.test.alpha");
        assert_eq!(plugin.status, PluginStatus::Stopped);
        assert!(plugin.container_id.is_some());

        // Runtime should have: ImageExists → PullImage → CreateContainer
        assert!(mock_ref.was_called(&RuntimeCall::PullImage(
            "test-com-test-alpha:latest".into()
        )));
        assert!(mock_ref.was_called(&RuntimeCall::CreateContainer(
            "nexus-com-test-alpha".into()
        )));
    }

    #[tokio::test]
    async fn install_skips_pull_for_existing_image() {
        let tmp = tempfile::tempdir().unwrap();
        let mock = Arc::new(MockRuntime::new().with_image("test-com-test-cached:latest"));
        let mock_ref = Arc::clone(&mock);
        let mut mgr = test_manager(tmp.path(), mock);

        let m = test_manifest("com.test.cached");
        mgr.install(m, vec![], vec![], None, None).await.unwrap();

        // PullImage should NOT have been called (image already existed)
        assert!(!mock_ref.was_called(&RuntimeCall::PullImage(
            "test-com-test-cached:latest".into()
        )));
        // But CreateContainer should still happen
        assert!(mock_ref.was_called(&RuntimeCall::CreateContainer(
            "nexus-com-test-cached".into()
        )));
    }

    #[tokio::test]
    async fn install_verifies_matching_digest() {
        let tmp = tempfile::tempdir().unwrap();
        let digest = "sha256:a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2";
        let mock = Arc::new(
            MockRuntime::new().with_image_digest("test-com-test-digest:latest", digest),
        );
        let mut mgr = test_manager(tmp.path(), mock);

        let mut m = test_manifest("com.test.digest");
        m.image_digest = Some(digest.into());

        let result = mgr.install(m, vec![], vec![], None, None).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn install_rejects_digest_mismatch() {
        let tmp = tempfile::tempdir().unwrap();
        let mock = Arc::new(MockRuntime::new().with_image_digest(
            "test-com-test-bad:latest",
            "sha256:0000000000000000000000000000000000000000000000000000000000000000",
        ));
        let mut mgr = test_manager(tmp.path(), mock);

        let mut m = test_manifest("com.test.bad");
        m.image_digest = Some(
            "sha256:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff".into(),
        );

        let result = mgr.install(m, vec![], vec![], None, None).await;
        assert!(result.is_err());
        let err = format!("{}", result.unwrap_err());
        assert!(err.contains("digest mismatch"), "error was: {err}");
    }

    #[tokio::test]
    async fn reinstall_removes_old_container() {
        let tmp = tempfile::tempdir().unwrap();
        let mock = Arc::new(MockRuntime::new());
        let mock_ref = Arc::clone(&mock);
        let mut mgr = test_manager(tmp.path(), mock);

        let m = test_manifest("com.test.reinstall");
        let first = mgr
            .install(m.clone(), vec![], vec![], None, None)
            .await
            .unwrap();
        let first_cid = first.container_id.unwrap();

        // Install again — should remove old container
        let second = mgr.install(m, vec![], vec![], None, None).await.unwrap();
        let second_cid = second.container_id.unwrap();

        assert_ne!(first_cid, second_cid);
        assert!(mock_ref.was_called(&RuntimeCall::RemoveContainer(first_cid)));
    }

    // -- stop --

    #[tokio::test]
    async fn stop_calls_runtime_and_updates_status() {
        let tmp = tempfile::tempdir().unwrap();
        let mock = Arc::new(MockRuntime::new());
        let mock_ref = Arc::clone(&mock);
        let mut mgr = test_manager(tmp.path(), mock);

        let m = test_manifest("com.test.stop");
        let plugin = mgr
            .install(m, vec![], vec![], None, None)
            .await
            .unwrap();
        let cid = plugin.container_id.clone().unwrap();

        // Simulate that start happened (set status to Running)
        if let Some(p) = mgr.storage.get_mut("com.test.stop") {
            p.status = PluginStatus::Running;
        }

        mgr.stop("com.test.stop").await.unwrap();

        assert!(mock_ref.was_called(&RuntimeCall::StopContainer(cid)));
        assert_eq!(
            mgr.storage.get("com.test.stop").unwrap().status,
            PluginStatus::Stopped
        );
    }

    // -- remove --

    #[tokio::test]
    async fn remove_cleans_container_image_and_volume() {
        let tmp = tempfile::tempdir().unwrap();
        let mock = Arc::new(MockRuntime::new());
        let mock_ref = Arc::clone(&mock);
        let mut mgr = test_manager(tmp.path(), mock);

        let m = test_manifest("com.test.remove");
        let plugin = mgr
            .install(m, vec![], vec![], None, None)
            .await
            .unwrap();
        let cid = plugin.container_id.clone().unwrap();

        mgr.remove("com.test.remove").await.unwrap();

        assert!(mock_ref.was_called(&RuntimeCall::RemoveContainer(cid)));
        assert!(mock_ref.was_called(&RuntimeCall::RemoveImage(
            "test-com-test-remove:latest".into()
        )));
        assert!(mock_ref.was_called(&RuntimeCall::RemoveVolume(
            "nexus-data-com-test-remove".into()
        )));
        assert!(mgr.storage.get("com.test.remove").is_none());
    }

    #[tokio::test]
    async fn remove_stops_running_container_first() {
        let tmp = tempfile::tempdir().unwrap();
        let mock = Arc::new(MockRuntime::new());
        let mock_ref = Arc::clone(&mock);
        let mut mgr = test_manager(tmp.path(), mock);

        let m = test_manifest("com.test.running");
        let plugin = mgr
            .install(m, vec![], vec![], None, None)
            .await
            .unwrap();
        let cid = plugin.container_id.clone().unwrap();

        // Simulate running state
        if let Some(p) = mgr.storage.get_mut("com.test.running") {
            p.status = PluginStatus::Running;
        }

        mgr.remove("com.test.running").await.unwrap();

        // Should stop before removing
        let calls = mock_ref.calls();
        let stop_idx = calls
            .iter()
            .position(|c| *c == RuntimeCall::StopContainer(cid.clone()))
            .expect("StopContainer should have been called");
        let remove_idx = calls
            .iter()
            .position(|c| *c == RuntimeCall::RemoveContainer(cid.clone()))
            .expect("RemoveContainer should have been called");
        assert!(
            stop_idx < remove_idx,
            "stop should happen before remove"
        );
    }

    #[tokio::test]
    async fn remove_nonexistent_errors() {
        let tmp = tempfile::tempdir().unwrap();
        let mock = Arc::new(MockRuntime::new());
        let mut mgr = test_manager(tmp.path(), mock);

        let result = mgr.remove("com.test.nope").await;
        assert!(result.is_err());
    }

    // -- logs --

    #[tokio::test]
    async fn logs_returns_container_output() {
        let tmp = tempfile::tempdir().unwrap();
        let mock = Arc::new(MockRuntime::new());
        let mut mgr = test_manager(tmp.path(), mock);

        let m = test_manifest("com.test.logs");
        mgr.install(m, vec![], vec![], None, None).await.unwrap();

        let logs = mgr.logs("com.test.logs", 100).await.unwrap();
        assert_eq!(logs.len(), 2);
        assert!(logs[0].contains("mock log"));
    }

    #[tokio::test]
    async fn logs_nonexistent_errors() {
        let tmp = tempfile::tempdir().unwrap();
        let mock = Arc::new(MockRuntime::new());
        let mgr = test_manager(tmp.path(), mock);

        let result = mgr.logs("com.test.ghost", 100).await;
        assert!(result.is_err());
    }

    // -- list --

    #[tokio::test]
    async fn list_returns_installed_plugins() {
        let tmp = tempfile::tempdir().unwrap();
        let mock = Arc::new(MockRuntime::new());
        let mut mgr = test_manager(tmp.path(), mock);

        assert_eq!(mgr.list().len(), 0);

        mgr.install(test_manifest("com.a"), vec![], vec![], None, None)
            .await
            .unwrap();
        mgr.install(test_manifest("com.b"), vec![], vec![], None, None)
            .await
            .unwrap();

        assert_eq!(mgr.list().len(), 2);
    }

    // -- start --

    #[tokio::test]
    async fn start_creates_fresh_container_and_waits() {
        let tmp = tempfile::tempdir().unwrap();
        let mock = Arc::new(MockRuntime::new());
        let mock_ref = Arc::clone(&mock);
        let mut mgr = test_manager(tmp.path(), mock);

        let m = test_manifest("com.test.start");
        let installed = mgr
            .install(m, vec![], vec![], None, None)
            .await
            .unwrap();
        let old_cid = installed.container_id.clone().unwrap();

        mgr.start("com.test.start").await.unwrap();

        let plugin = mgr.storage.get("com.test.start").unwrap();
        assert_eq!(plugin.status, PluginStatus::Running);

        // Should have a NEW container ID (start recreates the container)
        let new_cid = plugin.container_id.clone().unwrap();
        assert_ne!(old_cid, new_cid);

        // Old container should have been removed, new one created + started + waited
        assert!(mock_ref.was_called(&RuntimeCall::RemoveContainer(old_cid)));
        assert!(mock_ref.was_called(&RuntimeCall::StartContainer(new_cid.clone())));
        assert!(mock_ref.was_called(&RuntimeCall::WaitForReady {
            port: plugin.assigned_port,
            path: "/".into(),
        }));
    }

    #[tokio::test]
    async fn start_issues_fresh_auth_token() {
        let tmp = tempfile::tempdir().unwrap();
        let mock = Arc::new(MockRuntime::new());
        let mut mgr = test_manager(tmp.path(), mock);

        let m = test_manifest("com.test.token");
        mgr.install(m, vec![], vec![], None, None).await.unwrap();
        let token_before = mgr.storage.get("com.test.token").unwrap().auth_token.clone();

        mgr.start("com.test.token").await.unwrap();
        let token_after = mgr.storage.get("com.test.token").unwrap().auth_token.clone();

        assert_ne!(token_before, token_after, "start should rotate the auth token");
    }

    // -- update --

    #[tokio::test]
    async fn update_plugin_replaces_container_and_pulls_new_image() {
        let tmp = tempfile::tempdir().unwrap();
        let mock = Arc::new(MockRuntime::new());
        let mock_ref = Arc::clone(&mock);
        let mut mgr = test_manager(tmp.path(), mock);

        let m = test_manifest("com.test.update");
        mgr.install(m, vec![], vec![], None, None).await.unwrap();
        let old_cid = mgr
            .storage
            .get("com.test.update")
            .unwrap()
            .container_id
            .clone()
            .unwrap();

        // Build a v2 manifest
        let mut m2 = test_manifest("com.test.update");
        m2.version = "2.0.0".into();

        let updated = mgr.update_plugin(m2, None).await.unwrap();
        assert_eq!(updated.manifest.version, "2.0.0");
        assert_eq!(updated.status, PluginStatus::Stopped); // wasn't running

        let new_cid = updated.container_id.unwrap();
        assert_ne!(old_cid, new_cid);

        // Should have pulled the image and removed old container
        assert!(mock_ref.was_called(&RuntimeCall::RemoveContainer(old_cid)));
        assert!(mock_ref.was_called(&RuntimeCall::PullImage(
            "test-com-test-update:latest".into()
        )));
    }

    #[tokio::test]
    async fn update_plugin_restarts_if_was_running() {
        let tmp = tempfile::tempdir().unwrap();
        let mock = Arc::new(MockRuntime::new());
        let mock_ref = Arc::clone(&mock);
        let mut mgr = test_manager(tmp.path(), mock);

        let m = test_manifest("com.test.uprun");
        mgr.install(m, vec![], vec![], None, None).await.unwrap();

        // Simulate running state
        if let Some(p) = mgr.storage.get_mut("com.test.uprun") {
            p.status = PluginStatus::Running;
        }

        let mut m2 = test_manifest("com.test.uprun");
        m2.version = "2.0.0".into();
        let updated = mgr.update_plugin(m2, None).await.unwrap();

        assert_eq!(updated.status, PluginStatus::Running);

        // Should have started the new container and waited for readiness
        let new_cid = updated.container_id.unwrap();
        assert!(mock_ref.was_called(&RuntimeCall::StartContainer(new_cid)));
        assert!(mock_ref.was_called(&RuntimeCall::WaitForReady {
            port: updated.assigned_port,
            path: "/".into(),
        }));
    }

    // -- install + remove round-trip --

    #[tokio::test]
    async fn full_lifecycle_install_start_stop_remove() {
        let tmp = tempfile::tempdir().unwrap();
        let mock = Arc::new(MockRuntime::new());
        let mock_ref = Arc::clone(&mock);
        let mut mgr = test_manager(tmp.path(), mock);

        // Install
        let m = test_manifest("com.test.lifecycle");
        mgr.install(m, vec![], vec![], None, None).await.unwrap();
        assert_eq!(mgr.list().len(), 1);
        assert_eq!(
            mgr.storage.get("com.test.lifecycle").unwrap().status,
            PluginStatus::Stopped
        );

        // Start (real start, not simulated)
        mgr.start("com.test.lifecycle").await.unwrap();
        assert_eq!(
            mgr.storage.get("com.test.lifecycle").unwrap().status,
            PluginStatus::Running
        );

        // Stop
        mgr.stop("com.test.lifecycle").await.unwrap();
        assert_eq!(
            mgr.storage.get("com.test.lifecycle").unwrap().status,
            PluginStatus::Stopped
        );

        // Remove
        mgr.remove("com.test.lifecycle").await.unwrap();
        assert_eq!(mgr.list().len(), 0);

        // Verify the full call sequence covers every phase
        let calls = mock_ref.calls();
        let call_types: Vec<&str> = calls
            .iter()
            .map(|c| match c {
                RuntimeCall::ImageExists(_) => "image_exists",
                RuntimeCall::PullImage(_) => "pull",
                RuntimeCall::CreateContainer(_) => "create",
                RuntimeCall::StartContainer(_) => "start",
                RuntimeCall::WaitForReady { .. } => "wait_for_ready",
                RuntimeCall::StopContainer(_) => "stop",
                RuntimeCall::RemoveContainer(_) => "remove_container",
                RuntimeCall::RemoveImage(_) => "remove_image",
                RuntimeCall::RemoveVolume(_) => "remove_volume",
                _ => "other",
            })
            .collect();

        assert!(call_types.contains(&"image_exists"));
        assert!(call_types.contains(&"pull"));
        assert!(call_types.contains(&"create"));
        assert!(call_types.contains(&"start"));
        assert!(call_types.contains(&"wait_for_ready"));
        assert!(call_types.contains(&"stop"));
        assert!(call_types.contains(&"remove_container"));
        assert!(call_types.contains(&"remove_image"));
        assert!(call_types.contains(&"remove_volume"));
    }
}
