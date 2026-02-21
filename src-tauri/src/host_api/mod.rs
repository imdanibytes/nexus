pub mod approval;
pub mod containers;
pub mod events;
pub mod extensions;
pub mod filesystem;
pub mod mcp;
pub mod meta;
mod middleware;
pub mod network;
pub mod process;
mod rate_limit;
pub mod settings;
pub mod storage;
pub mod system;
mod theme;

use std::net::SocketAddr;
use std::sync::Arc;

use axum::{extract::DefaultBodyLimit, middleware as axum_middleware, routing, Extension, Json, Router};
use rmcp::transport::streamable_http_server::tower::{
    StreamableHttpServerConfig, StreamableHttpService,
};
use tokio_util::sync::CancellationToken;
use tower_http::cors::{AllowOrigin, Any, CorsLayer};
use utoipa::openapi::security::{Http, HttpAuthScheme, SecurityScheme};
use utoipa::{Modify, OpenApi};

use crate::api_keys::ApiKeyStore;
use crate::audit::writer::AuditWriter;
use crate::event_bus;
use crate::oauth;
use crate::ActiveTheme;
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
        description = "API available to Nexus plugins for interacting with the host system. Plugins authenticate via OAuth 2.1 client_credentials grant (POST /oauth/token) and use the access token as a Bearer token.",
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
        containers::list_all_containers,
        containers::inspect_container,
        containers::container_logs,
        containers::container_stats,
        containers::start_container,
        containers::stop_container,
        containers::restart_container,
        containers::remove_container,
        containers::list_images,
        containers::inspect_image,
        containers::remove_image,
        containers::list_volumes,
        containers::remove_volume,
        containers::list_networks,
        containers::remove_network,
        containers::engine_info,
        network::proxy_request,
        settings::get_settings,
        settings::put_settings,
        extensions::list_extensions,
        extensions::call_extension,
        meta::meta_self,
        meta::meta_stats,
        meta::meta_credentials_list,
        meta::meta_credentials_resolve,
        events::publish_event,
        events::subscribe_events,
        events::query_event_log,
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
        containers::ContainerInfo,
        containers::ImageInfo,
        containers::VolumeInfo,
        containers::NetworkInfo,
        containers::EngineInfo,
        containers::ContainerLogs,
        containers::StatusMessage,
        network::ProxyRequest,
        network::ProxyResponse,
        extensions::PluginExtensionView,
        extensions::PluginOperationView,
        extensions::ListExtensionsResponse,
        extensions::CallExtensionRequest,
        extensions::CallExtensionResponse,
        extensions::ExtensionErrorResponse,
        meta::MetaSelf,
        meta::MetaPermission,
        meta::MetaStats,
        meta::CredentialProviderList,
        meta::CredentialProvider,
        meta::CredentialScope,
        meta::CredentialRequest,
        meta::CredentialResponse,
        meta::MetaErrorResponse,
        events::PublishResponse,
        events::EventLogEntry,
        events::EventLogResponse,
        events::EventErrorResponse,
    )),
    modifiers(&SecurityAddon),
    tags(
        (name = "system", description = "Host system information"),
        (name = "filesystem", description = "Read and write files on the host"),
        (name = "process", description = "List host processes"),
        (name = "containers", description = "Container, image, volume, and network management"),
        (name = "network", description = "Network proxy for external requests"),
        (name = "settings", description = "Per-plugin settings (scoped to authenticated plugin)"),
        (name = "extensions", description = "Host extension operations"),
        (name = "meta", description = "Plugin self-introspection and credential vending"),
        (name = "events", description = "CloudEvents event bus — publish, subscribe (SSE), query log")
    )
)]
pub struct ApiDoc;

pub async fn start_server(
    state: AppState,
    approvals: Arc<ApprovalBridge>,
    oauth_store: Arc<oauth::OAuthStore>,
    active_theme: ActiveTheme,
    api_key_store: ApiKeyStore,
    dispatch: event_bus::Dispatch,
    audit: AuditWriter,
) -> Result<(), Box<dyn std::error::Error>> {
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::predicate(|origin, _| {
            let o = origin.as_bytes();
            o.starts_with(b"http://127.0.0.1")
                || o.starts_with(b"http://localhost")
                || o.starts_with(b"tauri://")
        }))
        .allow_methods(Any)
        .allow_headers(Any);

    // 100 requests per second per plugin — generous for normal use, blocks abuse
    let limiter = rate_limit::RateLimiter::new(100, std::time::Duration::from_secs(1));

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
        // Containers
        .route(
            "/v1/containers",
            routing::get(containers::list_all_containers),
        )
        .route(
            "/v1/containers/images",
            routing::get(containers::list_images),
        )
        .route(
            "/v1/containers/images/{id}",
            routing::get(containers::inspect_image)
                .delete(containers::remove_image),
        )
        .route(
            "/v1/containers/volumes",
            routing::get(containers::list_volumes),
        )
        .route(
            "/v1/containers/volumes/{name}",
            routing::delete(containers::remove_volume),
        )
        .route(
            "/v1/containers/networks",
            routing::get(containers::list_networks),
        )
        .route(
            "/v1/containers/networks/{id}",
            routing::delete(containers::remove_network),
        )
        .route(
            "/v1/containers/engine",
            routing::get(containers::engine_info),
        )
        .route(
            "/v1/containers/{id}",
            routing::get(containers::inspect_container)
                .delete(containers::remove_container),
        )
        .route(
            "/v1/containers/{id}/logs",
            routing::get(containers::container_logs),
        )
        .route(
            "/v1/containers/{id}/stats",
            routing::get(containers::container_stats),
        )
        .route(
            "/v1/containers/{id}/start",
            routing::post(containers::start_container),
        )
        .route(
            "/v1/containers/{id}/stop",
            routing::post(containers::stop_container),
        )
        .route(
            "/v1/containers/{id}/restart",
            routing::post(containers::restart_container),
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
        // Plugin metadata (self-introspection + credential vending)
        .route("/v1/meta/self", routing::get(meta::meta_self))
        .route("/v1/meta/stats", routing::get(meta::meta_stats))
        .route(
            "/v1/meta/credentials",
            routing::get(meta::meta_credentials_list),
        )
        .route(
            "/v1/meta/credentials/{ext_id}",
            routing::post(meta::meta_credentials_resolve),
        )
        // Events (CloudEvents bus)
        .route("/v1/events", routing::post(events::publish_event))
        .route(
            "/v1/events/subscribe",
            routing::get(events::subscribe_events),
        )
        .route("/v1/events/log", routing::get(events::query_event_log))
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
        .layer(Extension(oauth_store.clone()))
        .layer(Extension(approvals.clone()))
        .layer(Extension(dispatch.executor))
        .layer(Extension(dispatch.bus.clone()))
        .layer(Extension(dispatch.store))
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
    let mcp_event_bus = dispatch.bus;
    let audit_for_oauth = audit.clone();
    let audit_for_mcp_auth = audit.clone();
    let mcp_audit_for_factory = audit;
    let mcp_service = StreamableHttpService::new(
        move || {
            Ok(mcp::NexusMcpServer::new(
                mcp_state_for_factory.clone(),
                mcp_approvals_for_factory.clone(),
                mcp_audit_for_factory.clone(),
                mcp_event_bus.clone(),
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
        .layer(Extension(oauth_store.clone()))
        .layer(Extension(mcp_session_store))
        .layer(Extension(api_key_store))
        .layer(Extension(audit_for_mcp_auth));

    // OAuth 2.1 discovery + authorization endpoints (public, no auth required)
    // Global rate limit: 10 requests per 10 seconds across all callers
    let global_limiter = rate_limit::GlobalRateLimiter::new(10, std::time::Duration::from_secs(10));
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
        .layer(axum_middleware::from_fn(rate_limit::global_rate_limit_middleware))
        .layer(Extension(global_limiter))
        .layer(Extension(oauth_store.clone()))
        .layer(Extension(approvals.clone()))
        .layer(Extension(pending_auth))
        .layer(Extension(active_theme.clone()))
        .layer(Extension(audit_for_oauth));

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
        // Native MCP endpoint (streamable HTTP — primary connection mode)
        .merge(mcp_native_routes)
        // Authenticated routes (plugin Bearer token via OAuth 2.1)
        .nest("/api", authenticated_routes)
        .layer(cors)
        .layer(axum_middleware::from_fn(mcp::http_request_logging))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:9600").await?;
    log::info!("Host API server listening on 127.0.0.1:9600");
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}

async fn openapi_spec() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}
