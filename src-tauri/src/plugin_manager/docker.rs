use bollard::container::{
    Config, CreateContainerOptions, ListContainersOptions, LogOutput, LogsOptions,
    RemoveContainerOptions, StopContainerOptions,
};
use bollard::image::CreateImageOptions;
use bollard::network::CreateNetworkOptions;
use bollard::Docker;
use futures_util::StreamExt;
use std::collections::HashMap;

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

pub async fn create_container(
    name: &str,
    image: &str,
    host_port: u16,
    container_port: u16,
    env_vars: Vec<String>,
    labels: HashMap<String, String>,
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

pub async fn container_running(container_id: &str) -> NexusResult<bool> {
    let docker = connect()?;
    let info = docker
        .inspect_container(container_id, None)
        .await
        .map_err(NexusError::Docker)?;

    Ok(info
        .state
        .and_then(|s| s.running)
        .unwrap_or(false))
}
