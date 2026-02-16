pub mod approval;
pub mod auth;
pub mod docker;
pub mod extensions;
pub mod filesystem;
pub mod mcp;
pub mod mcp_client;
pub mod mcp_server;
mod middleware;
pub mod nexus_mcp;
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
use rmcp::transport::streamable_http_server::tower::{
    StreamableHttpServerConfig, StreamableHttpService,
};
use tokio_util::sync::CancellationToken;
use tower_http::cors::{Any, CorsLayer};
use utoipa::openapi::security::{Http, HttpAuthScheme, SecurityScheme};
use utoipa::{Modify, OpenApi};

use crate::oauth;
use crate::ActiveTheme;
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
        filesystem::glob_files,
        filesystem::grep_files,
        filesystem::edit_file,
        process::list_processes,
        process::exec_command,
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
        filesystem::GlobResult,
        filesystem::GrepResult,
        filesystem::GrepFileMatch,
        filesystem::GrepLine,
        filesystem::EditRequest,
        process::ProcessInfo,
        process::ExecRequest,
        process::ExecResult,
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
    oauth_store: Arc<oauth::OAuthStore>,
    active_theme: ActiveTheme,
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
        .route("/v1/fs/glob", routing::get(filesystem::glob_files))
        .route("/v1/fs/grep", routing::get(filesystem::grep_files))
        .route("/v1/fs/edit", routing::post(filesystem::edit_file))
        // Process
        .route("/v1/process/list", routing::get(process::list_processes))
        .route("/v1/process/exec", routing::post(process::exec_command))
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

    // Native MCP server (streamable HTTP) — the primary gateway endpoint.
    // Clients connect via: http://127.0.0.1:9600/mcp
    let mcp_cancel = CancellationToken::new();
    let mcp_config = StreamableHttpServerConfig {
        stateful_mode: true,
        cancellation_token: mcp_cancel.clone(),
        ..Default::default()
    };

    let mcp_state_for_factory = state.clone();
    let mcp_approvals_for_factory = approvals.clone();
    let mcp_service = StreamableHttpService::new(
        move || {
            Ok(mcp_server::NexusMcpServer::new(
                mcp_state_for_factory.clone(),
                mcp_approvals_for_factory.clone(),
            ))
        },
        Arc::new(rmcp::transport::streamable_http_server::session::local::LocalSessionManager::default()),
        mcp_config,
    );

    // Wrap the MCP service as an axum route with gateway auth.
    // McpSessionStore remembers authenticated sessions so subsequent requests
    // (which may not carry the gateway token) are allowed through.
    let mcp_session_store = mcp::McpSessionStore::new();
    let mcp_native_routes = Router::new()
        .nest_service("/mcp", mcp_service)
        .layer(axum_middleware::from_fn_with_state(
            state.clone(),
            mcp::gateway_auth_middleware,
        ))
        .layer(Extension(sessions.clone()))
        .layer(Extension(oauth_store.clone()))
        .layer(Extension(mcp_session_store));

    // Token exchange route — public, plugins call this with their secret
    let auth_routes = Router::new()
        .route("/v1/auth/token", routing::post(auth::create_token))
        .layer(Extension(sessions.clone()));

    // OAuth 2.1 discovery + authorization endpoints (public, no auth required)
    let pending_auth = oauth::authorize::PendingAuthMap::new();
    let oauth_routes = Router::new()
        .route(
            "/.well-known/oauth-protected-resource",
            routing::get(oauth::metadata::protected_resource),
        )
        .route(
            "/.well-known/oauth-protected-resource/mcp",
            routing::get(oauth::metadata::protected_resource),
        )
        .route(
            "/.well-known/oauth-authorization-server",
            routing::get(oauth::metadata::authorization_server),
        )
        .route("/oauth/register", routing::post(oauth::registration::register_client))
        .route("/oauth/authorize", routing::get(oauth::authorize::authorize))
        .route("/oauth/authorize/poll/{state}", routing::get(oauth::authorize::authorize_poll))
        .route("/oauth/token", routing::post(oauth::token::token_exchange))
        .layer(Extension(oauth_store.clone()))
        .layer(Extension(approvals.clone()))
        .layer(Extension(pending_auth))
        .layer(Extension(active_theme.clone()));

    let theme_routes = Router::new()
        .route("/api/v1/theme.css", routing::get(theme::theme_css))
        .route("/api/v1/theme", routing::get(theme::theme_active))
        .route(
            "/api/v1/theme/fonts/{filename}",
            routing::get(theme::theme_font),
        )
        .layer(Extension(active_theme.clone()));

    let app = Router::new()
        // Public routes (no auth required) — theme CSS, fonts, and active theme query
        .merge(theme_routes)
        // OpenAPI spec
        .route("/api/openapi.json", routing::get(openapi_spec))
        // OAuth 2.1 endpoints (public — discovery, registration, authorization, token)
        .merge(oauth_routes)
        // Token exchange (public — plugins exchange secret for access token)
        .nest("/api", auth_routes)
        // Native MCP endpoint (streamable HTTP — primary connection mode)
        .merge(mcp_native_routes)
        // Authenticated routes (plugin Bearer token auth via session store)
        .nest("/api", authenticated_routes)
        .layer(cors)
        .layer(axum_middleware::from_fn(mcp::http_request_logging))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:9600").await?;
    log::info!("Host API server listening on 127.0.0.1:9600");
    axum::serve(listener, app).await?;

    Ok(())
}

async fn openapi_spec() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}
