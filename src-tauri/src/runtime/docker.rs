use async_trait::async_trait;
use bollard::container::LogOutput;
use bollard::query_parameters::{
    BuildImageOptions, CreateContainerOptions, CreateImageOptions, ListContainersOptions,
    ListNetworksOptions, LogsOptions, RemoveContainerOptions, RemoveImageOptions,
    StartContainerOptions, StatsOptions, StopContainerOptions,
};
use bollard::service::{
    ContainerCreateBody, ContainerSummaryStateEnum, HostConfig, Mount, MountTypeEnum,
    NetworkCreateRequest, PortBinding,
};
use bollard::Docker;
use futures_util::StreamExt;
use std::collections::HashMap;
use std::path::Path;

use super::{
    ContainerConfig, ContainerFilters, ContainerInfo, ContainerRuntime, ContainerState,
    ResourceUsage, RuntimeError,
};

// ---------------------------------------------------------------------------
// DockerRuntime
// ---------------------------------------------------------------------------

pub struct DockerRuntime {
    docker: Docker,
}

impl DockerRuntime {
    pub fn new() -> Result<Self, RuntimeError> {
        let docker = Docker::connect_with_local_defaults()
            .map_err(|e| RuntimeError::Other(format!("Docker connection failed: {e}")))?;
        Ok(Self { docker })
    }
}

fn to_err(e: bollard::errors::Error) -> RuntimeError {
    match &e {
        bollard::errors::Error::DockerResponseServerError {
            status_code: 404, ..
        } => RuntimeError::NotFound(e.to_string()),
        _ => RuntimeError::Other(e.to_string()),
    }
}

// ---------------------------------------------------------------------------
// Trait implementation
// ---------------------------------------------------------------------------

#[async_trait]
impl ContainerRuntime for DockerRuntime {
    fn engine_id(&self) -> &str {
        "docker"
    }

    fn socket_path(&self) -> String {
        #[cfg(unix)]
        {
            std::env::var("DOCKER_HOST")
                .unwrap_or_else(|_| "/var/run/docker.sock".to_string())
        }
        #[cfg(windows)]
        {
            std::env::var("DOCKER_HOST")
                .unwrap_or_else(|_| r"\\.\pipe\docker_engine".to_string())
        }
    }

    async fn ping(&self) -> Result<(), RuntimeError> {
        self.docker.ping().await.map_err(to_err)?;
        Ok(())
    }

    async fn version(&self) -> Result<Option<String>, RuntimeError> {
        let v = self.docker.version().await.map_err(to_err)?;
        Ok(v.version)
    }

    async fn ensure_network(&self, name: &str) -> Result<(), RuntimeError> {
        let networks = self
            .docker
            .list_networks(None::<ListNetworksOptions>)
            .await
            .map_err(to_err)?;

        let exists = networks
            .iter()
            .any(|n| n.name.as_deref().is_some_and(|n| n == name));

        if !exists {
            self.docker
                .create_network(NetworkCreateRequest {
                    name: name.to_string(),
                    driver: Some("bridge".to_string()),
                    ..Default::default()
                })
                .await
                .map_err(to_err)?;
            log::info!("Created Docker network: {}", name);
        }

        Ok(())
    }

    async fn image_exists(&self, image: &str) -> Result<bool, RuntimeError> {
        match self.docker.inspect_image(image).await {
            Ok(_) => Ok(true),
            Err(bollard::errors::Error::DockerResponseServerError {
                status_code: 404, ..
            }) => Ok(false),
            Err(e) => Err(to_err(e)),
        }
    }

    async fn pull_image(&self, image: &str) -> Result<(), RuntimeError> {
        if self.image_exists(image).await? {
            log::info!("Image {} found locally, skipping pull", image);
            return Ok(());
        }

        let (repo, tag) = if let Some((r, t)) = image.rsplit_once(':') {
            (r, t)
        } else {
            (image, "latest")
        };

        let opts = CreateImageOptions {
            from_image: Some(repo.to_string()),
            tag: Some(tag.to_string()),
            ..Default::default()
        };

        let mut stream = self.docker.create_image(Some(opts), None, None);
        while let Some(result) = stream.next().await {
            match result {
                Ok(info) => {
                    if let Some(status) = info.status {
                        log::debug!("Pull: {}", status);
                    }
                }
                Err(e) => return Err(to_err(e)),
            }
        }

        Ok(())
    }

    async fn build_image(&self, context_dir: &Path, tag: &str) -> Result<(), RuntimeError> {
        let tar_bytes = create_build_context(context_dir)?;

        let opts = BuildImageOptions {
            t: Some(tag.to_string()),
            rm: true,
            forcerm: true,
            ..Default::default()
        };

        let body = bollard::body_full(tar_bytes.into());
        let mut stream = self.docker.build_image(opts, None, Some(body));
        while let Some(result) = stream.next().await {
            match result {
                Ok(info) => {
                    if let Some(stream) = info.stream {
                        let msg = stream.trim();
                        if !msg.is_empty() {
                            log::debug!("Build: {}", msg);
                        }
                    }
                    if let Some(detail) = info.error_detail {
                        let msg = detail.message.unwrap_or_default();
                        return Err(RuntimeError::Other(format!("Docker build error: {}", msg)));
                    }
                }
                Err(e) => return Err(to_err(e)),
            }
        }

        log::info!("Built image: {}", tag);
        Ok(())
    }

    async fn get_image_digest(&self, image: &str) -> Result<Option<String>, RuntimeError> {
        let inspect = self.docker.inspect_image(image).await.map_err(to_err)?;

        if let Some(repo_digests) = inspect.repo_digests {
            for digest_str in &repo_digests {
                if let Some(digest) = digest_str.split('@').nth(1) {
                    if digest.starts_with("sha256:") {
                        return Ok(Some(digest.to_string()));
                    }
                }
            }
        }

        Ok(None)
    }

    async fn remove_image(&self, image: &str) -> Result<(), RuntimeError> {
        self.docker
            .remove_image(
                image,
                Some(RemoveImageOptions {
                    force: false,
                    noprune: false,
                    ..Default::default()
                }),
                None,
            )
            .await
            .map_err(to_err)?;
        Ok(())
    }

    async fn create_container(&self, config: ContainerConfig) -> Result<String, RuntimeError> {
        let port_binding = PortBinding {
            host_ip: Some("127.0.0.1".to_string()),
            host_port: Some(config.host_port.to_string()),
        };

        let container_port_key = format!("{}/tcp", config.container_port);

        let mut port_bindings = HashMap::new();
        port_bindings.insert(container_port_key.clone(), Some(vec![port_binding]));

        let mounts = match config.data_volume {
            Some(ref vol_name) => vec![Mount {
                target: Some("/data".to_string()),
                source: Some(vol_name.to_string()),
                typ: Some(MountTypeEnum::VOLUME),
                read_only: Some(false),
                ..Default::default()
            }],
            None => vec![],
        };

        let host_config = HostConfig {
            port_bindings: Some(port_bindings),
            network_mode: Some(config.network.clone()),
            extra_hosts: Some(vec!["host.docker.internal:host-gateway".to_string()]),
            cap_drop: Some(config.security.cap_drop.clone()),
            cap_add: Some(config.security.cap_add.clone()),
            security_opt: if config.security.no_new_privileges {
                Some(vec!["no-new-privileges:true".to_string()])
            } else {
                None
            },
            binds: Some(vec![]),
            mounts: Some(mounts),
            nano_cpus: config.limits.nano_cpus,
            memory: config.limits.memory_bytes,
            ..Default::default()
        };

        let body = ContainerCreateBody {
            image: Some(config.image.clone()),
            env: Some(config.env_vars.clone()),
            labels: Some(config.labels.clone()),
            exposed_ports: Some(vec![container_port_key]),
            host_config: Some(host_config),
            ..Default::default()
        };

        let opts = CreateContainerOptions {
            name: Some(config.name.clone()),
            ..Default::default()
        };

        let response = self
            .docker
            .create_container(Some(opts), body)
            .await
            .map_err(to_err)?;

        Ok(response.id)
    }

    async fn start_container(&self, id: &str) -> Result<(), RuntimeError> {
        self.docker
            .start_container(id, None::<StartContainerOptions>)
            .await
            .map_err(to_err)?;
        Ok(())
    }

    async fn stop_container(&self, id: &str) -> Result<(), RuntimeError> {
        self.docker
            .stop_container(id, Some(StopContainerOptions { t: Some(10), signal: None }))
            .await
            .map_err(to_err)?;
        Ok(())
    }

    async fn remove_container(&self, id: &str) -> Result<(), RuntimeError> {
        self.docker
            .remove_container(
                id,
                Some(RemoveContainerOptions {
                    force: true,
                    ..Default::default()
                }),
            )
            .await
            .map_err(to_err)?;
        Ok(())
    }

    async fn container_state(&self, id: &str) -> Result<ContainerState, RuntimeError> {
        let info = self
            .docker
            .inspect_container(id, None)
            .await
            .map_err(to_err)?;

        let running = info.state.and_then(|s| s.running).unwrap_or(false);

        Ok(if running {
            ContainerState::Running
        } else {
            ContainerState::Stopped
        })
    }

    async fn list_containers(
        &self,
        filters: ContainerFilters,
    ) -> Result<Vec<ContainerInfo>, RuntimeError> {
        let mut filter_map: HashMap<String, Vec<String>> = HashMap::new();
        for (k, v) in &filters.labels {
            filter_map
                .entry("label".to_string())
                .or_default()
                .push(if v.is_empty() {
                    k.clone()
                } else {
                    format!("{k}={v}")
                });
        }

        let opts = ListContainersOptions {
            all: true,
            filters: Some(filter_map),
            ..Default::default()
        };

        let containers = self.docker.list_containers(Some(opts)).await.map_err(to_err)?;

        Ok(containers
            .into_iter()
            .map(|c| ContainerInfo {
                id: c.id.unwrap_or_default(),
                names: c.names.unwrap_or_default(),
                image: c.image.unwrap_or_default(),
                state: c.state.map(|s| s.to_string()).unwrap_or_default(),
                status: c.status.unwrap_or_default(),
            })
            .collect())
    }

    async fn get_logs(&self, id: &str, tail: u32) -> Result<Vec<String>, RuntimeError> {
        let opts = LogsOptions {
            stdout: true,
            stderr: true,
            tail: tail.to_string(),
            ..Default::default()
        };

        let mut stream = self.docker.logs(id, Some(opts));
        let mut lines = Vec::new();

        while let Some(result) = stream.next().await {
            match result {
                Ok(output) => {
                    let line = match output {
                        LogOutput::StdOut { message } => {
                            String::from_utf8_lossy(&message).to_string()
                        }
                        LogOutput::StdErr { message } => {
                            String::from_utf8_lossy(&message).to_string()
                        }
                        _ => continue,
                    };
                    lines.push(line);
                }
                Err(e) => return Err(to_err(e)),
            }
        }

        Ok(lines)
    }

    async fn inspect_container_raw(
        &self,
        id: &str,
    ) -> Result<serde_json::Value, RuntimeError> {
        let info = self
            .docker
            .inspect_container(id, None)
            .await
            .map_err(to_err)?;

        serde_json::to_value(info)
            .map_err(|e| RuntimeError::Other(format!("JSON serialization failed: {e}")))
    }

    async fn aggregate_stats(
        &self,
        filters: ContainerFilters,
    ) -> Result<ResourceUsage, RuntimeError> {
        let containers = self.list_containers(filters).await?;

        let mut total_cpu = 0.0_f64;
        let mut total_memory_bytes = 0_u64;

        for container in &containers {
            if container.state != ContainerSummaryStateEnum::RUNNING.to_string() {
                continue;
            }

            let opts = StatsOptions {
                stream: false,
                one_shot: true,
            };

            let mut stream = self.docker.stats(&container.id, Some(opts));
            if let Some(Ok(stats)) = stream.next().await {
                if let (Some(cpu), Some(precpu)) = (&stats.cpu_stats, &stats.precpu_stats) {
                    if let (Some(cpu_usage), Some(precpu_usage)) =
                        (&cpu.cpu_usage, &precpu.cpu_usage)
                    {
                        let cpu_delta = cpu_usage.total_usage.unwrap_or(0) as f64
                            - precpu_usage.total_usage.unwrap_or(0) as f64;
                        let system_delta = cpu.system_cpu_usage.unwrap_or(0) as f64
                            - precpu.system_cpu_usage.unwrap_or(0) as f64;
                        let num_cpus = cpu.online_cpus.unwrap_or(1) as f64;

                        if system_delta > 0.0 {
                            total_cpu += (cpu_delta / system_delta) * num_cpus * 100.0;
                        }
                    }
                }

                if let Some(mem) = &stats.memory_stats {
                    total_memory_bytes += mem.usage.unwrap_or(0);
                }
            }
        }

        Ok(ResourceUsage {
            cpu_percent: (total_cpu * 10.0).round() / 10.0,
            memory_mb: (total_memory_bytes as f64 / 1_048_576.0 * 10.0).round() / 10.0,
        })
    }

    async fn remove_volume(&self, name: &str) -> Result<(), RuntimeError> {
        self.docker
            .remove_volume(name, None::<bollard::query_parameters::RemoveVolumeOptions>)
            .await
            .map_err(to_err)?;
        log::info!("Removed Docker volume: {}", name);
        Ok(())
    }

    async fn wait_for_ready(
        &self,
        port: u16,
        path: &str,
        timeout: std::time::Duration,
    ) -> Result<(), RuntimeError> {
        wait_for_ready(port, path, timeout).await
    }
}

// ---------------------------------------------------------------------------
// Standalone utilities (not part of the trait)
// ---------------------------------------------------------------------------

/// Check if a Docker image is available — locally first, then in its remote registry.
///
/// Supports ghcr.io and Docker Hub for remote checks. Returns false for unrecognized
/// registries or network errors (fail-open: install button still enabled, pull will
/// fail with a clear error message instead).
pub async fn check_image_available(runtime: &dyn ContainerRuntime, image: &str) -> bool {
    match runtime.image_exists(image).await {
        Ok(true) => return true,
        Ok(false) => {}
        Err(e) => {
            log::warn!("Local image check failed for {}: {}", image, e);
        }
    }

    match check_image_available_inner(image).await {
        Ok(available) => available,
        Err(e) => {
            log::warn!("Image availability check failed for {}: {}", image, e);
            true
        }
    }
}

async fn check_image_available_inner(image: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let (registry, repository, tag) = parse_image_ref(image);

    let (token_url, manifest_url) = match registry.as_str() {
        "ghcr.io" => (
            format!(
                "https://ghcr.io/token?scope=repository:{}:pull",
                repository
            ),
            format!(
                "https://ghcr.io/v2/{}/manifests/{}",
                repository, tag
            ),
        ),
        "docker.io" | "" => (
            format!(
                "https://auth.docker.io/token?service=registry.docker.io&scope=repository:{}:pull",
                repository
            ),
            format!(
                "https://registry-1.docker.io/v2/{}/manifests/{}",
                repository, tag
            ),
        ),
        other => {
            let base = format!("https://{}", other);
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .build()?;
            let resp = client
                .head(format!("{}/v2/{}/manifests/{}", base, repository, tag))
                .header(
                    "Accept",
                    "application/vnd.oci.image.index.v1+json, application/vnd.oci.image.manifest.v1+json, application/vnd.docker.distribution.manifest.list.v2+json, application/vnd.docker.distribution.manifest.v2+json",
                )
                .send()
                .await?;
            return Ok(resp.status().is_success());
        }
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    #[derive(serde::Deserialize)]
    struct TokenResponse {
        token: String,
    }
    let token_resp: TokenResponse = client.get(&token_url).send().await?.json().await?;

    let resp = client
        .head(&manifest_url)
        .header("Authorization", format!("Bearer {}", token_resp.token))
        .header(
            "Accept",
            "application/vnd.oci.image.index.v1+json, application/vnd.oci.image.manifest.v1+json, application/vnd.docker.distribution.manifest.list.v2+json, application/vnd.docker.distribution.manifest.v2+json",
        )
        .send()
        .await?;

    Ok(resp.status().is_success())
}

fn parse_image_ref(image: &str) -> (String, String, String) {
    let (name, tag) = if let Some((n, t)) = image.rsplit_once(':') {
        (n, t)
    } else {
        (image, "latest")
    };

    let parts: Vec<&str> = name.splitn(2, '/').collect();
    if parts.len() == 2 && (parts[0].contains('.') || parts[0].contains(':')) {
        let registry = parts[0].to_string();
        let repository = parts[1].to_string();
        (registry, repository, tag.to_string())
    } else if parts.len() == 2 {
        ("docker.io".to_string(), name.to_string(), tag.to_string())
    } else {
        (
            "docker.io".to_string(),
            format!("library/{}", name),
            tag.to_string(),
        )
    }
}

// ---------------------------------------------------------------------------
// Build-context helpers (internal)
// ---------------------------------------------------------------------------

fn parse_dockerignore(context_dir: &Path) -> Vec<(String, bool)> {
    let ignore_path = context_dir.join(".dockerignore");
    let content = match std::fs::read_to_string(&ignore_path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    content
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                return None;
            }
            if let Some(pattern) = trimmed.strip_prefix('!') {
                let pattern = pattern.trim_end_matches('/');
                Some((pattern.to_string(), true))
            } else {
                let pattern = trimmed.trim_end_matches('/');
                Some((pattern.to_string(), false))
            }
        })
        .collect()
}

fn is_ignored(rel_path: &str, rules: &[(String, bool)]) -> bool {
    let mut ignored = false;
    for (pattern, negated) in rules {
        let matches =
            glob_match(pattern, rel_path) || rel_path.starts_with(&format!("{}/", pattern));
        if matches {
            ignored = !negated;
        }
    }
    ignored
}

fn glob_match(pattern: &str, text: &str) -> bool {
    let mut regex_str = String::from("^");
    for ch in pattern.chars() {
        match ch {
            '*' => regex_str.push_str(".*"),
            '?' => regex_str.push('.'),
            '.' | '+' | '(' | ')' | '{' | '}' | '[' | ']' | '^' | '$' | '|' | '\\' => {
                regex_str.push('\\');
                regex_str.push(ch);
            }
            _ => regex_str.push(ch),
        }
    }
    regex_str.push('$');
    regex::Regex::new(&regex_str)
        .map(|re| re.is_match(text))
        .unwrap_or(false)
}

fn create_build_context(context_dir: &Path) -> Result<Vec<u8>, RuntimeError> {
    let rules = parse_dockerignore(context_dir);
    let mut archive = tar::Builder::new(Vec::new());

    fn walk_dir(
        dir: &Path,
        base: &Path,
        rules: &[(String, bool)],
        archive: &mut tar::Builder<Vec<u8>>,
    ) -> Result<(), RuntimeError> {
        let entries = std::fs::read_dir(dir).map_err(|e| {
            RuntimeError::Other(format!("Failed to read dir {}: {}", dir.display(), e))
        })?;

        for entry in entries {
            let entry =
                entry.map_err(|e| RuntimeError::Other(e.to_string()))?;
            let abs_path = entry.path();
            let rel_path = abs_path
                .strip_prefix(base)
                .unwrap_or(&abs_path)
                .to_string_lossy();

            if is_ignored(&rel_path, rules) {
                continue;
            }

            if abs_path.is_dir() {
                walk_dir(&abs_path, base, rules, archive)?;
            } else {
                archive
                    .append_path_with_name(&abs_path, &*rel_path)
                    .map_err(|e| {
                        RuntimeError::Other(format!(
                            "Failed to add {} to build context: {}",
                            rel_path, e
                        ))
                    })?;
            }
        }
        Ok(())
    }

    walk_dir(context_dir, context_dir, &rules, &mut archive)?;

    archive
        .into_inner()
        .map_err(|e| RuntimeError::Other(format!("Failed to finalize build context: {}", e)))
}

/// Polls a plugin's HTTP endpoint until it responds (2xx/3xx) or the timeout expires.
pub async fn wait_for_ready(
    port: u16,
    path: &str,
    timeout: std::time::Duration,
) -> Result<(), RuntimeError> {
    let url = format!("http://127.0.0.1:{}{}", port, path);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
        .map_err(|e| RuntimeError::Other(format!("HTTP client error: {}", e)))?;

    let deadline = tokio::time::Instant::now() + timeout;
    let mut interval = tokio::time::interval(std::time::Duration::from_millis(250));

    loop {
        interval.tick().await;

        if tokio::time::Instant::now() > deadline {
            log::warn!("Plugin readiness timeout after {:?} for {}", timeout, url);
            return Ok(());
        }

        match client.get(&url).send().await {
            Ok(resp) if resp.status().is_success() || resp.status().is_redirection() => {
                log::info!("Plugin ready at {}", url);
                return Ok(());
            }
            Ok(resp) => {
                log::debug!("Plugin not ready yet: {} → {}", url, resp.status());
            }
            Err(_) => {
                log::debug!("Plugin not ready yet: {} → connection refused", url);
            }
        }
    }
}
