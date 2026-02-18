pub mod docker;

#[cfg(test)]
pub mod mock;

use async_trait::async_trait;
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

// ---------------------------------------------------------------------------
// Error
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error("container not found: {0}")]
    NotFound(String),

    #[error("network error: {0}")]
    Network(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Other(String),
}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Resource limits applied to plugin containers.
#[derive(Debug, Clone, Default)]
pub struct ResourceLimits {
    /// CPU limit in nanoseconds (1e9 = 1 full CPU core).
    pub nano_cpus: Option<i64>,
    /// Memory limit in bytes.
    pub memory_bytes: Option<i64>,
}

/// Security hardening options for a container.
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    pub cap_drop: Vec<String>,
    pub cap_add: Vec<String>,
    pub no_new_privileges: bool,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            cap_drop: vec!["ALL".to_string()],
            cap_add: vec!["NET_BIND_SERVICE".to_string()],
            no_new_privileges: true,
        }
    }
}

/// All parameters needed to create a container.
#[derive(Debug, Clone)]
pub struct ContainerConfig {
    pub name: String,
    pub image: String,
    pub host_port: u16,
    pub container_port: u16,
    pub env_vars: Vec<String>,
    pub labels: HashMap<String, String>,
    pub limits: ResourceLimits,
    pub data_volume: Option<String>,
    pub network: String,
    pub security: SecurityConfig,
}

/// High-level container state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerState {
    Running,
    Stopped,
    Gone,
}

/// Lightweight container info returned by list operations.
#[derive(Debug, Clone, Serialize)]
pub struct ContainerInfo {
    pub id: String,
    pub names: Vec<String>,
    pub image: String,
    pub state: String,
    pub status: String,
}

/// Label-based filter for listing containers.
#[derive(Debug, Clone, Default)]
pub struct ContainerFilters {
    pub labels: HashMap<String, String>,
}

/// Aggregate resource usage across containers.
#[derive(Debug, Clone, Serialize)]
pub struct ResourceUsage {
    pub cpu_percent: f64,
    pub memory_mb: f64,
}

/// Lightweight image info returned by list operations.
#[derive(Debug, Clone, Serialize)]
pub struct ImageInfo {
    pub id: String,
    pub repo_tags: Vec<String>,
    pub size: i64,
    pub created: i64,
}

/// Lightweight volume info returned by list operations.
#[derive(Debug, Clone, Serialize)]
pub struct VolumeInfo {
    pub name: String,
    pub driver: String,
    pub mountpoint: String,
    pub created_at: Option<String>,
}

/// Lightweight network info returned by list operations.
#[derive(Debug, Clone, Serialize)]
pub struct NetworkInfo {
    pub id: String,
    pub name: String,
    pub driver: String,
    pub scope: String,
}

/// Container engine system-level info.
#[derive(Debug, Clone, Serialize)]
pub struct EngineInfo {
    pub engine_id: String,
    pub version: Option<String>,
    pub os: Option<String>,
    pub arch: Option<String>,
    pub cpus: Option<i64>,
    pub memory_bytes: Option<i64>,
}

// ---------------------------------------------------------------------------
// Trait
// ---------------------------------------------------------------------------

#[async_trait]
pub trait ContainerRuntime: Send + Sync {
    // Identity
    /// Short engine identifier, e.g. `"docker"`, `"podman"`. Frontend maps to display names.
    fn engine_id(&self) -> &str;
    /// Socket or pipe path used to connect to the engine.
    fn socket_path(&self) -> String;
    /// Hostname that resolves to the host machine from inside containers.
    /// Docker: `host.docker.internal`, Podman: `host.containers.internal`.
    fn host_gateway_hostname(&self) -> &str;

    // Daemon
    async fn ping(&self) -> Result<(), RuntimeError>;
    async fn version(&self) -> Result<Option<String>, RuntimeError>;

    // Network
    async fn ensure_network(&self, name: &str) -> Result<(), RuntimeError>;

    // Images
    async fn image_exists(&self, image: &str) -> Result<bool, RuntimeError>;
    async fn pull_image(&self, image: &str) -> Result<(), RuntimeError>;
    async fn build_image(&self, context_dir: &Path, tag: &str) -> Result<(), RuntimeError>;
    async fn get_image_digest(&self, image: &str) -> Result<Option<String>, RuntimeError>;
    async fn remove_image(&self, image: &str) -> Result<(), RuntimeError>;
    async fn list_images(&self) -> Result<Vec<ImageInfo>, RuntimeError>;
    async fn inspect_image_raw(&self, id: &str) -> Result<serde_json::Value, RuntimeError>;

    // Containers
    async fn create_container(&self, config: ContainerConfig) -> Result<String, RuntimeError>;
    async fn start_container(&self, id: &str) -> Result<(), RuntimeError>;
    async fn stop_container(&self, id: &str) -> Result<(), RuntimeError>;
    async fn restart_container(&self, id: &str) -> Result<(), RuntimeError>;
    async fn remove_container(&self, id: &str) -> Result<(), RuntimeError>;
    async fn container_state(&self, id: &str) -> Result<ContainerState, RuntimeError>;
    async fn list_containers(
        &self,
        filters: ContainerFilters,
    ) -> Result<Vec<ContainerInfo>, RuntimeError>;
    async fn get_logs(&self, id: &str, tail: u32) -> Result<Vec<String>, RuntimeError>;
    async fn inspect_container_raw(
        &self,
        id: &str,
    ) -> Result<serde_json::Value, RuntimeError>;

    // Stats
    async fn aggregate_stats(
        &self,
        filters: ContainerFilters,
    ) -> Result<ResourceUsage, RuntimeError>;

    // Volumes
    async fn list_volumes(&self) -> Result<Vec<VolumeInfo>, RuntimeError>;
    async fn remove_volume(&self, name: &str) -> Result<(), RuntimeError>;

    // Networks
    async fn list_networks(&self) -> Result<Vec<NetworkInfo>, RuntimeError>;
    async fn remove_network(&self, id: &str) -> Result<(), RuntimeError>;

    // Engine
    async fn engine_info(&self) -> Result<EngineInfo, RuntimeError>;

    // Readiness
    /// Poll a container's HTTP endpoint until it responds or the timeout expires.
    async fn wait_for_ready(
        &self,
        port: u16,
        path: &str,
        timeout: Duration,
    ) -> Result<(), RuntimeError>;
}
