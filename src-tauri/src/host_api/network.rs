use axum::{extract::State, http::StatusCode, Extension, Json};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
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

/// Returns true if an IP address is private/loopback/link-local.
fn is_private_ip(ip: IpAddr) -> bool {
    let ip = canonicalize_ip(ip);
    match ip {
        IpAddr::V4(v4) => v4.is_private() || v4.is_loopback() || v4.is_link_local(),
        IpAddr::V6(v6) => {
            v6.is_loopback()
                || v6.segments()[0] == 0xfe80 // link-local
                || v6.segments()[0] & 0xfe00 == 0xfc00 // unique local (fc00::/7)
        }
    }
}

/// Returns true if the host resolves to a private/loopback/link-local address.
fn is_private_host(host: &str) -> bool {
    let host = strip_brackets(host);

    if host == "localhost" || host == "host.docker.internal" {
        return true;
    }

    if let Ok(raw_ip) = host.parse::<IpAddr>() {
        return is_private_ip(raw_ip);
    }

    // Treat .local and .internal TLDs as local
    host.ends_with(".local") || host.ends_with(".internal")
}

/// Resolve a hostname via DNS and validate the resolved IP.
///
/// Returns `(resolved_addr, is_private)`. Rejects metadata IPs outright.
/// This is the DNS rebinding mitigation — we classify based on the actual
/// resolved IP, not the hostname string.
async fn resolve_and_classify(
    host: &str,
    port: u16,
) -> Result<(SocketAddr, bool), StatusCode> {
    let lookup = format!("{}:{}", strip_brackets(host), port);
    let addrs: Vec<SocketAddr> = tokio::net::lookup_host(&lookup)
        .await
        .map_err(|e| {
            log::warn!("DNS resolution failed for {}: {}", host, e);
            StatusCode::BAD_GATEWAY
        })?
        .collect();

    if addrs.is_empty() {
        log::warn!("DNS resolution returned no addresses for {}", host);
        return Err(StatusCode::BAD_GATEWAY);
    }

    let addr = addrs[0];
    let ip = canonicalize_ip(addr.ip());

    // Check if resolved IP hits a metadata endpoint
    if is_metadata_ip(&ip.to_string()) {
        log::warn!(
            "DNS rebinding blocked: {} resolved to metadata IP {}",
            host, ip
        );
        return Err(StatusCode::FORBIDDEN);
    }

    Ok((addr, is_private_ip(ip)))
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

    // Hostname-level checks first (scheme, metadata, Host API relay)
    let _hostname_perm = required_network_permission(&parsed)?;

    let host = parsed.host_str().ok_or(StatusCode::BAD_REQUEST)?.to_string();
    let port = parsed.port_or_known_default().unwrap_or(80);

    // DNS rebinding mitigation: resolve the hostname to an IP and classify
    // based on the ACTUAL resolved address, not the hostname string.
    // Pin the connection to the resolved IP so there's no TOCTOU race.
    let (resolved_addr, resolved_is_private) = if host.parse::<IpAddr>().is_ok() {
        // IP literal — parse directly, no DNS needed
        let bare = strip_brackets(&host);
        let ip: IpAddr = bare.parse().map_err(|_| StatusCode::BAD_REQUEST)?;
        let addr = SocketAddr::new(canonicalize_ip(ip), port);
        (addr, is_private_ip(ip))
    } else {
        // Hostname — resolve via DNS and validate the resolved IP
        resolve_and_classify(&host, port).await?
    };

    // Classify based on resolved IP
    if resolved_is_private && resolved_addr.port() == HOST_API_PORT {
        return Err(StatusCode::FORBIDDEN);
    }

    let required_perm = if resolved_is_private {
        Permission::NetworkLocal
    } else {
        Permission::NetworkInternet
    };

    // Check permission
    {
        let mgr = state.read().await;
        if !mgr.permissions.has_permission(&auth.plugin_id, &required_perm) {
            return Err(StatusCode::FORBIDDEN);
        }
    }

    // Pin the hostname to the resolved IP so reqwest connects to exactly
    // the address we validated (no TOCTOU window for DNS rebinding).
    let client_builder = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .danger_accept_invalid_certs(resolved_is_private)
        .resolve(&host, resolved_addr);

    let initial_is_private = resolved_is_private;

    let client = client_builder.redirect(reqwest::redirect::Policy::custom(move |attempt| {
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

    let response = builder.send().await.map_err(|e| {
        log::warn!(
            "Proxy request failed: url={} plugin={} error={}",
            req.url,
            auth.plugin_id,
            e
        );
        StatusCode::BAD_GATEWAY
    })?;

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

#[cfg(test)]
mod tests {
    use super::*;

    // ── strip_brackets ───────────────────────────────────────

    #[test]
    fn strip_brackets_ipv6() {
        assert_eq!(strip_brackets("[::1]"), "::1");
    }

    #[test]
    fn strip_brackets_ipv4_passthrough() {
        assert_eq!(strip_brackets("127.0.0.1"), "127.0.0.1");
    }

    #[test]
    fn strip_brackets_hostname_passthrough() {
        assert_eq!(strip_brackets("example.com"), "example.com");
    }

    #[test]
    fn strip_brackets_partial_bracket_ignored() {
        // Malformed bracket notation — just pass through
        assert_eq!(strip_brackets("[::1"), "[::1");
        assert_eq!(strip_brackets("::1]"), "::1]");
    }

    // ── canonicalize_ip ──────────────────────────────────────

    #[test]
    fn canonicalize_ipv4_passthrough() {
        let ip: IpAddr = "1.2.3.4".parse().unwrap();
        assert_eq!(canonicalize_ip(ip), ip);
    }

    #[test]
    fn canonicalize_ipv6_mapped_to_v4() {
        let ip: IpAddr = "::ffff:127.0.0.1".parse().unwrap();
        let canonical = canonicalize_ip(ip);
        assert_eq!(canonical, "127.0.0.1".parse::<IpAddr>().unwrap());
    }

    #[test]
    fn canonicalize_native_ipv6_unchanged() {
        let ip: IpAddr = "::1".parse().unwrap();
        assert_eq!(canonicalize_ip(ip), ip);
    }

    #[test]
    fn canonicalize_ipv6_mapped_metadata() {
        let ip: IpAddr = "::ffff:169.254.169.254".parse().unwrap();
        let canonical = canonicalize_ip(ip);
        assert_eq!(canonical, "169.254.169.254".parse::<IpAddr>().unwrap());
    }

    // ── is_private_ip ────────────────────────────────────────

    #[test]
    fn private_ip_loopback_v4() {
        assert!(is_private_ip("127.0.0.1".parse().unwrap()));
    }

    #[test]
    fn private_ip_loopback_v6() {
        assert!(is_private_ip("::1".parse().unwrap()));
    }

    #[test]
    fn private_ip_rfc1918_10() {
        assert!(is_private_ip("10.0.0.1".parse().unwrap()));
    }

    #[test]
    fn private_ip_rfc1918_172() {
        assert!(is_private_ip("172.16.0.1".parse().unwrap()));
    }

    #[test]
    fn private_ip_rfc1918_192() {
        assert!(is_private_ip("192.168.1.1".parse().unwrap()));
    }

    #[test]
    fn private_ip_link_local_v4() {
        assert!(is_private_ip("169.254.1.1".parse().unwrap()));
    }

    #[test]
    fn private_ip_link_local_v6() {
        assert!(is_private_ip("fe80::1".parse().unwrap()));
    }

    #[test]
    fn private_ip_unique_local_v6() {
        assert!(is_private_ip("fc00::1".parse().unwrap()));
        assert!(is_private_ip("fd00::1".parse().unwrap()));
    }

    #[test]
    fn private_ip_mapped_loopback() {
        // ::ffff:127.0.0.1 should be detected as private via canonicalization
        assert!(is_private_ip("::ffff:127.0.0.1".parse().unwrap()));
    }

    #[test]
    fn private_ip_mapped_rfc1918() {
        assert!(is_private_ip("::ffff:192.168.1.1".parse().unwrap()));
    }

    #[test]
    fn public_ip_not_private() {
        assert!(!is_private_ip("8.8.8.8".parse().unwrap()));
        assert!(!is_private_ip("1.1.1.1".parse().unwrap()));
        assert!(!is_private_ip("93.184.216.34".parse().unwrap()));
    }

    #[test]
    fn public_ipv6_not_private() {
        assert!(!is_private_ip("2001:4860:4860::8888".parse().unwrap()));
    }

    // ── is_private_host ──────────────────────────────────────

    #[test]
    fn private_host_localhost() {
        assert!(is_private_host("localhost"));
    }

    #[test]
    fn private_host_docker_internal() {
        assert!(is_private_host("host.docker.internal"));
    }

    #[test]
    fn private_host_dot_local() {
        assert!(is_private_host("myservice.local"));
    }

    #[test]
    fn private_host_dot_internal() {
        assert!(is_private_host("myservice.internal"));
    }

    #[test]
    fn private_host_ip_literal() {
        assert!(is_private_host("127.0.0.1"));
        assert!(is_private_host("192.168.1.1"));
        assert!(is_private_host("10.0.0.1"));
    }

    #[test]
    fn private_host_ipv6_brackets() {
        assert!(is_private_host("[::1]"));
        assert!(is_private_host("[fe80::1]"));
    }

    #[test]
    fn public_host_not_private() {
        assert!(!is_private_host("example.com"));
        assert!(!is_private_host("api.github.com"));
        assert!(!is_private_host("8.8.8.8"));
    }

    // ── is_metadata_ip ───────────────────────────────────────

    #[test]
    fn metadata_aws() {
        assert!(is_metadata_ip("169.254.169.254"));
    }

    #[test]
    fn metadata_gcp() {
        assert!(is_metadata_ip("metadata.google.internal"));
    }

    #[test]
    fn metadata_alibaba() {
        assert!(is_metadata_ip("100.100.100.200"));
    }

    #[test]
    fn metadata_ipv6_mapped_aws() {
        assert!(is_metadata_ip("::ffff:169.254.169.254"));
    }

    #[test]
    fn metadata_ipv6_mapped_alibaba() {
        assert!(is_metadata_ip("::ffff:100.100.100.200"));
    }

    #[test]
    fn metadata_fd00_prefix() {
        assert!(is_metadata_ip("fd00::1"));
        assert!(is_metadata_ip("fd00:ec2::254"));
    }

    #[test]
    fn metadata_not_regular_ips() {
        assert!(!is_metadata_ip("8.8.8.8"));
        assert!(!is_metadata_ip("192.168.1.1"));
        assert!(!is_metadata_ip("example.com"));
    }

    #[test]
    fn metadata_bracketed_ipv6() {
        assert!(is_metadata_ip("[::ffff:169.254.169.254]"));
    }

    // ── required_network_permission ──────────────────────────

    fn parse_url(s: &str) -> reqwest::Url {
        reqwest::Url::parse(s).unwrap()
    }

    #[test]
    fn permission_http_public() {
        let result = required_network_permission(&parse_url("http://example.com/"));
        assert_eq!(result.unwrap(), Permission::NetworkInternet);
    }

    #[test]
    fn permission_https_public() {
        let result = required_network_permission(&parse_url("https://api.github.com/repos"));
        assert_eq!(result.unwrap(), Permission::NetworkInternet);
    }

    #[test]
    fn permission_http_private() {
        let result = required_network_permission(&parse_url("http://localhost:8080/"));
        assert_eq!(result.unwrap(), Permission::NetworkLocal);
    }

    #[test]
    fn permission_private_ip_literal() {
        let result = required_network_permission(&parse_url("http://192.168.1.1/"));
        assert_eq!(result.unwrap(), Permission::NetworkLocal);
    }

    #[test]
    fn permission_rejects_file_scheme() {
        let result = required_network_permission(&parse_url("file:///etc/passwd"));
        assert_eq!(result.unwrap_err(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn permission_rejects_ftp_scheme() {
        let result = required_network_permission(&parse_url("ftp://example.com/file"));
        assert_eq!(result.unwrap_err(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn permission_blocks_aws_metadata() {
        let result = required_network_permission(
            &parse_url("http://169.254.169.254/latest/meta-data/"),
        );
        assert_eq!(result.unwrap_err(), StatusCode::FORBIDDEN);
    }

    #[test]
    fn permission_blocks_gcp_metadata() {
        let result = required_network_permission(
            &parse_url("http://metadata.google.internal/computeMetadata/v1/"),
        );
        assert_eq!(result.unwrap_err(), StatusCode::FORBIDDEN);
    }

    #[test]
    fn permission_blocks_alibaba_metadata() {
        let result = required_network_permission(
            &parse_url("http://100.100.100.200/latest/meta-data/"),
        );
        assert_eq!(result.unwrap_err(), StatusCode::FORBIDDEN);
    }

    #[test]
    fn permission_blocks_host_api_relay_localhost() {
        let result = required_network_permission(
            &parse_url("http://localhost:9600/api/v1/system/info"),
        );
        assert_eq!(result.unwrap_err(), StatusCode::FORBIDDEN);
    }

    #[test]
    fn permission_blocks_host_api_relay_127() {
        let result = required_network_permission(
            &parse_url("http://127.0.0.1:9600/api/v1/system/info"),
        );
        assert_eq!(result.unwrap_err(), StatusCode::FORBIDDEN);
    }

    #[test]
    fn permission_blocks_host_api_relay_docker() {
        let result = required_network_permission(
            &parse_url("http://host.docker.internal:9600/api/v1/system/info"),
        );
        assert_eq!(result.unwrap_err(), StatusCode::FORBIDDEN);
    }

    #[test]
    fn permission_allows_private_non_api_port() {
        // Same host but different port — should be allowed as NetworkLocal
        let result = required_network_permission(
            &parse_url("http://localhost:8080/some/path"),
        );
        assert_eq!(result.unwrap(), Permission::NetworkLocal);
    }

    #[test]
    fn permission_blocks_ipv6_host_api() {
        let result = required_network_permission(
            &parse_url("http://[::1]:9600/api/v1/system/info"),
        );
        assert_eq!(result.unwrap_err(), StatusCode::FORBIDDEN);
    }
}
