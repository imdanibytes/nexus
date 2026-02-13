use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use axum::extract::Request;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use super::middleware::AuthenticatedPlugin;

/// Per-plugin fixed-window rate limiter.
///
/// Each plugin gets `max_requests` per `window`. When exceeded, requests
/// are rejected with 429 Too Many Requests until the window resets.
///
/// Runs AFTER auth middleware so the plugin identity is available.
#[derive(Clone)]
pub struct RateLimiter {
    inner: &'static RateLimiterInner,
}

struct RateLimiterInner {
    counters: Mutex<HashMap<String, WindowCounter>>,
    max_requests: u64,
    window: Duration,
}

struct WindowCounter {
    count: u64,
    window_start: Instant,
}

impl RateLimiter {
    /// Create a rate limiter allowing `max_requests` per `window` per plugin.
    pub fn new(max_requests: u64, window: Duration) -> Self {
        let inner = Box::leak(Box::new(RateLimiterInner {
            counters: Mutex::new(HashMap::new()),
            max_requests,
            window,
        }));
        Self { inner }
    }

    /// Check if a request from `plugin_id` is allowed. Returns `true` if
    /// under the limit, `false` if the plugin should be throttled.
    pub fn check(&self, plugin_id: &str) -> bool {
        let mut counters = self.inner.counters.lock().unwrap_or_else(|e| e.into_inner());
        let now = Instant::now();

        let counter = counters.entry(plugin_id.to_string()).or_insert(WindowCounter {
            count: 0,
            window_start: now,
        });

        // Reset window if expired
        if now.duration_since(counter.window_start) >= self.inner.window {
            counter.count = 0;
            counter.window_start = now;
        }

        counter.count += 1;
        counter.count <= self.inner.max_requests
    }
}

/// Axum middleware that enforces per-plugin rate limits.
///
/// Must be layered AFTER `auth_middleware` so that `AuthenticatedPlugin` is
/// present in request extensions.
pub async fn rate_limit_middleware(
    req: Request,
    next: Next,
) -> Response {
    // Extract the rate limiter and plugin identity from extensions
    let limiter = req.extensions().get::<RateLimiter>().cloned();
    let plugin_id = req
        .extensions()
        .get::<AuthenticatedPlugin>()
        .map(|a| a.plugin_id.clone());

    if let (Some(limiter), Some(plugin_id)) = (limiter, plugin_id) {
        if !limiter.check(&plugin_id) {
            log::warn!("Rate limited plugin={}", plugin_id);
            return (
                StatusCode::TOO_MANY_REQUESTS,
                [("retry-after", "1")],
                "Rate limit exceeded",
            )
                .into_response();
        }
    }

    next.run(req).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn under_limit_is_allowed() {
        let limiter = RateLimiter::new(5, Duration::from_secs(1));
        assert!(limiter.check("plugin-a"));
        assert!(limiter.check("plugin-a"));
        assert!(limiter.check("plugin-a"));
    }

    #[test]
    fn at_limit_is_allowed() {
        let limiter = RateLimiter::new(3, Duration::from_secs(1));
        assert!(limiter.check("plugin-a")); // 1
        assert!(limiter.check("plugin-a")); // 2
        assert!(limiter.check("plugin-a")); // 3 (at limit)
    }

    #[test]
    fn over_limit_is_blocked() {
        let limiter = RateLimiter::new(3, Duration::from_secs(1));
        assert!(limiter.check("plugin-a")); // 1
        assert!(limiter.check("plugin-a")); // 2
        assert!(limiter.check("plugin-a")); // 3
        assert!(!limiter.check("plugin-a")); // 4 — blocked
        assert!(!limiter.check("plugin-a")); // 5 — still blocked
    }

    #[test]
    fn plugins_have_independent_counters() {
        let limiter = RateLimiter::new(2, Duration::from_secs(1));
        assert!(limiter.check("plugin-a")); // a: 1
        assert!(limiter.check("plugin-a")); // a: 2
        assert!(!limiter.check("plugin-a")); // a: blocked

        assert!(limiter.check("plugin-b")); // b: 1 — independent
        assert!(limiter.check("plugin-b")); // b: 2
        assert!(!limiter.check("plugin-b")); // b: blocked
    }

    #[test]
    fn window_resets_after_duration() {
        let limiter = RateLimiter::new(2, Duration::from_millis(50));
        assert!(limiter.check("plugin-a"));
        assert!(limiter.check("plugin-a"));
        assert!(!limiter.check("plugin-a")); // blocked

        // Wait for window to expire
        std::thread::sleep(Duration::from_millis(60));

        assert!(limiter.check("plugin-a")); // window reset — allowed
        assert!(limiter.check("plugin-a")); // still in new window
        assert!(!limiter.check("plugin-a")); // blocked again
    }
}
