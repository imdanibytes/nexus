use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use axum::extract::{Extension, Path, Query};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Json, Redirect, Response};

use crate::host_api::approval::{ApprovalBridge, ApprovalDecision, ApprovalRequest};
use crate::ActiveTheme;

use super::store::{normalize_redirect_uri, OAuthStore};
use super::types::AuthorizeParams;

// ---------------------------------------------------------------------------
// Pending authorization state (shared between authorize + poll endpoints)
// ---------------------------------------------------------------------------

/// Stores the result of a pending authorization. The authorize endpoint
/// returns HTML immediately and spawns the approval flow in the background.
/// The poll endpoint checks this map to see if the approval completed.
struct PendingAuth {
    client_name: String,
    created: Instant,
    /// None = still waiting, Some(url) = redirect URL (success or error)
    redirect_url: Option<String>,
}

/// Shared map of pending authorizations, keyed by `state` param (unique nonce
/// per authorization attempt). Using `state` as key ensures browser reloads
/// of the same authorize URL don't create duplicate approval requests.
#[derive(Clone, Default)]
pub struct PendingAuthMap(Arc<Mutex<HashMap<String, PendingAuth>>>);

impl PendingAuthMap {
    pub fn new() -> Self {
        Self::default()
    }

    fn cleanup_stale(&self) {
        let cutoff = Instant::now() - std::time::Duration::from_secs(15 * 60);
        let mut map = self.0.lock().unwrap_or_else(|e| e.into_inner());
        map.retain(|_, v| v.created > cutoff);
    }
}

// ---------------------------------------------------------------------------
// GET /oauth/authorize
// ---------------------------------------------------------------------------

/// OAuth 2.1 Authorization Endpoint — Authorization Code + PKCE.
///
/// Pre-approved clients get an immediate redirect. Others get an HTML consent
/// page that polls for approval status — no more hanging connections.
pub async fn authorize(
    Extension(store): Extension<Arc<OAuthStore>>,
    Extension(approvals): Extension<Arc<ApprovalBridge>>,
    Extension(pending): Extension<PendingAuthMap>,
    Extension(theme): Extension<ActiveTheme>,
    Query(params): Query<AuthorizeParams>,
) -> Result<Response, StatusCode> {
    let theme_name = theme.get();

    // Validate response_type
    if params.response_type != "code" {
        return Ok(error_redirect(
            &params.redirect_uri,
            "unsupported_response_type",
            "Only 'code' is supported",
            &params.state,
        ));
    }

    // Validate code_challenge_method
    if params.code_challenge_method != "S256" {
        return Ok(error_redirect(
            &params.redirect_uri,
            "invalid_request",
            "Only S256 is supported",
            &params.state,
        ));
    }

    // Look up client
    let client = store
        .get_client(&params.client_id)
        .ok_or(StatusCode::BAD_REQUEST)?;

    // Validate redirect_uri matches registration
    let normalized_uri = normalize_redirect_uri(params.redirect_uri.clone());
    let uri_valid = client.redirect_uris.iter().any(|registered| {
        normalize_redirect_uri(registered.clone()) == normalized_uri
    });
    if !uri_valid {
        log::warn!(
            "OAuth authorize: redirect_uri mismatch for client {}. Got: {}, registered: {:?}",
            params.client_id,
            params.redirect_uri,
            client.redirect_uris
        );
        return Err(StatusCode::BAD_REQUEST);
    }

    let scopes: Vec<String> = if params.scope.is_empty() {
        vec!["mcp".to_string()]
    } else {
        params.scope.split_whitespace().map(String::from).collect()
    };

    // Pre-approved clients skip consent — immediate redirect
    if store.is_client_approved(&params.client_id) {
        log::info!(
            "OAuth authorize: client {} pre-approved, skipping consent",
            client.client_name
        );
        let code = store.create_authorization_code(
            params.client_id,
            params.redirect_uri.clone(),
            params.code_challenge,
            scopes,
            params.resource,
            params.state.clone(),
        );
        return Ok(success_redirect(&params.redirect_uri, &code, &params.state));
    }

    // Deduplicate: if this exact state is already pending, just return the page again
    {
        let map = pending.0.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(entry) = map.get(&params.state) {
            // Already pending or completed — return the consent page (it will poll)
            let html = consent_html(&entry.client_name, &params.state, &theme_name);
            return Ok(([(header::CONTENT_TYPE, "text/html; charset=utf-8")], html).into_response());
        }
    }

    // Cleanup stale entries
    pending.cleanup_stale();

    let client_name = client.client_name.clone();

    // Store pending entry BEFORE spawning the approval task
    {
        let mut map = pending.0.lock().unwrap_or_else(|e| e.into_inner());
        map.insert(
            params.state.clone(),
            PendingAuth {
                client_name: client_name.clone(),
                created: Instant::now(),
                redirect_url: None,
            },
        );
    }

    // Spawn the approval flow in the background
    let pending_clone = pending.clone();
    let store_clone = store.clone();
    let state_key = params.state.clone();
    let redirect_uri = params.redirect_uri.clone();
    let client_id = params.client_id.clone();
    let code_challenge = params.code_challenge.clone();
    let resource = params.resource.clone();
    let scopes_clone = scopes.clone();

    tokio::spawn(async move {
        let request_id = uuid::Uuid::new_v4().to_string();
        let mut context = std::collections::HashMap::new();
        context.insert("client_name".to_string(), client_name.clone());
        context.insert("client_id".to_string(), client_id.clone());
        context.insert("scopes".to_string(), scopes_clone.join(", "));

        let approval_req = ApprovalRequest {
            id: request_id,
            plugin_id: client_id.clone(),
            plugin_name: client_name.clone(),
            category: "oauth_authorize".to_string(),
            permission: "mcp".to_string(),
            context,
        };

        log::info!(
            "OAuth authorize: requesting consent for client {}",
            client_name
        );

        let decision = approvals.request_approval(approval_req).await;

        let redirect_url = match decision {
            ApprovalDecision::Approve => {
                store_clone.approve_client(&client_id);
                let code = store_clone.create_authorization_code(
                    client_id,
                    redirect_uri.clone(),
                    code_challenge,
                    scopes_clone,
                    resource,
                    state_key.clone(),
                );
                log::info!(
                    "OAuth authorize: client {} approved (persistent)",
                    client_name
                );
                success_url(&redirect_uri, &code, &state_key)
            }
            ApprovalDecision::ApproveOnce => {
                let code = store_clone.create_authorization_code_once(
                    client_id,
                    redirect_uri.clone(),
                    code_challenge,
                    scopes_clone,
                    resource,
                    state_key.clone(),
                );
                log::info!(
                    "OAuth authorize: client {} approved (1 hour, no refresh)",
                    client_name
                );
                success_url(&redirect_uri, &code, &state_key)
            }
            ApprovalDecision::Deny => {
                log::info!("OAuth authorize: client {} denied", client_name);
                error_url(&redirect_uri, "access_denied", "User denied the request", &state_key)
            }
        };

        // Store the result so the poll endpoint can return it
        let mut map = pending_clone.0.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(entry) = map.get_mut(&state_key) {
            entry.redirect_url = Some(redirect_url);
        }
    });

    // Return the consent HTML page immediately
    let html = consent_html(&client.client_name, &params.state, &theme_name);
    Ok(([(header::CONTENT_TYPE, "text/html; charset=utf-8")], html).into_response())
}

// ---------------------------------------------------------------------------
// GET /oauth/authorize/poll/:state
// ---------------------------------------------------------------------------

#[derive(serde::Serialize)]
pub(crate) struct PollResponse {
    status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    redirect: Option<String>,
}

/// Poll endpoint — returns JSON with the authorization status.
pub(crate) async fn authorize_poll(
    Extension(pending): Extension<PendingAuthMap>,
    Path(state): Path<String>,
) -> Json<PollResponse> {
    let map = pending.0.lock().unwrap_or_else(|e| e.into_inner());
    match map.get(&state) {
        Some(entry) if entry.redirect_url.is_some() => Json(PollResponse {
            status: "complete",
            redirect: entry.redirect_url.clone(),
        }),
        Some(_) => Json(PollResponse {
            status: "waiting",
            redirect: None,
        }),
        None => Json(PollResponse {
            status: "expired",
            redirect: None,
        }),
    }
}

// ---------------------------------------------------------------------------
// URL builders
// ---------------------------------------------------------------------------

fn success_url(redirect_uri: &str, code: &str, state: &str) -> String {
    let sep = if redirect_uri.contains('?') { "&" } else { "?" };
    format!(
        "{}{}code={}&state={}",
        redirect_uri,
        sep,
        urlencoding::encode(code),
        urlencoding::encode(state),
    )
}

fn error_url(redirect_uri: &str, error: &str, description: &str, state: &str) -> String {
    let sep = if redirect_uri.contains('?') { "&" } else { "?" };
    format!(
        "{}{}error={}&error_description={}&state={}",
        redirect_uri,
        sep,
        urlencoding::encode(error),
        urlencoding::encode(description),
        urlencoding::encode(state),
    )
}

fn success_redirect(redirect_uri: &str, code: &str, state: &str) -> Response {
    Redirect::to(&success_url(redirect_uri, code, state)).into_response()
}

fn error_redirect(redirect_uri: &str, error: &str, description: &str, state: &str) -> Response {
    Redirect::to(&error_url(redirect_uri, error, description, state)).into_response()
}

// ---------------------------------------------------------------------------
// HTML consent page
// ---------------------------------------------------------------------------

fn consent_html(client_name: &str, state: &str, theme: &str) -> String {
    let client_escaped = client_name.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;");
    let state_escaped = state.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;");

    // Theme-aware accent color
    let accent = if theme == "nebula" { "#8b8bf5" } else { "#2DD4A8" };

    format!(
        include_str!("consent.html"),
        accent = accent,
        client_escaped = client_escaped,
        state_escaped = state_escaped,
    )
}
