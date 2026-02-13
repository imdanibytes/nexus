use axum::{extract::State, http::StatusCode, Extension, Json};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;
use utoipa::ToSchema;

use super::middleware::AuthenticatedPlugin;
use crate::permissions::Permission;
use crate::AppState;

/// Host API listens on this port — plugins must not proxy to it.
const HOST_API_PORT: u16 = 9600;

/// Maximum response body size (10 MB).
const MAX_RESPONSE_BYTES: usize = 10 * 1024 * 1024;

#[derive(Deserialize, ToSchema)]
pub struct ProxyRequest {
    pub url: String,
    pub method: String,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct ProxyResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: String,
}

/// Strip brackets from IPv6 host strings. The `url` crate's `host_str()`
/// returns IPv6 addresses in bracket notation (`[::1]`), but `IpAddr::parse`
/// only accepts the bare address.
fn strip_brackets(host: &str) -> &str {
    host.strip_prefix('[')
        .and_then(|s| s.strip_suffix(']'))
        .unwrap_or(host)
}

/// Canonicalize an IP address. Converts IPv4-mapped IPv6 (`::ffff:1.2.3.4`)
/// to plain IPv4 so that all subsequent checks use a single code path.
fn canonicalize_ip(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V6(v6) => {
            if let Some(mapped) = v6.to_ipv4_mapped() {
                IpAddr::V4(mapped)
            } else {
                IpAddr::V6(v6)
            }
        }
        v4 => v4,
    }
}

/// Returns true if the host resolves to a private/loopback/link-local address.
fn is_private_host(host: &str) -> bool {
    let host = strip_brackets(host);

    if host == "localhost" || host == "host.docker.internal" {
        return true;
    }

    if let Ok(raw_ip) = host.parse::<IpAddr>() {
        let ip = canonicalize_ip(raw_ip);
        return match ip {
            IpAddr::V4(v4) => v4.is_private() || v4.is_loopback() || v4.is_link_local(),
            IpAddr::V6(v6) => {
                v6.is_loopback()
                    || v6.segments()[0] == 0xfe80 // link-local
                    || v6.segments()[0] & 0xfe00 == 0xfc00 // unique local (fc00::/7)
            }
        };
    }

    // Treat .local and .internal TLDs as local
    host.ends_with(".local") || host.ends_with(".internal")
}

/// Returns true if the URL targets a cloud metadata endpoint.
fn is_metadata_ip(host: &str) -> bool {
    let host = strip_brackets(host);

    // Check string matches first
    if matches!(
        host,
        "169.254.169.254" | "metadata.google.internal" | "100.100.100.200"
    ) || host.starts_with("fd00:")
    {
        return true;
    }

    // Also catch IPv6-mapped metadata IPs (e.g. ::ffff:169.254.169.254)
    if let Ok(raw_ip) = host.parse::<IpAddr>() {
        let ip = canonicalize_ip(raw_ip);
        if let IpAddr::V4(v4) = ip {
            let octets = v4.octets();
            // 169.254.169.254 (AWS/Azure)
            if octets == [169, 254, 169, 254] {
                return true;
            }
            // 100.100.100.200 (Alibaba)
            if octets == [100, 100, 100, 200] {
                return true;
            }
        }
    }

    false
}

/// Classify the request and return the required permission, or reject it.
fn required_network_permission(url: &reqwest::Url) -> Result<Permission, StatusCode> {
    // Only allow http and https
    match url.scheme() {
        "http" | "https" => {}
        _ => return Err(StatusCode::BAD_REQUEST),
    }

    let host = url.host_str().ok_or(StatusCode::BAD_REQUEST)?;

    // Block cloud metadata endpoints
    if is_metadata_ip(host) {
        return Err(StatusCode::FORBIDDEN);
    }

    // Block access to the Host API itself (anti-relay)
    if let Some(port) = url.port_or_known_default() {
        if is_private_host(host) && port == HOST_API_PORT {
            return Err(StatusCode::FORBIDDEN);
        }
    }

    if is_private_host(host) {
        Ok(Permission::NetworkLocal)
    } else {
        Ok(Permission::NetworkInternet)
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/network/proxy",
    tag = "network",
    security(("bearer_auth" = [])),
    request_body = ProxyRequest,
    responses(
        (status = 200, description = "Proxied response", body = ProxyResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 502, description = "Upstream error")
    )
)]
pub async fn proxy_request(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedPlugin>,
    Json(req): Json<ProxyRequest>,
) -> Result<Json<ProxyResponse>, StatusCode> {
    // Parse and validate URL
    let parsed = reqwest::Url::parse(&req.url).map_err(|_| StatusCode::BAD_REQUEST)?;
    let required_perm = required_network_permission(&parsed)?;

    // Check permission
    {
        let mgr = state.read().await;
        if !mgr.permissions.has_permission(&auth.plugin_id, &required_perm) {
            return Err(StatusCode::FORBIDDEN);
        }
    }

    // Build a client with a custom redirect policy that re-validates each hop.
    // Without this, a public URL could redirect to a metadata IP or private host,
    // bypassing the initial URL classification.
    let initial_is_private = is_private_host(
        parsed.host_str().unwrap_or("")
    );

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::custom(move |attempt| {
            let url = attempt.url();
            if let Some(host) = url.host_str() {
                // Always block metadata endpoints
                if is_metadata_ip(host) {
                    return attempt.error("redirect to metadata endpoint blocked");
                }
                // Always block Host API relay
                if let Some(port) = url.port_or_known_default() {
                    if is_private_host(host) && port == HOST_API_PORT {
                        return attempt.error("redirect to Host API blocked");
                    }
                }
                // Block public → private redirect (SSRF via open redirect)
                if !initial_is_private && is_private_host(host) {
                    return attempt.error("redirect from public to private network blocked");
                }
            }
            if attempt.previous().len() >= 5 {
                attempt.stop()
            } else {
                attempt.follow()
            }
        }))
        .build()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let method = req
        .method
        .parse::<reqwest::Method>()
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let mut builder = client.request(method, parsed);

    for (key, value) in &req.headers {
        builder = builder.header(key, value);
    }

    if let Some(body) = req.body {
        builder = builder.body(body);
    }

    let response = builder
        .send()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    // Check content-length before reading body
    if let Some(len) = response.content_length() {
        if len > MAX_RESPONSE_BYTES as u64 {
            return Err(StatusCode::BAD_GATEWAY);
        }
    }

    let status = response.status().as_u16();
    let headers: HashMap<String, String> = response
        .headers()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();

    let body = response
        .text()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Enforce size limit on actual body
    if body.len() > MAX_RESPONSE_BYTES {
        return Err(StatusCode::BAD_GATEWAY);
    }

    Ok(Json(ProxyResponse {
        status,
        headers,
        body,
    }))
}
