//! Unified Bearer token validation for all middleware (RFC 6750 §2.1).
//!
//! One function, used by both the plugin auth middleware and the MCP gateway
//! auth middleware, eliminating duplicated header parsing and store lookups.
//!
//! The `Authorization: Bearer <token>` scheme is defined in RFC 6750 §2.1.
//! Per RFC 7235 §2.1, the auth-scheme ("Bearer") comparison MUST be
//! case-insensitive.

use axum::http::HeaderMap;

use super::store::OAuthStore;
use crate::permissions::rar::AuthorizationDetail;

/// Result of validating a Bearer token from the Authorization header.
pub enum TokenValidation {
    /// Token is valid and not expired.
    Valid {
        client_id: String,
        client_name: String,
        plugin_id: Option<String>,
        authorization_details: Vec<AuthorizationDetail>,
    },
    /// A Bearer token was provided but is expired, revoked, or invalid.
    Invalid,
    /// No Authorization: Bearer header was present.
    Missing,
}

/// Extract and validate a Bearer token from HTTP headers.
///
/// Per RFC 7235 §2.1, the auth-scheme comparison is case-insensitive:
/// `Bearer`, `bearer`, `BEARER` are all valid.
pub fn validate_bearer(headers: &HeaderMap, oauth_store: &OAuthStore) -> TokenValidation {
    let bearer = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| {
            // RFC 7235 §2.1: auth-scheme comparison MUST be case-insensitive.
            if v.len() > 7 && v[..7].eq_ignore_ascii_case("bearer ") {
                Some(&v[7..])
            } else {
                None
            }
        });

    let token_str = match bearer {
        Some(t) => t,
        None => return TokenValidation::Missing,
    };

    match oauth_store.validate_access_token(token_str) {
        Some(access_token) => TokenValidation::Valid {
            client_id: access_token.client_id,
            client_name: access_token.client_name,
            plugin_id: access_token.plugin_id,
            authorization_details: access_token.authorization_details,
        },
        None => TokenValidation::Invalid,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderMap;

    fn test_store() -> (OAuthStore, tempfile::TempDir) {
        let dir = tempfile::TempDir::new().unwrap();
        let store = OAuthStore::load(dir.path());
        (store, dir)
    }

    #[test]
    fn missing_header_returns_missing() {
        let (store, _dir) = test_store();
        let headers = HeaderMap::new();
        assert!(matches!(validate_bearer(&headers, &store), TokenValidation::Missing));
    }

    #[test]
    fn non_bearer_header_returns_missing() {
        let (store, _dir) = test_store();
        let mut headers = HeaderMap::new();
        headers.insert("authorization", "Basic dXNlcjpwYXNz".parse().unwrap());
        assert!(matches!(validate_bearer(&headers, &store), TokenValidation::Missing));
    }

    #[test]
    fn invalid_token_returns_invalid() {
        let (store, _dir) = test_store();
        let mut headers = HeaderMap::new();
        headers.insert("authorization", "Bearer bad-token".parse().unwrap());
        assert!(matches!(validate_bearer(&headers, &store), TokenValidation::Invalid));
    }

    #[test]
    fn valid_token_returns_valid() {
        let (store, _dir) = test_store();
        let access = store.create_access_token(
            "client-1".into(),
            "Test Client".into(),
            vec!["mcp".into()],
            "http://127.0.0.1:9600/mcp".into(),
            Some("com.test.plugin".into()),
            vec![],
        );

        let mut headers = HeaderMap::new();
        headers.insert(
            "authorization",
            format!("Bearer {}", access.token).parse().unwrap(),
        );

        match validate_bearer(&headers, &store) {
            TokenValidation::Valid {
                client_id,
                client_name,
                plugin_id,
                ..
            } => {
                assert_eq!(client_id, "client-1");
                assert_eq!(client_name, "Test Client");
                assert_eq!(plugin_id, Some("com.test.plugin".to_string()));
            }
            _ => panic!("expected Valid"),
        }
    }

    /// RFC 7235 §2.1: auth-scheme comparison MUST be case-insensitive.
    #[test]
    fn bearer_scheme_case_insensitive() {
        let (store, _dir) = test_store();
        let access = store.create_access_token(
            "client-1".into(),
            "Test Client".into(),
            vec!["mcp".into()],
            "http://127.0.0.1:9600/mcp".into(),
            None,
            vec![],
        );

        for scheme in ["bearer", "Bearer", "BEARER", "bEaReR"] {
            let mut headers = HeaderMap::new();
            headers.insert(
                "authorization",
                format!("{} {}", scheme, access.token).parse().unwrap(),
            );
            assert!(
                matches!(validate_bearer(&headers, &store), TokenValidation::Valid { .. }),
                "scheme '{}' must be accepted per RFC 7235 §2.1",
                scheme
            );
        }
    }

    #[test]
    fn expired_token_returns_invalid() {
        let (store, _dir) = test_store();
        let access = store.create_access_token(
            "client-1".into(),
            "Test Client".into(),
            vec![],
            "".into(),
            None,
            vec![],
        );
        store.expire_access_token(&access.token);

        let mut headers = HeaderMap::new();
        headers.insert(
            "authorization",
            format!("Bearer {}", access.token).parse().unwrap(),
        );

        assert!(matches!(validate_bearer(&headers, &store), TokenValidation::Invalid));
    }
}
