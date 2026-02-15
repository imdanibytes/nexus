pub mod approval;
pub mod auth;
pub mod docker;
pub mod extensions;
pub mod filesystem;
pub mod mcp;
mod middleware;
pub mod network;
pub mod process;
mod rate_limit;
pub mod settings;
pub mod storage;
pub mod system;
mod theme;

use std::sync::Arc;
use std::time::Duration;

use axum::{extract::DefaultBodyLimit, middleware as axum_middleware, routing, Extension, Json, Router};
use tower_http::cors::{Any, CorsLayer};
use utoipa::openapi::security::{Http, HttpAuthScheme, SecurityScheme};
use utoipa::{Modify, OpenApi};

use crate::AppState;
use approval::ApprovalBridge;
use auth::SessionStore;

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
        description = "API available to Nexus plugins for interacting with the host system. Plugins exchange their secret (NEXUS_PLUGIN_SECRET) for a short-lived access token via POST /v1/auth/token, then use it as a Bearer token.",
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

    // 100 requests per second per plugin — generous for normal use, blocks abuse
    let limiter = rate_limit::RateLimiter::new(100, std::time::Duration::from_secs(1));

    // Session store for short-lived access tokens (15 minute TTL)
    let sessions = Arc::new(SessionStore::new(Duration::from_secs(15 * 60)));

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
        // Extensions
        .route(
            "/v1/extensions",
            routing::get(extensions::list_extensions),
        )
        .route(
            "/v1/extensions/{ext_id}/{operation}",
            routing::post(extensions::call_extension),
        )
        // Plugin settings (scoped to authenticated plugin)
        .route(
            "/v1/settings",
            routing::get(settings::get_settings).put(settings::put_settings),
        )
        // Plugin key-value storage (scoped to authenticated plugin)
        .route("/v1/storage", routing::get(storage::list_keys))
        .route(
            "/v1/storage/{key}",
            routing::get(storage::get_value)
                .put(storage::put_value)
                .delete(storage::delete_value),
        )
        // Rate limiting runs after auth (needs plugin identity)
        .layer(axum_middleware::from_fn(rate_limit::rate_limit_middleware))
        .layer(Extension(limiter))
        .layer(axum_middleware::from_fn_with_state(
            state.clone(),
            middleware::auth_middleware,
        ))
        .layer(Extension(sessions.clone()))
        .layer(Extension(approvals.clone()))
        // 5 MB request body limit for all authenticated routes
        .layer(DefaultBodyLimit::max(5 * 1024 * 1024));

    // MCP gateway routes — accepts X-Nexus-Gateway-Token (sidecar) OR
    // Bearer token with mcp:call permission (plugins)
    let mcp_routes = Router::new()
        .route("/v1/mcp/tools", routing::get(mcp::list_tools))
        .route("/v1/mcp/call", routing::post(mcp::call_tool))
        .route("/v1/mcp/events", routing::get(mcp::tool_events))
        .layer(axum_middleware::from_fn_with_state(
            state.clone(),
            mcp::gateway_auth_middleware,
        ))
        .layer(Extension(sessions.clone()))
        .layer(Extension(approvals.clone()));

    // Token exchange route — public, plugins call this with their secret
    let auth_routes = Router::new()
        .route("/v1/auth/token", routing::post(auth::create_token))
        .layer(Extension(sessions.clone()));

    let app = Router::new()
        // Public routes (no auth required)
        .route("/api/v1/theme.css", routing::get(theme::theme_css))
        .route(
            "/api/v1/theme/fonts/{filename}",
            routing::get(theme::theme_font),
        )
        // OpenAPI spec
        .route("/api/openapi.json", routing::get(openapi_spec))
        // Token exchange (public — plugins exchange secret for access token)
        .nest("/api", auth_routes)
        // MCP gateway routes (gateway token auth)
        .nest("/api", mcp_routes)
        // Authenticated routes (plugin Bearer token auth via session store)
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
