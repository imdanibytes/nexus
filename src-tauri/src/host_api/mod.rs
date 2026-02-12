mod docker;
mod filesystem;
mod middleware;
mod network;
mod process;
mod system;

use axum::{middleware as axum_middleware, routing, Router};
use tower_http::cors::{Any, CorsLayer};

use crate::AppState;

pub async fn start_server(state: AppState) -> Result<(), Box<dyn std::error::Error>> {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let api_routes = Router::new()
        // System
        .route("/v1/system/info", routing::get(system::system_info))
        // Filesystem
        .route("/v1/fs/read", routing::get(filesystem::read_file))
        .route("/v1/fs/list", routing::get(filesystem::list_dir))
        .route("/v1/fs/write", routing::post(filesystem::write_file))
        // Process
        .route("/v1/process/list", routing::get(process::list_processes))
        // Docker
        .route(
            "/v1/docker/containers",
            routing::get(docker::list_containers),
        )
        .route(
            "/v1/docker/stats/{id}",
            routing::get(docker::container_stats),
        )
        // Network
        .route("/v1/network/proxy", routing::post(network::proxy_request))
        .layer(axum_middleware::from_fn_with_state(
            state.clone(),
            middleware::auth_middleware,
        ));

    let app = Router::new()
        .nest("/api", api_routes)
        .layer(cors)
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:9600").await?;
    log::info!("Host API server listening on :9600");
    axum::serve(listener, app).await?;

    Ok(())
}
