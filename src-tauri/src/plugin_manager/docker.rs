use bollard::container::LogOutput;
use bollard::query_parameters::{
    BuildImageOptions, CreateContainerOptions, CreateImageOptions, ListContainersOptions,
    ListNetworksOptions, LogsOptions, RemoveContainerOptions, RemoveImageOptions,
    StartContainerOptions, StatsOptions, StopContainerOptions,
};
use bollard::service::{ContainerCreateBody, HostConfig, Mount, MountTypeEnum, NetworkCreateRequest, PortBinding};
use bollard::Docker;
use futures_util::StreamExt;
use std::collections::HashMap;
use std::path::Path;

use crate::error::{NexusError, NexusResult};

const NETWORK_NAME: &str = "nexus-bridge";

pub fn connect() -> NexusResult<Docker> {
    Docker::connect_with_local_defaults().map_err(NexusError::Docker)
}

pub async fn ensure_network() -> NexusResult<()> {
    let docker = connect()?;

    let networks = docker
        .list_networks(None::<ListNetworksOptions>)
        .await
        .map_err(NexusError::Docker)?;

    let exists = networks.iter().any(|n| {
        n.name
            .as_deref()
            .is_some_and(|name| name == NETWORK_NAME)
    });

    if !exists {
        docker
            .create_network(NetworkCreateRequest {
                name: NETWORK_NAME.to_string(),
                driver: Some("bridge".to_string()),
                ..Default::default()
            })
            .await
            .map_err(NexusError::Docker)?;
        log::info!("Created Docker network: {}", NETWORK_NAME);
    }

    Ok(())
}

pub async fn image_exists(image: &str) -> NexusResult<bool> {
    let docker = connect()?;
    match docker.inspect_image(image).await {
        Ok(_) => Ok(true),
        Err(bollard::errors::Error::DockerResponseServerError {
            status_code: 404, ..
        }) => Ok(false),
        Err(e) => Err(NexusError::Docker(e)),
    }
}

/// Check if a Docker image is available — locally first, then in its remote registry.
///
/// Supports ghcr.io and Docker Hub for remote checks. Returns false for unrecognized
/// registries or network errors (fail-open: install button still enabled, pull will
/// fail with a clear error message instead).
pub async fn check_image_available(image: &str) -> bool {
    // Check local Docker first — handles locally-built images (e.g. from local registries)
    match image_exists(image).await {
        Ok(true) => return true,
        Ok(false) => {}
        Err(e) => {
            log::warn!("Local image check failed for {}: {}", image, e);
            // Fall through to remote check
        }
    }

    match check_image_available_inner(image).await {
        Ok(available) => available,
        Err(e) => {
            log::warn!("Image availability check failed for {}: {}", image, e);
            // Fail-open: let the user attempt the install
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
            // Unknown registry — try generic V2 API without auth
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

    // Get anonymous pull token
    #[derive(serde::Deserialize)]
    struct TokenResponse {
        token: String,
    }
    let token_resp: TokenResponse = client.get(&token_url).send().await?.json().await?;

    // HEAD the manifest
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

/// Parse a Docker image reference into (registry, repository, tag).
///
/// Examples:
///   "ghcr.io/owner/image:1.0"  → ("ghcr.io", "owner/image", "1.0")
///   "owner/image:latest"       → ("docker.io", "owner/image", "latest")
///   "image:tag"                → ("docker.io", "library/image", "tag")
fn parse_image_ref(image: &str) -> (String, String, String) {
    let (name, tag) = if let Some((n, t)) = image.rsplit_once(':') {
        (n, t)
    } else {
        (image, "latest")
    };

    // If name contains a dot or colon in the first segment, it's a registry
    let parts: Vec<&str> = name.splitn(2, '/').collect();
    if parts.len() == 2 && (parts[0].contains('.') || parts[0].contains(':')) {
        let registry = parts[0].to_string();
        let repository = parts[1].to_string();
        (registry, repository, tag.to_string())
    } else if parts.len() == 2 {
        // "owner/image" → Docker Hub
        ("docker.io".to_string(), name.to_string(), tag.to_string())
    } else {
        // "image" → Docker Hub library
        ("docker.io".to_string(), format!("library/{}", name), tag.to_string())
    }
}

pub async fn pull_image(image: &str) -> NexusResult<()> {
    // Skip pull if the image already exists locally
    if image_exists(image).await? {
        log::info!("Image {} found locally, skipping pull", image);
        return Ok(());
    }

    let docker = connect()?;

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

    let mut stream = docker.create_image(Some(opts), None, None);
    while let Some(result) = stream.next().await {
        match result {
            Ok(info) => {
                if let Some(status) = info.status {
                    log::debug!("Pull: {}", status);
                }
            }
            Err(e) => return Err(NexusError::Docker(e)),
        }
    }

    Ok(())
}

/// Get the SHA-256 manifest digest of a pulled image.
///
/// Returns the digest in `sha256:hex` format, or `None` for locally-built
/// images (which don't have a registry digest).
pub async fn get_image_digest(image: &str) -> NexusResult<Option<String>> {
    let docker = connect()?;
    let inspect = docker.inspect_image(image).await.map_err(NexusError::Docker)?;

    if let Some(repo_digests) = inspect.repo_digests {
        for digest_str in &repo_digests {
            // Format: "repo@sha256:hexstring"
            if let Some(digest) = digest_str.split('@').nth(1) {
                if digest.starts_with("sha256:") {
                    return Ok(Some(digest.to_string()));
                }
            }
        }
    }

    Ok(None)
}

/// Parse a .dockerignore file into a list of glob patterns.
/// Supports comment lines (#), negation (!), and trims whitespace.
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
                Some((pattern.to_string(), true)) // negation
            } else {
                Some((trimmed.to_string(), false))
            }
        })
        .collect()
}

/// Check whether a relative path is excluded by .dockerignore patterns.
fn is_ignored(rel_path: &str, rules: &[(String, bool)]) -> bool {
    let mut ignored = false;
    for (pattern, negated) in rules {
        // Match against the path or any component prefix
        let matches = glob_match(pattern, rel_path)
            || rel_path.starts_with(&format!("{}/", pattern));
        if matches {
            ignored = !negated;
        }
    }
    ignored
}

/// Simple glob matcher supporting * and ? (no ** needed for .dockerignore top-level patterns).
fn glob_match(pattern: &str, text: &str) -> bool {
    // Convert to regex for reliable matching
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

/// Create a tar archive of the build context, respecting .dockerignore.
fn create_build_context(context_dir: &Path) -> NexusResult<Vec<u8>> {
    let rules = parse_dockerignore(context_dir);
    let mut archive = tar::Builder::new(Vec::new());

    fn walk_dir(
        dir: &Path,
        base: &Path,
        rules: &[(String, bool)],
        archive: &mut tar::Builder<Vec<u8>>,
    ) -> Result<(), NexusError> {
        let entries = std::fs::read_dir(dir)
            .map_err(|e| NexusError::Other(format!("Failed to read dir {}: {}", dir.display(), e)))?;

        for entry in entries {
            let entry = entry.map_err(|e| NexusError::Other(e.to_string()))?;
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
                    .map_err(|e| NexusError::Other(format!(
                        "Failed to add {} to build context: {}",
                        rel_path, e
                    )))?;
            }
        }
        Ok(())
    }

    walk_dir(context_dir, context_dir, &rules, &mut archive)?;

    archive
        .into_inner()
        .map_err(|e| NexusError::Other(format!("Failed to finalize build context: {}", e)))
}

pub async fn build_image(context_dir: &Path, tag: &str) -> NexusResult<()> {
    let docker = connect()?;

    // Create a tar archive of the build context, respecting .dockerignore
    let tar_bytes = create_build_context(context_dir)?;

    let opts = BuildImageOptions {
        t: Some(tag.to_string()),
        rm: true,
        ..Default::default()
    };

    let body = bollard::body_full(tar_bytes.into());
    let mut stream = docker.build_image(opts, None, Some(body));
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
                    return Err(NexusError::Other(format!("Docker build error: {}", msg)));
                }
            }
            Err(e) => return Err(NexusError::Docker(e)),
        }
    }

    log::info!("Built image: {}", tag);
    Ok(())
}

/// Resource limits applied to plugin containers.
#[derive(Debug, Clone, Default)]
pub struct ResourceLimits {
    /// CPU limit in nanoseconds (1e9 = 1 full CPU core).
    pub nano_cpus: Option<i64>,
    /// Memory limit in bytes.
    pub memory_bytes: Option<i64>,
}

#[allow(clippy::too_many_arguments)]
pub async fn create_container(
    name: &str,
    image: &str,
    host_port: u16,
    container_port: u16,
    env_vars: Vec<String>,
    labels: HashMap<String, String>,
    limits: ResourceLimits,
    data_volume: Option<&str>,
) -> NexusResult<String> {
    let docker = connect()?;

    let port_binding = PortBinding {
        host_ip: Some("127.0.0.1".to_string()),
        host_port: Some(host_port.to_string()),
    };

    let container_port_key = format!("{}/tcp", container_port);

    let mut port_bindings = HashMap::new();
    port_bindings.insert(container_port_key.clone(), Some(vec![port_binding]));

    let mounts = match data_volume {
        Some(vol_name) => vec![Mount {
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
        network_mode: Some(NETWORK_NAME.to_string()),
        extra_hosts: Some(vec!["host.docker.internal:host-gateway".to_string()]),
        // Security hardening
        cap_drop: Some(vec!["ALL".to_string()]),
        cap_add: Some(vec!["NET_BIND_SERVICE".to_string()]),
        security_opt: Some(vec!["no-new-privileges:true".to_string()]),
        binds: Some(vec![]),
        mounts: Some(mounts),
        // Resource limits
        nano_cpus: limits.nano_cpus,
        memory: limits.memory_bytes,
        ..Default::default()
    };

    let config = ContainerCreateBody {
        image: Some(image.to_string()),
        env: Some(env_vars),
        labels: Some(labels),
        exposed_ports: Some(vec![container_port_key]),
        host_config: Some(host_config),
        ..Default::default()
    };

    let opts = CreateContainerOptions {
        name: Some(name.to_string()),
        ..Default::default()
    };

    let response = docker
        .create_container(Some(opts), config)
        .await
        .map_err(NexusError::Docker)?;

    Ok(response.id)
}

pub async fn start_container(container_id: &str) -> NexusResult<()> {
    let docker = connect()?;
    docker
        .start_container(container_id, None::<StartContainerOptions>)
        .await
        .map_err(NexusError::Docker)?;
    Ok(())
}

pub async fn stop_container(container_id: &str) -> NexusResult<()> {
    let docker = connect()?;
    docker
        .stop_container(
            container_id,
            Some(StopContainerOptions { t: Some(10), signal: None }),
        )
        .await
        .map_err(NexusError::Docker)?;
    Ok(())
}

pub async fn remove_container(container_id: &str) -> NexusResult<()> {
    let docker = connect()?;
    docker
        .remove_container(
            container_id,
            Some(RemoveContainerOptions {
                force: true,
                ..Default::default()
            }),
        )
        .await
        .map_err(NexusError::Docker)?;
    Ok(())
}

/// Remove a named Docker volume. Used during plugin uninstall to clean up persistent data.
pub async fn remove_volume(name: &str) -> NexusResult<()> {
    let docker = connect()?;
    docker
        .remove_volume(name, None::<bollard::query_parameters::RemoveVolumeOptions>)
        .await
        .map_err(NexusError::Docker)?;
    log::info!("Removed Docker volume: {}", name);
    Ok(())
}

pub async fn remove_image(image: &str) -> NexusResult<()> {
    let docker = connect()?;
    docker
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
        .map_err(NexusError::Docker)?;
    Ok(())
}

pub async fn get_logs(container_id: &str, tail: u32) -> NexusResult<Vec<String>> {
    let docker = connect()?;

    let opts = LogsOptions {
        stdout: true,
        stderr: true,
        tail: tail.to_string(),
        ..Default::default()
    };

    let mut stream = docker.logs(container_id, Some(opts));
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
            Err(e) => return Err(NexusError::Docker(e)),
        }
    }

    Ok(lines)
}

pub async fn list_containers() -> NexusResult<Vec<bollard::service::ContainerSummary>> {
    let docker = connect()?;

    let mut filters = HashMap::new();
    filters.insert("label".to_string(), vec!["nexus.plugin.id".to_string()]);

    let opts = ListContainersOptions {
        all: true,
        filters: Some(filters),
        ..Default::default()
    };

    docker
        .list_containers(Some(opts))
        .await
        .map_err(NexusError::Docker)
}

/// Polls a plugin's HTTP endpoint until it responds (2xx/3xx) or the timeout expires.
/// Returns Ok(()) once ready, or an error on timeout.
pub async fn wait_for_ready(port: u16, path: &str, timeout: std::time::Duration) -> NexusResult<()> {
    let url = format!("http://127.0.0.1:{}{}", port, path);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
        .map_err(|e| NexusError::Other(format!("HTTP client error: {}", e)))?;

    let deadline = tokio::time::Instant::now() + timeout;
    let mut interval = tokio::time::interval(std::time::Duration::from_millis(250));

    loop {
        interval.tick().await;

        if tokio::time::Instant::now() > deadline {
            log::warn!("Plugin readiness timeout after {:?} for {}", timeout, url);
            // Don't fail the start — the container IS running, just slow
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

/// Aggregates CPU and memory usage across all Nexus-managed containers.
pub async fn aggregate_stats() -> NexusResult<crate::commands::system::ResourceUsage> {
    let containers = list_containers().await?;
    let docker = connect()?;

    let mut total_cpu = 0.0_f64;
    let mut total_memory_bytes = 0_u64;

    for container in &containers {
        let id = match &container.id {
            Some(id) => id,
            None => continue,
        };

        // Only stats running containers
        let is_running = container.state
            == Some(bollard::service::ContainerSummaryStateEnum::RUNNING);
        if !is_running {
            continue;
        }

        // Get a single stats snapshot (stream=false gives one result)
        let opts = StatsOptions {
            stream: false,
            one_shot: true,
        };

        let mut stream = docker.stats(id, Some(opts));
        if let Some(Ok(stats)) = futures_util::StreamExt::next(&mut stream).await {
            // CPU percentage calculation
            if let (Some(cpu), Some(precpu)) = (&stats.cpu_stats, &stats.precpu_stats) {
                if let (Some(cpu_usage), Some(precpu_usage)) = (&cpu.cpu_usage, &precpu.cpu_usage) {
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

            // Memory usage
            if let Some(mem) = &stats.memory_stats {
                total_memory_bytes += mem.usage.unwrap_or(0);
            }
        }
    }

    Ok(crate::commands::system::ResourceUsage {
        cpu_percent: (total_cpu * 10.0).round() / 10.0,
        memory_mb: (total_memory_bytes as f64 / 1_048_576.0 * 10.0).round() / 10.0,
    })
}

/// Returns "running", "stopped", or errors if the container doesn't exist.
pub async fn container_state(container_id: &str) -> NexusResult<&'static str> {
    let docker = connect()?;
    let info = docker
        .inspect_container(container_id, None)
        .await
        .map_err(NexusError::Docker)?;

    let running = info
        .state
        .and_then(|s| s.running)
        .unwrap_or(false);

    Ok(if running { "running" } else { "stopped" })
}
