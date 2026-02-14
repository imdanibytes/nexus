pub mod docker;
pub mod health;
pub mod manifest;
pub mod registry;
pub mod storage;

use crate::error::{NexusError, NexusResult};
use crate::extensions::ipc::AppIpcRouter;
use crate::extensions::loader::ExtensionLoader;
use crate::extensions::registry::ExtensionRegistry;
use crate::permissions::PermissionStore;
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
    pub storage: PluginStorage,
    pub permissions: PermissionStore,
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
}

impl PluginManager {
    pub fn new(data_dir: PathBuf) -> Self {
        let storage = PluginStorage::load(&data_dir).unwrap_or_default();
        let permissions = PermissionStore::load(&data_dir).unwrap_or_default();
        let registry_store = registry::RegistryStore::load(&data_dir).unwrap_or_default();
        let settings = NexusSettings::load(&data_dir).unwrap_or_default();
        let plugin_settings = PluginSettingsStore::load(&data_dir).unwrap_or_default();
        let mcp_settings = McpSettings::load(&data_dir).unwrap_or_default();
        let update_state = crate::update_checker::load_update_state(&data_dir);

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
    ) -> NexusResult<InstalledPlugin> {
        manifest
            .validate()
            .map_err(NexusError::InvalidManifest)?;

        check_min_nexus_version(&manifest)?;

        if let Some(existing) = self.storage.get(&manifest.id) {
            log::info!("Reinstalling plugin '{}' (replacing existing)", manifest.id);

            // Stop and remove old container, but keep volume (data) and permissions
            if let Some(container_id) = &existing.container_id {
                if existing.status == PluginStatus::Running {
                    let _ = docker::stop_container(container_id).await;
                }
                let _ = docker::remove_container(container_id).await;
            }

            self.storage.remove(&manifest.id)?;
        }

        // Pull the Docker image (skip if already present — e.g. locally built)
        let image_exists = docker::image_exists(&manifest.image).await.unwrap_or(false);
        if image_exists {
            log::info!("Image already exists locally: {}", manifest.image);
        } else {
            log::info!("Pulling image: {}", manifest.image);
            docker::pull_image(&manifest.image).await?;
        }

        // Verify image digest if declared in manifest
        if let Some(ref expected_digest) = manifest.image_digest {
            match docker::get_image_digest(&manifest.image).await? {
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
            Some(&volume_name),
        )
        .await?;

        let plugin = InstalledPlugin {
            manifest,
            container_id: Some(container_id),
            status: PluginStatus::Stopped,
            assigned_port: port,
            auth_token: token_hash,
            installed_at: chrono::Utc::now(),
            manifest_url_origin: manifest_url.and_then(storage::extract_url_host),
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
            .unwrap_or_else(|| manifest.ui.path.clone());

        // Remove the old container (if any)
        if let Some(ref cid) = old_container_id {
            let _ = docker::stop_container(cid).await;
            let _ = docker::remove_container(cid).await;
        }

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

        let new_container_id = docker::create_container(
            &container_name,
            &manifest.image,
            port,
            manifest.ui.port,
            env_vars,
            labels,
            limits,
            Some(&volume_name),
        )
        .await?;

        docker::start_container(&new_container_id).await?;
        docker::wait_for_ready(port, &ready_path, std::time::Duration::from_secs(15)).await?;

        if let Some(plugin) = self.storage.get_mut(plugin_id) {
            plugin.auth_token = new_hash;
            plugin.container_id = Some(new_container_id);
            plugin.status = PluginStatus::Running;
        }
        self.storage.save()?;

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

        // Remove persistent data: Docker volume + KV storage
        let volume_name = data_volume_name(plugin_id);
        if let Err(e) = docker::remove_volume(&volume_name).await {
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

        docker::get_logs(container_id, tail).await
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

        // Stop old container
        if let Some(ref cid) = old_container_id {
            if was_running {
                let _ = docker::stop_container(cid).await;
            }
            let _ = docker::remove_container(cid).await;
        }

        // Pull new image
        log::info!("Pulling updated image: {}", manifest.image);
        docker::pull_image(&manifest.image).await?;

        // Verify digest if present
        if let Some(ref expected_digest) = manifest.image_digest {
            match docker::get_image_digest(&manifest.image).await? {
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

        let new_container_id = docker::create_container(
            &container_name,
            &manifest.image,
            port,
            manifest.ui.port,
            env_vars,
            labels,
            limits,
            Some(&volume_name),
        )
        .await?;

        let updated_plugin = InstalledPlugin {
            manifest,
            container_id: Some(new_container_id.clone()),
            status: PluginStatus::Stopped,
            assigned_port: port,
            auth_token: new_hash,
            installed_at: chrono::Utc::now(),
            manifest_url_origin: preserved_origin,
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
                .unwrap_or_else(|| updated_plugin.manifest.ui.path.clone());

            docker::start_container(&new_container_id).await?;
            docker::wait_for_ready(port, &ready_path, std::time::Duration::from_secs(15)).await?;

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
