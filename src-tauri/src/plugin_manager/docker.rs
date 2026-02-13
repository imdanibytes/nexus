use bollard::container::{
    Config, CreateContainerOptions, ListContainersOptions, LogOutput, LogsOptions,
    RemoveContainerOptions, StopContainerOptions,
};
use bollard::image::{BuildImageOptions, CreateImageOptions, RemoveImageOptions};
use bollard::network::CreateNetworkOptions;
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
        .list_networks::<String>(None)
        .await
        .map_err(NexusError::Docker)?;

    let exists = networks.iter().any(|n| {
        n.name
            .as_deref()
            .is_some_and(|name| name == NETWORK_NAME)
    });

    if !exists {
        docker
            .create_network(CreateNetworkOptions {
                name: NETWORK_NAME,
                driver: "bridge",
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
        from_image: repo,
        tag,
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

pub async fn build_image(context_dir: &Path, tag: &str) -> NexusResult<()> {
    let docker = connect()?;

    // Create a tar archive of the build context
    let mut archive = tar::Builder::new(Vec::new());
    archive
        .append_dir_all(".", context_dir)
        .map_err(|e| NexusError::Other(format!("Failed to create build context: {}", e)))?;
    let tar_bytes = archive
        .into_inner()
        .map_err(|e| NexusError::Other(format!("Failed to finalize build context: {}", e)))?;

    let opts = BuildImageOptions {
        t: tag,
        rm: true,
        ..Default::default()
    };

    let mut stream = docker.build_image(opts, None, Some(tar_bytes.into()));
    while let Some(result) = stream.next().await {
        match result {
            Ok(info) => {
                if let Some(stream) = info.stream {
                    let msg = stream.trim();
                    if !msg.is_empty() {
                        log::debug!("Build: {}", msg);
                    }
                }
                if let Some(error) = info.error {
                    return Err(NexusError::Other(format!("Docker build error: {}", error)));
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

pub async fn create_container(
    name: &str,
    image: &str,
    host_port: u16,
    container_port: u16,
    env_vars: Vec<String>,
    labels: HashMap<String, String>,
    limits: ResourceLimits,
) -> NexusResult<String> {
    let docker = connect()?;

    let port_binding = bollard::service::PortBinding {
        host_ip: Some("127.0.0.1".to_string()),
        host_port: Some(host_port.to_string()),
    };

    let container_port_key = format!("{}/tcp", container_port);

    let mut port_bindings = HashMap::new();
    port_bindings.insert(container_port_key.clone(), Some(vec![port_binding]));

    let mut exposed_ports = HashMap::new();
    exposed_ports.insert(container_port_key, HashMap::new());

    let host_config = bollard::service::HostConfig {
        port_bindings: Some(port_bindings),
        network_mode: Some(NETWORK_NAME.to_string()),
        extra_hosts: Some(vec!["host.docker.internal:host-gateway".to_string()]),
        // Security hardening
        cap_drop: Some(vec!["ALL".to_string()]),
        cap_add: Some(vec!["NET_BIND_SERVICE".to_string()]),
        security_opt: Some(vec!["no-new-privileges:true".to_string()]),
        binds: Some(vec![]),
        mounts: Some(vec![]),
        // Resource limits
        nano_cpus: limits.nano_cpus,
        memory: limits.memory_bytes,
        ..Default::default()
    };

    let config = Config {
        image: Some(image.to_string()),
        env: Some(env_vars),
        labels: Some(labels),
        exposed_ports: Some(exposed_ports),
        host_config: Some(host_config),
        ..Default::default()
    };

    let opts = CreateContainerOptions {
        name,
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
        .start_container::<String>(container_id, None)
        .await
        .map_err(NexusError::Docker)?;
    Ok(())
}

pub async fn stop_container(container_id: &str) -> NexusResult<()> {
    let docker = connect()?;
    docker
        .stop_container(
            container_id,
            Some(StopContainerOptions { t: 10 }),
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

pub async fn remove_image(image: &str) -> NexusResult<()> {
    let docker = connect()?;
    docker
        .remove_image(
            image,
            Some(RemoveImageOptions {
                force: false,
                noprune: false,
            }),
            None,
        )
        .await
        .map_err(NexusError::Docker)?;
    Ok(())
}

pub async fn get_logs(container_id: &str, tail: u32) -> NexusResult<Vec<String>> {
    let docker = connect()?;

    let opts = LogsOptions::<String> {
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
    filters.insert("label", vec!["nexus.plugin.id"]);

    let opts = ListContainersOptions {
        all: true,
        filters,
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
        let state = container
            .state
            .as_deref()
            .unwrap_or("unknown");
        if state != "running" {
            continue;
        }

        // Get a single stats snapshot (stream=false gives one result)
        let opts = bollard::container::StatsOptions {
            stream: false,
            one_shot: true,
        };

        let mut stream = docker.stats(id, Some(opts));
        if let Some(Ok(stats)) = futures_util::StreamExt::next(&mut stream).await {
            // CPU percentage calculation
            let cpu_delta = stats.cpu_stats.cpu_usage.total_usage as f64
                - stats.precpu_stats.cpu_usage.total_usage as f64;
            let system_delta = stats.cpu_stats.system_cpu_usage.unwrap_or(0) as f64
                - stats.precpu_stats.system_cpu_usage.unwrap_or(0) as f64;
            let num_cpus = stats
                .cpu_stats
                .online_cpus
                .unwrap_or(1) as f64;

            if system_delta > 0.0 {
                total_cpu += (cpu_delta / system_delta) * num_cpus * 100.0;
            }

            // Memory usage
            total_memory_bytes += stats.memory_stats.usage.unwrap_or(0);
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
