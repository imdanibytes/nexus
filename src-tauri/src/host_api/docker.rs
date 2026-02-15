use axum::{extract::Path, http::StatusCode, Json};
use utoipa::ToSchema;

use crate::runtime::ContainerFilters;

// Re-export the runtime ContainerInfo with the ToSchema derive for OpenAPI docs.
#[derive(serde::Serialize, ToSchema)]
pub struct ContainerInfo {
    pub id: String,
    pub names: Vec<String>,
    pub image: String,
    pub state: String,
    pub status: String,
}

impl From<crate::runtime::ContainerInfo> for ContainerInfo {
    fn from(c: crate::runtime::ContainerInfo) -> Self {
        Self {
            id: c.id,
            names: c.names,
            image: c.image,
            state: c.state,
            status: c.status,
        }
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/docker/containers",
    tag = "docker",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Docker containers", body = Vec<ContainerInfo>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    )
)]
pub async fn list_containers(
    axum::extract::State(state): axum::extract::State<crate::AppState>,
) -> Result<Json<Vec<ContainerInfo>>, StatusCode> {
    let runtime = { state.read().await.runtime.clone() };

    let mut filters = ContainerFilters::default();
    filters
        .labels
        .insert("nexus.plugin.id".to_string(), String::new());

    let containers = runtime
        .list_containers(filters)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let infos: Vec<ContainerInfo> = containers.into_iter().map(ContainerInfo::from).collect();
    Ok(Json(infos))
}

#[utoipa::path(
    get,
    path = "/api/v1/docker/stats/{id}",
    tag = "docker",
    security(("bearer_auth" = [])),
    params(
        ("id" = String, Path, description = "Container ID")
    ),
    responses(
        (status = 200, description = "Container stats (Docker inspect)", body = Object),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    )
)]
pub async fn container_stats(
    axum::extract::State(state): axum::extract::State<crate::AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let runtime = { state.read().await.runtime.clone() };

    // Return FORBIDDEN instead of NOT_FOUND to prevent container ID enumeration
    let stats = runtime
        .inspect_container_raw(&id)
        .await
        .map_err(|_| StatusCode::FORBIDDEN)?;

    Ok(Json(stats))
}
