use axum::{extract::Path, http::StatusCode, Json};
use serde::Serialize;

use crate::plugin_manager::docker as docker_client;

#[derive(Serialize)]
pub struct ContainerInfo {
    pub id: String,
    pub names: Vec<String>,
    pub image: String,
    pub state: String,
    pub status: String,
}

pub async fn list_containers() -> Result<Json<Vec<ContainerInfo>>, StatusCode> {
    let containers = docker_client::list_containers()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let infos: Vec<ContainerInfo> = containers
        .into_iter()
        .map(|c| ContainerInfo {
            id: c.id.unwrap_or_default(),
            names: c.names.unwrap_or_default(),
            image: c.image.unwrap_or_default(),
            state: c.state.unwrap_or_default(),
            status: c.status.unwrap_or_default(),
        })
        .collect();

    Ok(Json(infos))
}

pub async fn container_stats(
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let docker =
        docker_client::connect().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let stats = docker
        .inspect_container(&id, None)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    serde_json::to_value(stats)
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}
