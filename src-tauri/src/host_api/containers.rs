use axum::{extract::Path, extract::Query, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::runtime::{ContainerFilters, RuntimeError};

// ---------------------------------------------------------------------------
// Response types (OpenAPI schemas)
// ---------------------------------------------------------------------------

#[derive(Serialize, ToSchema)]
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

#[derive(Serialize, ToSchema)]
pub struct ImageInfo {
    pub id: String,
    pub repo_tags: Vec<String>,
    pub size: i64,
    pub created: i64,
}

impl From<crate::runtime::ImageInfo> for ImageInfo {
    fn from(i: crate::runtime::ImageInfo) -> Self {
        Self {
            id: i.id,
            repo_tags: i.repo_tags,
            size: i.size,
            created: i.created,
        }
    }
}

#[derive(Serialize, ToSchema)]
pub struct VolumeInfo {
    pub name: String,
    pub driver: String,
    pub mountpoint: String,
    pub created_at: Option<String>,
}

impl From<crate::runtime::VolumeInfo> for VolumeInfo {
    fn from(v: crate::runtime::VolumeInfo) -> Self {
        Self {
            name: v.name,
            driver: v.driver,
            mountpoint: v.mountpoint,
            created_at: v.created_at,
        }
    }
}

#[derive(Serialize, ToSchema)]
pub struct NetworkInfo {
    pub id: String,
    pub name: String,
    pub driver: String,
    pub scope: String,
}

impl From<crate::runtime::NetworkInfo> for NetworkInfo {
    fn from(n: crate::runtime::NetworkInfo) -> Self {
        Self {
            id: n.id,
            name: n.name,
            driver: n.driver,
            scope: n.scope,
        }
    }
}

#[derive(Serialize, ToSchema)]
pub struct EngineInfo {
    pub engine_id: String,
    pub version: Option<String>,
    pub os: Option<String>,
    pub arch: Option<String>,
    pub cpus: Option<i64>,
    pub memory_bytes: Option<i64>,
}

impl From<crate::runtime::EngineInfo> for EngineInfo {
    fn from(e: crate::runtime::EngineInfo) -> Self {
        Self {
            engine_id: e.engine_id,
            version: e.version,
            os: e.os,
            arch: e.arch,
            cpus: e.cpus,
            memory_bytes: e.memory_bytes,
        }
    }
}

#[derive(Serialize, ToSchema)]
pub struct ContainerLogs {
    pub lines: Vec<String>,
}

#[derive(Serialize, ToSchema)]
pub struct StatusMessage {
    pub status: String,
}

// ---------------------------------------------------------------------------
// Query parameter types
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct LogsQuery {
    pub tail: Option<u32>,
}

#[derive(Deserialize)]
pub struct StopQuery {
    pub timeout: Option<i64>,
}

#[derive(Deserialize)]
pub struct ForceQuery {
    pub force: Option<bool>,
}

// ---------------------------------------------------------------------------
// Error mapping
// ---------------------------------------------------------------------------

fn runtime_err(e: RuntimeError) -> StatusCode {
    match e {
        RuntimeError::NotFound(_) => StatusCode::NOT_FOUND,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

// ---------------------------------------------------------------------------
// Container endpoints
// ---------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/api/v1/containers",
    tag = "containers",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "All containers", body = Vec<ContainerInfo>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    )
)]
pub async fn list_all_containers(
    axum::extract::State(state): axum::extract::State<crate::AppState>,
) -> Result<Json<Vec<ContainerInfo>>, StatusCode> {
    let runtime = { state.read().await.runtime.clone() };
    let containers = runtime
        .list_containers(ContainerFilters::default())
        .await
        .map_err(runtime_err)?;
    Ok(Json(containers.into_iter().map(ContainerInfo::from).collect()))
}

#[utoipa::path(
    get,
    path = "/api/v1/containers/{id}",
    tag = "containers",
    security(("bearer_auth" = [])),
    params(("id" = String, Path, description = "Container ID or name")),
    responses(
        (status = 200, description = "Container inspect", body = Object),
        (status = 404, description = "Container not found"),
    )
)]
pub async fn inspect_container(
    axum::extract::State(state): axum::extract::State<crate::AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let runtime = { state.read().await.runtime.clone() };
    let info = runtime
        .inspect_container_raw(&id)
        .await
        .map_err(runtime_err)?;
    Ok(Json(info))
}

#[utoipa::path(
    get,
    path = "/api/v1/containers/{id}/logs",
    tag = "containers",
    security(("bearer_auth" = [])),
    params(
        ("id" = String, Path, description = "Container ID or name"),
        ("tail" = Option<u32>, Query, description = "Number of lines to return (default: 100)")
    ),
    responses(
        (status = 200, description = "Container logs", body = ContainerLogs),
        (status = 404, description = "Container not found"),
    )
)]
pub async fn container_logs(
    axum::extract::State(state): axum::extract::State<crate::AppState>,
    Path(id): Path<String>,
    Query(query): Query<LogsQuery>,
) -> Result<Json<ContainerLogs>, StatusCode> {
    let runtime = { state.read().await.runtime.clone() };
    let tail = query.tail.unwrap_or(100);
    let lines = runtime.get_logs(&id, tail).await.map_err(runtime_err)?;
    Ok(Json(ContainerLogs { lines }))
}

#[utoipa::path(
    get,
    path = "/api/v1/containers/{id}/stats",
    tag = "containers",
    security(("bearer_auth" = [])),
    params(("id" = String, Path, description = "Container ID or name")),
    responses(
        (status = 200, description = "One-shot container stats", body = Object),
        (status = 404, description = "Container not found"),
    )
)]
pub async fn container_stats(
    axum::extract::State(state): axum::extract::State<crate::AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let runtime = { state.read().await.runtime.clone() };
    let stats = runtime
        .inspect_container_raw(&id)
        .await
        .map_err(runtime_err)?;
    Ok(Json(stats))
}

#[utoipa::path(
    post,
    path = "/api/v1/containers/{id}/start",
    tag = "containers",
    security(("bearer_auth" = [])),
    params(("id" = String, Path, description = "Container ID or name")),
    responses(
        (status = 200, description = "Container started", body = StatusMessage),
        (status = 404, description = "Container not found"),
    )
)]
pub async fn start_container(
    axum::extract::State(state): axum::extract::State<crate::AppState>,
    Path(id): Path<String>,
) -> Result<Json<StatusMessage>, StatusCode> {
    let runtime = { state.read().await.runtime.clone() };
    runtime.start_container(&id).await.map_err(runtime_err)?;
    Ok(Json(StatusMessage { status: "started".to_string() }))
}

#[utoipa::path(
    post,
    path = "/api/v1/containers/{id}/stop",
    tag = "containers",
    security(("bearer_auth" = [])),
    params(("id" = String, Path, description = "Container ID or name")),
    responses(
        (status = 200, description = "Container stopped", body = StatusMessage),
        (status = 404, description = "Container not found"),
    )
)]
pub async fn stop_container(
    axum::extract::State(state): axum::extract::State<crate::AppState>,
    Path(id): Path<String>,
) -> Result<Json<StatusMessage>, StatusCode> {
    let runtime = { state.read().await.runtime.clone() };
    runtime.stop_container(&id).await.map_err(runtime_err)?;
    Ok(Json(StatusMessage { status: "stopped".to_string() }))
}

#[utoipa::path(
    post,
    path = "/api/v1/containers/{id}/restart",
    tag = "containers",
    security(("bearer_auth" = [])),
    params(("id" = String, Path, description = "Container ID or name")),
    responses(
        (status = 200, description = "Container restarted", body = StatusMessage),
        (status = 404, description = "Container not found"),
    )
)]
pub async fn restart_container(
    axum::extract::State(state): axum::extract::State<crate::AppState>,
    Path(id): Path<String>,
) -> Result<Json<StatusMessage>, StatusCode> {
    let runtime = { state.read().await.runtime.clone() };
    runtime.restart_container(&id).await.map_err(runtime_err)?;
    Ok(Json(StatusMessage { status: "restarted".to_string() }))
}

#[utoipa::path(
    delete,
    path = "/api/v1/containers/{id}",
    tag = "containers",
    security(("bearer_auth" = [])),
    params(
        ("id" = String, Path, description = "Container ID or name"),
        ("force" = Option<bool>, Query, description = "Force remove (default: false)")
    ),
    responses(
        (status = 200, description = "Container removed", body = StatusMessage),
        (status = 404, description = "Container not found"),
    )
)]
pub async fn remove_container(
    axum::extract::State(state): axum::extract::State<crate::AppState>,
    Path(id): Path<String>,
) -> Result<Json<StatusMessage>, StatusCode> {
    let runtime = { state.read().await.runtime.clone() };
    runtime.remove_container(&id).await.map_err(runtime_err)?;
    Ok(Json(StatusMessage { status: "removed".to_string() }))
}

// ---------------------------------------------------------------------------
// Image endpoints
// ---------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/api/v1/containers/images",
    tag = "containers",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "All images", body = Vec<ImageInfo>),
    )
)]
pub async fn list_images(
    axum::extract::State(state): axum::extract::State<crate::AppState>,
) -> Result<Json<Vec<ImageInfo>>, StatusCode> {
    let runtime = { state.read().await.runtime.clone() };
    let images = runtime.list_images().await.map_err(runtime_err)?;
    Ok(Json(images.into_iter().map(ImageInfo::from).collect()))
}

#[utoipa::path(
    get,
    path = "/api/v1/containers/images/{id}",
    tag = "containers",
    security(("bearer_auth" = [])),
    params(("id" = String, Path, description = "Image ID or name:tag")),
    responses(
        (status = 200, description = "Image inspect", body = Object),
        (status = 404, description = "Image not found"),
    )
)]
pub async fn inspect_image(
    axum::extract::State(state): axum::extract::State<crate::AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let runtime = { state.read().await.runtime.clone() };
    let info = runtime
        .inspect_image_raw(&id)
        .await
        .map_err(runtime_err)?;
    Ok(Json(info))
}

#[utoipa::path(
    delete,
    path = "/api/v1/containers/images/{id}",
    tag = "containers",
    security(("bearer_auth" = [])),
    params(
        ("id" = String, Path, description = "Image ID or name:tag"),
        ("force" = Option<bool>, Query, description = "Force remove (default: false)")
    ),
    responses(
        (status = 200, description = "Image removed", body = StatusMessage),
        (status = 404, description = "Image not found"),
    )
)]
pub async fn remove_image(
    axum::extract::State(state): axum::extract::State<crate::AppState>,
    Path(id): Path<String>,
) -> Result<Json<StatusMessage>, StatusCode> {
    let runtime = { state.read().await.runtime.clone() };
    runtime.remove_image(&id).await.map_err(runtime_err)?;
    Ok(Json(StatusMessage { status: "removed".to_string() }))
}

// ---------------------------------------------------------------------------
// Volume endpoints
// ---------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/api/v1/containers/volumes",
    tag = "containers",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "All volumes", body = Vec<VolumeInfo>),
    )
)]
pub async fn list_volumes(
    axum::extract::State(state): axum::extract::State<crate::AppState>,
) -> Result<Json<Vec<VolumeInfo>>, StatusCode> {
    let runtime = { state.read().await.runtime.clone() };
    let volumes = runtime.list_volumes().await.map_err(runtime_err)?;
    Ok(Json(volumes.into_iter().map(VolumeInfo::from).collect()))
}

#[utoipa::path(
    delete,
    path = "/api/v1/containers/volumes/{name}",
    tag = "containers",
    security(("bearer_auth" = [])),
    params(("name" = String, Path, description = "Volume name")),
    responses(
        (status = 200, description = "Volume removed", body = StatusMessage),
        (status = 404, description = "Volume not found"),
    )
)]
pub async fn remove_volume(
    axum::extract::State(state): axum::extract::State<crate::AppState>,
    Path(name): Path<String>,
) -> Result<Json<StatusMessage>, StatusCode> {
    let runtime = { state.read().await.runtime.clone() };
    runtime.remove_volume(&name).await.map_err(runtime_err)?;
    Ok(Json(StatusMessage { status: "removed".to_string() }))
}

// ---------------------------------------------------------------------------
// Network endpoints
// ---------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/api/v1/containers/networks",
    tag = "containers",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "All networks", body = Vec<NetworkInfo>),
    )
)]
pub async fn list_networks(
    axum::extract::State(state): axum::extract::State<crate::AppState>,
) -> Result<Json<Vec<NetworkInfo>>, StatusCode> {
    let runtime = { state.read().await.runtime.clone() };
    let networks = runtime.list_networks().await.map_err(runtime_err)?;
    Ok(Json(networks.into_iter().map(NetworkInfo::from).collect()))
}

#[utoipa::path(
    delete,
    path = "/api/v1/containers/networks/{id}",
    tag = "containers",
    security(("bearer_auth" = [])),
    params(("id" = String, Path, description = "Network ID")),
    responses(
        (status = 200, description = "Network removed", body = StatusMessage),
        (status = 404, description = "Network not found"),
    )
)]
pub async fn remove_network(
    axum::extract::State(state): axum::extract::State<crate::AppState>,
    Path(id): Path<String>,
) -> Result<Json<StatusMessage>, StatusCode> {
    let runtime = { state.read().await.runtime.clone() };
    runtime.remove_network(&id).await.map_err(runtime_err)?;
    Ok(Json(StatusMessage { status: "removed".to_string() }))
}

// ---------------------------------------------------------------------------
// Engine endpoint
// ---------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/api/v1/containers/engine",
    tag = "containers",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Container engine info", body = EngineInfo),
    )
)]
pub async fn engine_info(
    axum::extract::State(state): axum::extract::State<crate::AppState>,
) -> Result<Json<EngineInfo>, StatusCode> {
    let runtime = { state.read().await.runtime.clone() };
    let info = runtime.engine_info().await.map_err(runtime_err)?;
    Ok(Json(EngineInfo::from(info)))
}
