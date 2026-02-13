pub mod approval;
pub mod docker;
pub mod filesystem;
mod middleware;
pub mod network;
pub mod process;
pub mod settings;
pub mod system;
mod theme;

use std::sync::Arc;

use axum::{extract::DefaultBodyLimit, middleware as axum_middleware, routing, Extension, Json, Router};
use tower_http::cors::{Any, CorsLayer};
use utoipa::openapi::security::{Http, HttpAuthScheme, SecurityScheme};
use utoipa::{Modify, OpenApi};

use crate::AppState;
use approval::ApprovalBridge;

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_auth",
                SecurityScheme::Http(Http::new(HttpAuthScheme::Bearer)),
            );
        }
    }
}

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Nexus Host API",
        description = "API available to Nexus plugins for interacting with the host system. All authenticated endpoints require a Bearer token (NEXUS_TOKEN).",
        version = "0.1.0",
        license(name = "MIT")
    ),
    paths(
        system::system_info,
        filesystem::read_file,
        filesystem::list_dir,
        filesystem::write_file,
        process::list_processes,
        docker::list_containers,
        docker::container_stats,
        network::proxy_request,
        settings::get_settings,
        settings::put_settings,
    ),
    components(schemas(
        system::SystemInfo,
        filesystem::FileContent,
        filesystem::DirEntry,
        filesystem::DirListing,
        filesystem::WriteRequest,
        process::ProcessInfo,
        docker::ContainerInfo,
        network::ProxyRequest,
        network::ProxyResponse,
    )),
    modifiers(&SecurityAddon),
    tags(
        (name = "system", description = "Host system information"),
        (name = "filesystem", description = "Read and write files on the host"),
        (name = "process", description = "List host processes"),
        (name = "docker", description = "Docker container inspection"),
        (name = "network", description = "Network proxy for external requests"),
        (name = "settings", description = "Per-plugin settings (scoped to authenticated plugin)")
    )
)]
pub struct ApiDoc;

pub async fn start_server(
    state: AppState,
    approvals: Arc<ApprovalBridge>,
) -> Result<(), Box<dyn std::error::Error>> {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let authenticated_routes = Router::new()
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
        // Plugin settings (scoped to authenticated plugin)
        .route(
            "/v1/settings",
            routing::get(settings::get_settings).put(settings::put_settings),
        )
        .layer(axum_middleware::from_fn_with_state(
            state.clone(),
            middleware::auth_middleware,
        ))
        .layer(Extension(approvals))
        // 5 MB request body limit for all authenticated routes
        .layer(DefaultBodyLimit::max(5 * 1024 * 1024));

    let app = Router::new()
        // Public routes (no auth required)
        .route("/api/v1/theme.css", routing::get(theme::theme_css))
        .route(
            "/api/v1/theme/fonts/{filename}",
            routing::get(theme::theme_font),
        )
        // OpenAPI spec
        .route("/api/openapi.json", routing::get(openapi_spec))
        // Authenticated routes
        .nest("/api", authenticated_routes)
        .layer(cors)
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:9600").await?;
    log::info!("Host API server listening on 127.0.0.1:9600");
    axum::serve(listener, app).await?;

    Ok(())
}

async fn openapi_spec() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}
