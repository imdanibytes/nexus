//! In-memory mock implementation of `ContainerRuntime` for testing.
//!
//! Tracks all calls and manages fake container state so that `PluginManager`
//! can be tested without Docker.

use async_trait::async_trait;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

use super::{
    ContainerConfig, ContainerFilters, ContainerInfo, ContainerRuntime, ContainerState,
    EngineInfo, ImageInfo, NetworkInfo, ResourceUsage, RuntimeError, VolumeInfo,
};

// ---------------------------------------------------------------------------
// Call recording
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeCall {
    Ping,
    Version,
    EnsureNetwork(String),
    ImageExists(String),
    PullImage(String),
    BuildImage { context_dir: String, tag: String },
    GetImageDigest(String),
    ListImages,
    InspectImageRaw(String),
    RemoveImage(String),
    CreateContainer(String),  // name
    StartContainer(String),   // id
    StopContainer(String),    // id
    RestartContainer(String), // id
    RemoveContainer(String),  // id or name
    ContainerState(String),   // id
    ListContainers,
    GetLogs { id: String, tail: u32 },
    InspectContainerRaw(String),
    AggregateStats,
    ListVolumes,
    RemoveVolume(String),
    ListNetworks,
    RemoveNetwork(String),
    EngineInfo,
    WaitForReady { port: u16, path: String },
}

// ---------------------------------------------------------------------------
// Mock state
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct FakeContainer {
    id: String,
    name: String,
    image: String,
    running: bool,
}

#[derive(Debug)]
struct Inner {
    calls: Vec<RuntimeCall>,
    images: HashMap<String, Option<String>>, // image -> optional digest
    containers: HashMap<String, FakeContainer>, // id -> container
    container_by_name: HashMap<String, String>, // name -> id
    volumes: HashMap<String, ()>,
    next_id: u64,
    // Behavior overrides for testing edge cases
    fail_pull: bool,
    fail_create: bool,
    fail_start: bool,
}

pub struct MockRuntime {
    inner: Mutex<Inner>,
}

impl MockRuntime {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(Inner {
                calls: Vec::new(),
                images: HashMap::new(),
                containers: HashMap::new(),
                container_by_name: HashMap::new(),
                volumes: HashMap::new(),
                next_id: 1,
                fail_pull: false,
                fail_create: false,
                fail_start: false,
            }),
        }
    }

    /// Pre-populate with a local image so `pull_image` is skipped.
    pub fn with_image(self, image: &str) -> Self {
        self.inner.lock().unwrap().images.insert(image.to_string(), None);
        self
    }

    /// Pre-populate with an image + digest.
    pub fn with_image_digest(self, image: &str, digest: &str) -> Self {
        self.inner
            .lock()
            .unwrap()
            .images
            .insert(image.to_string(), Some(digest.to_string()));
        self
    }

    /// Make `pull_image` fail.
    pub fn fail_pull(self) -> Self {
        self.inner.lock().unwrap().fail_pull = true;
        self
    }

    /// Make `create_container` fail.
    pub fn fail_create(self) -> Self {
        self.inner.lock().unwrap().fail_create = true;
        self
    }

    /// Make `start_container` fail.
    pub fn fail_start(self) -> Self {
        self.inner.lock().unwrap().fail_start = true;
        self
    }

    /// Return all recorded calls.
    pub fn calls(&self) -> Vec<RuntimeCall> {
        self.inner.lock().unwrap().calls.clone()
    }

    /// Count how many times a specific call was made.
    pub fn call_count(&self, needle: &RuntimeCall) -> usize {
        self.inner
            .lock()
            .unwrap()
            .calls
            .iter()
            .filter(|c| *c == needle)
            .count()
    }

    /// Check if a specific call was made.
    pub fn was_called(&self, needle: &RuntimeCall) -> bool {
        self.call_count(needle) > 0
    }

    /// Return how many containers currently exist.
    pub fn container_count(&self) -> usize {
        self.inner.lock().unwrap().containers.len()
    }

    /// Return how many images currently exist.
    pub fn image_count(&self) -> usize {
        self.inner.lock().unwrap().images.len()
    }

    /// Check if a volume exists.
    pub fn volume_exists(&self, name: &str) -> bool {
        self.inner.lock().unwrap().volumes.contains_key(name)
    }
}

// ---------------------------------------------------------------------------
// ContainerRuntime implementation
// ---------------------------------------------------------------------------

#[async_trait]
impl ContainerRuntime for MockRuntime {
    fn engine_id(&self) -> &str {
        "mock"
    }

    fn socket_path(&self) -> String {
        "/tmp/mock.sock".to_string()
    }

    fn host_gateway_hostname(&self) -> &str {
        "host.docker.internal"
    }

    async fn ping(&self) -> Result<(), RuntimeError> {
        self.inner.lock().unwrap().calls.push(RuntimeCall::Ping);
        Ok(())
    }

    async fn version(&self) -> Result<Option<String>, RuntimeError> {
        self.inner.lock().unwrap().calls.push(RuntimeCall::Version);
        Ok(Some("mock-1.0.0".to_string()))
    }

    async fn ensure_network(&self, name: &str) -> Result<(), RuntimeError> {
        self.inner
            .lock()
            .unwrap()
            .calls
            .push(RuntimeCall::EnsureNetwork(name.to_string()));
        Ok(())
    }

    async fn image_exists(&self, image: &str) -> Result<bool, RuntimeError> {
        let mut inner = self.inner.lock().unwrap();
        inner.calls.push(RuntimeCall::ImageExists(image.to_string()));
        Ok(inner.images.contains_key(image))
    }

    async fn pull_image(&self, image: &str) -> Result<(), RuntimeError> {
        let mut inner = self.inner.lock().unwrap();
        inner.calls.push(RuntimeCall::PullImage(image.to_string()));

        if inner.fail_pull {
            return Err(RuntimeError::Network(format!(
                "mock: pull failed for {}",
                image
            )));
        }

        // Simulate: if already exists, skip. Otherwise add it.
        if !inner.images.contains_key(image) {
            inner.images.insert(image.to_string(), None);
        }
        Ok(())
    }

    async fn build_image(&self, context_dir: &Path, tag: &str) -> Result<(), RuntimeError> {
        let mut inner = self.inner.lock().unwrap();
        inner.calls.push(RuntimeCall::BuildImage {
            context_dir: context_dir.display().to_string(),
            tag: tag.to_string(),
        });
        inner.images.insert(tag.to_string(), None);
        Ok(())
    }

    async fn get_image_digest(&self, image: &str) -> Result<Option<String>, RuntimeError> {
        let mut inner = self.inner.lock().unwrap();
        inner
            .calls
            .push(RuntimeCall::GetImageDigest(image.to_string()));
        Ok(inner.images.get(image).and_then(|d| d.clone()))
    }

    async fn list_images(&self) -> Result<Vec<ImageInfo>, RuntimeError> {
        let mut inner = self.inner.lock().unwrap();
        inner.calls.push(RuntimeCall::ListImages);
        Ok(inner
            .images
            .iter()
            .map(|(name, _digest)| ImageInfo {
                id: format!("sha256:mock-{}", name),
                repo_tags: vec![name.clone()],
                size: 100_000_000,
                created: 1700000000,
            })
            .collect())
    }

    async fn inspect_image_raw(&self, id: &str) -> Result<serde_json::Value, RuntimeError> {
        let mut inner = self.inner.lock().unwrap();
        inner
            .calls
            .push(RuntimeCall::InspectImageRaw(id.to_string()));
        if inner.images.contains_key(id) {
            Ok(serde_json::json!({ "Id": id }))
        } else {
            Err(RuntimeError::NotFound(id.to_string()))
        }
    }

    async fn remove_image(&self, image: &str) -> Result<(), RuntimeError> {
        let mut inner = self.inner.lock().unwrap();
        inner
            .calls
            .push(RuntimeCall::RemoveImage(image.to_string()));
        inner.images.remove(image);
        Ok(())
    }

    async fn create_container(&self, config: ContainerConfig) -> Result<String, RuntimeError> {
        let mut inner = self.inner.lock().unwrap();
        inner
            .calls
            .push(RuntimeCall::CreateContainer(config.name.clone()));

        if inner.fail_create {
            return Err(RuntimeError::Other("mock: create failed".to_string()));
        }

        let id = format!("mock-container-{}", inner.next_id);
        inner.next_id += 1;

        // Track the volume if provided
        if let Some(ref vol) = config.data_volume {
            inner.volumes.insert(vol.clone(), ());
        }

        inner.container_by_name.insert(config.name.clone(), id.clone());
        inner.containers.insert(
            id.clone(),
            FakeContainer {
                id: id.clone(),
                name: config.name,
                image: config.image,
                running: false,
            },
        );

        Ok(id)
    }

    async fn start_container(&self, id: &str) -> Result<(), RuntimeError> {
        let mut inner = self.inner.lock().unwrap();
        inner
            .calls
            .push(RuntimeCall::StartContainer(id.to_string()));

        if inner.fail_start {
            return Err(RuntimeError::Other("mock: start failed".to_string()));
        }

        if let Some(c) = inner.containers.get_mut(id) {
            c.running = true;
            Ok(())
        } else {
            Err(RuntimeError::NotFound(id.to_string()))
        }
    }

    async fn stop_container(&self, id: &str) -> Result<(), RuntimeError> {
        let mut inner = self.inner.lock().unwrap();
        inner.calls.push(RuntimeCall::StopContainer(id.to_string()));

        if let Some(c) = inner.containers.get_mut(id) {
            c.running = false;
        }
        Ok(())
    }

    async fn restart_container(&self, id: &str) -> Result<(), RuntimeError> {
        let mut inner = self.inner.lock().unwrap();
        inner
            .calls
            .push(RuntimeCall::RestartContainer(id.to_string()));

        if let Some(c) = inner.containers.get_mut(id) {
            c.running = true;
            Ok(())
        } else {
            Err(RuntimeError::NotFound(id.to_string()))
        }
    }

    async fn remove_container(&self, id_or_name: &str) -> Result<(), RuntimeError> {
        let mut inner = self.inner.lock().unwrap();
        inner
            .calls
            .push(RuntimeCall::RemoveContainer(id_or_name.to_string()));

        // Try by ID first, then by name
        if inner.containers.remove(id_or_name).is_some() {
            inner.container_by_name.retain(|_, v| v != id_or_name);
        } else if let Some(id) = inner.container_by_name.remove(id_or_name) {
            inner.containers.remove(&id);
        }
        // Don't error on missing — matches how PluginManager uses it
        Ok(())
    }

    async fn container_state(&self, id: &str) -> Result<ContainerState, RuntimeError> {
        let mut inner = self.inner.lock().unwrap();
        inner
            .calls
            .push(RuntimeCall::ContainerState(id.to_string()));

        match inner.containers.get(id) {
            Some(c) if c.running => Ok(ContainerState::Running),
            Some(_) => Ok(ContainerState::Stopped),
            None => Ok(ContainerState::Gone),
        }
    }

    async fn list_containers(
        &self,
        _filters: ContainerFilters,
    ) -> Result<Vec<ContainerInfo>, RuntimeError> {
        let mut inner = self.inner.lock().unwrap();
        inner.calls.push(RuntimeCall::ListContainers);

        Ok(inner
            .containers
            .values()
            .map(|c| ContainerInfo {
                id: c.id.clone(),
                names: vec![c.name.clone()],
                image: c.image.clone(),
                state: if c.running {
                    "running".to_string()
                } else {
                    "exited".to_string()
                },
                status: if c.running {
                    "Up 5 minutes".to_string()
                } else {
                    "Exited (0) 5 minutes ago".to_string()
                },
            })
            .collect())
    }

    async fn get_logs(&self, id: &str, tail: u32) -> Result<Vec<String>, RuntimeError> {
        let mut inner = self.inner.lock().unwrap();
        inner.calls.push(RuntimeCall::GetLogs {
            id: id.to_string(),
            tail,
        });

        if inner.containers.contains_key(id) {
            Ok(vec![
                "mock log line 1".to_string(),
                "mock log line 2".to_string(),
            ])
        } else {
            Err(RuntimeError::NotFound(id.to_string()))
        }
    }

    async fn inspect_container_raw(
        &self,
        id: &str,
    ) -> Result<serde_json::Value, RuntimeError> {
        let mut inner = self.inner.lock().unwrap();
        inner
            .calls
            .push(RuntimeCall::InspectContainerRaw(id.to_string()));

        match inner.containers.get(id) {
            Some(c) => Ok(serde_json::json!({
                "Id": c.id,
                "Name": c.name,
                "State": { "Running": c.running },
            })),
            None => Err(RuntimeError::NotFound(id.to_string())),
        }
    }

    async fn container_stats_raw(
        &self,
        id: &str,
    ) -> Result<serde_json::Value, RuntimeError> {
        let inner = self.inner.lock().unwrap();
        if inner.containers.contains_key(id) {
            Ok(serde_json::json!({
                "cpu_stats": {
                    "cpu_usage": { "total_usage": 100000 },
                    "system_cpu_usage": 1000000,
                    "online_cpus": 2
                },
                "precpu_stats": {
                    "cpu_usage": { "total_usage": 90000 },
                    "system_cpu_usage": 900000
                },
                "memory_stats": {
                    "usage": 52428800_u64,
                    "limit": 2147483648_u64
                }
            }))
        } else {
            Err(RuntimeError::NotFound(id.to_string()))
        }
    }

    async fn aggregate_stats(
        &self,
        _filters: ContainerFilters,
    ) -> Result<ResourceUsage, RuntimeError> {
        let mut inner = self.inner.lock().unwrap();
        inner.calls.push(RuntimeCall::AggregateStats);

        Ok(ResourceUsage {
            cpu_percent: 0.0,
            memory_mb: 0.0,
        })
    }

    async fn list_volumes(&self) -> Result<Vec<VolumeInfo>, RuntimeError> {
        let mut inner = self.inner.lock().unwrap();
        inner.calls.push(RuntimeCall::ListVolumes);
        Ok(inner
            .volumes
            .keys()
            .map(|name| VolumeInfo {
                name: name.clone(),
                driver: "local".to_string(),
                mountpoint: format!("/var/lib/docker/volumes/{}", name),
                created_at: None,
            })
            .collect())
    }

    async fn remove_volume(&self, name: &str) -> Result<(), RuntimeError> {
        let mut inner = self.inner.lock().unwrap();
        inner
            .calls
            .push(RuntimeCall::RemoveVolume(name.to_string()));
        inner.volumes.remove(name);
        Ok(())
    }

    async fn list_networks(&self) -> Result<Vec<NetworkInfo>, RuntimeError> {
        let mut inner = self.inner.lock().unwrap();
        inner.calls.push(RuntimeCall::ListNetworks);
        Ok(vec![NetworkInfo {
            id: "mock-network-1".to_string(),
            name: "bridge".to_string(),
            driver: "bridge".to_string(),
            scope: "local".to_string(),
        }])
    }

    async fn remove_network(&self, id: &str) -> Result<(), RuntimeError> {
        let mut inner = self.inner.lock().unwrap();
        inner
            .calls
            .push(RuntimeCall::RemoveNetwork(id.to_string()));
        Ok(())
    }

    async fn engine_info(&self) -> Result<EngineInfo, RuntimeError> {
        let mut inner = self.inner.lock().unwrap();
        inner.calls.push(RuntimeCall::EngineInfo);
        Ok(EngineInfo {
            engine_id: "mock".to_string(),
            version: Some("mock-1.0.0".to_string()),
            os: Some("linux".to_string()),
            arch: Some("amd64".to_string()),
            cpus: Some(4),
            memory_bytes: Some(8_000_000_000),
        })
    }

    async fn wait_for_ready(
        &self,
        port: u16,
        path: &str,
        _timeout: std::time::Duration,
    ) -> Result<(), RuntimeError> {
        let mut inner = self.inner.lock().unwrap();
        inner.calls.push(RuntimeCall::WaitForReady {
            port,
            path: path.to_string(),
        });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn ping_and_version() {
        let rt = MockRuntime::new();
        assert!(rt.ping().await.is_ok());
        let v = rt.version().await.unwrap();
        assert_eq!(v, Some("mock-1.0.0".to_string()));
        assert!(rt.was_called(&RuntimeCall::Ping));
        assert!(rt.was_called(&RuntimeCall::Version));
    }

    #[tokio::test]
    async fn image_lifecycle() {
        let rt = MockRuntime::new();

        assert!(!rt.image_exists("test:latest").await.unwrap());
        rt.pull_image("test:latest").await.unwrap();
        assert!(rt.image_exists("test:latest").await.unwrap());
        rt.remove_image("test:latest").await.unwrap();
        assert!(!rt.image_exists("test:latest").await.unwrap());
    }

    #[tokio::test]
    async fn pull_skips_existing() {
        let rt = MockRuntime::new().with_image("cached:v1");
        rt.pull_image("cached:v1").await.unwrap();
        assert_eq!(rt.image_count(), 1);
    }

    #[tokio::test]
    async fn pull_failure() {
        let rt = MockRuntime::new().fail_pull();
        let result = rt.pull_image("fail:latest").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RuntimeError::Network(_)));
    }

    #[tokio::test]
    async fn container_lifecycle() {
        let rt = MockRuntime::new().with_image("my-image:latest");

        let id = rt
            .create_container(ContainerConfig {
                name: "test-container".into(),
                image: "my-image:latest".into(),
                host_port: 8080,
                container_port: 80,
                env_vars: vec![],
                labels: HashMap::new(),
                limits: Default::default(),
                data_volume: None,
                network: "bridge".into(),
                security: Default::default(),
            })
            .await
            .unwrap();

        // Created but not running
        assert_eq!(
            rt.container_state(&id).await.unwrap(),
            ContainerState::Stopped
        );

        // Start
        rt.start_container(&id).await.unwrap();
        assert_eq!(
            rt.container_state(&id).await.unwrap(),
            ContainerState::Running
        );

        // Stop
        rt.stop_container(&id).await.unwrap();
        assert_eq!(
            rt.container_state(&id).await.unwrap(),
            ContainerState::Stopped
        );

        // Remove
        rt.remove_container(&id).await.unwrap();
        assert_eq!(
            rt.container_state(&id).await.unwrap(),
            ContainerState::Gone
        );
    }

    #[tokio::test]
    async fn remove_by_name() {
        let rt = MockRuntime::new().with_image("img:1");

        let id = rt
            .create_container(ContainerConfig {
                name: "named-container".into(),
                image: "img:1".into(),
                host_port: 9000,
                container_port: 80,
                env_vars: vec![],
                labels: HashMap::new(),
                limits: Default::default(),
                data_volume: None,
                network: "bridge".into(),
                security: Default::default(),
            })
            .await
            .unwrap();

        assert_eq!(rt.container_count(), 1);
        rt.remove_container("named-container").await.unwrap();
        assert_eq!(rt.container_count(), 0);
        assert_eq!(
            rt.container_state(&id).await.unwrap(),
            ContainerState::Gone
        );
    }

    #[tokio::test]
    async fn volume_tracking() {
        let rt = MockRuntime::new().with_image("img:1");

        rt.create_container(ContainerConfig {
            name: "vol-test".into(),
            image: "img:1".into(),
            host_port: 9000,
            container_port: 80,
            env_vars: vec![],
            labels: HashMap::new(),
            limits: Default::default(),
            data_volume: Some("my-volume".into()),
            network: "bridge".into(),
            security: Default::default(),
        })
        .await
        .unwrap();

        assert!(rt.volume_exists("my-volume"));
        rt.remove_volume("my-volume").await.unwrap();
        assert!(!rt.volume_exists("my-volume"));
    }

    #[tokio::test]
    async fn logs_returns_lines() {
        let rt = MockRuntime::new().with_image("img:1");
        let id = rt
            .create_container(ContainerConfig {
                name: "log-test".into(),
                image: "img:1".into(),
                host_port: 9000,
                container_port: 80,
                env_vars: vec![],
                labels: HashMap::new(),
                limits: Default::default(),
                data_volume: None,
                network: "bridge".into(),
                security: Default::default(),
            })
            .await
            .unwrap();

        let logs = rt.get_logs(&id, 100).await.unwrap();
        assert_eq!(logs.len(), 2);
    }

    #[tokio::test]
    async fn logs_missing_container_errors() {
        let rt = MockRuntime::new();
        let result = rt.get_logs("nonexistent", 100).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn inspect_raw_returns_json() {
        let rt = MockRuntime::new().with_image("img:1");
        let id = rt
            .create_container(ContainerConfig {
                name: "inspect-test".into(),
                image: "img:1".into(),
                host_port: 9000,
                container_port: 80,
                env_vars: vec![],
                labels: HashMap::new(),
                limits: Default::default(),
                data_volume: None,
                network: "bridge".into(),
                security: Default::default(),
            })
            .await
            .unwrap();

        let json = rt.inspect_container_raw(&id).await.unwrap();
        assert_eq!(json["Id"], id);
    }

    #[tokio::test]
    async fn list_containers_returns_all() {
        let rt = MockRuntime::new().with_image("img:1");
        for i in 0..3 {
            rt.create_container(ContainerConfig {
                name: format!("c-{}", i),
                image: "img:1".into(),
                host_port: 9000 + i,
                container_port: 80,
                env_vars: vec![],
                labels: HashMap::new(),
                limits: Default::default(),
                data_volume: None,
                network: "bridge".into(),
                security: Default::default(),
            })
            .await
            .unwrap();
        }

        let list = rt.list_containers(ContainerFilters::default()).await.unwrap();
        assert_eq!(list.len(), 3);
    }

    #[tokio::test]
    async fn create_failure() {
        let rt = MockRuntime::new().fail_create();
        let result = rt
            .create_container(ContainerConfig {
                name: "fail".into(),
                image: "img:1".into(),
                host_port: 9000,
                container_port: 80,
                env_vars: vec![],
                labels: HashMap::new(),
                limits: Default::default(),
                data_volume: None,
                network: "bridge".into(),
                security: Default::default(),
            })
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn image_digest() {
        let rt = MockRuntime::new().with_image_digest("img:1", "sha256:abc123");

        let digest = rt.get_image_digest("img:1").await.unwrap();
        assert_eq!(digest, Some("sha256:abc123".to_string()));

        let none = rt.get_image_digest("missing:1").await.unwrap();
        assert_eq!(none, None);
    }

    #[tokio::test]
    async fn call_history_is_ordered() {
        let rt = MockRuntime::new();
        rt.ping().await.unwrap();
        rt.version().await.unwrap();
        rt.ensure_network("test").await.unwrap();

        let calls = rt.calls();
        assert_eq!(calls[0], RuntimeCall::Ping);
        assert_eq!(calls[1], RuntimeCall::Version);
        assert_eq!(
            calls[2],
            RuntimeCall::EnsureNetwork("test".to_string())
        );
    }
}
