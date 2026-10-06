//! The shared, per-process part of the HTTP layer: configuration, crypto, clock and app state.

use std::any::Any;
use std::sync::Arc;
use std::time::Duration;

use axum::http::{HeaderName, HeaderValue};

use crate::clock::SharedClock;
use crate::csp::ContentSecurityPolicy;
use crate::crypto::SharedCrypto;
use crate::rate_limit::RateLimitStore;
use crate::exceptions::ErrorPages;
use crate::request::ProxyConfig;
use crate::session::SessionConfig;

#[derive(Debug, Clone)]
pub struct KitConfig {
    pub proxy: ProxyConfig,
    /// `config.force_ssl`: redirect plain HTTP to HTTPS, send HSTS, flag cookies `secure`.
    pub force_ssl: bool,
    /// The HSTS header value sent with `force_ssl` (`ssl_options = { hsts: { subdomains: true } }`).
    pub hsts: String,
    pub session: SessionConfig,
    /// `forgery_protection_origin_check` (on since `load_defaults 5.0`).
    pub forgery_protection_origin_check: bool,
    /// `action_dispatch.default_headers` (`load_defaults 7.1`).
    pub default_headers: Vec<(HeaderName, HeaderValue)>,
    /// `config.content_security_policy`, with its nonce generator; `None` sends no policy.
    pub content_security_policy: Option<Arc<ContentSecurityPolicy>>,
    /// `public/404.html`, `422.html`, `500.html`, ... (`ActionDispatch::PublicExceptions`).
    pub error_pages: ErrorPages,
    /// Largest request body accepted; `None` is unlimited, like Puma.
    pub max_body_bytes: Option<usize>,
    /// Per-request timeout (`408` when exceeded); `None` disables it.
    pub request_timeout: Option<Duration>,
}

impl Default for KitConfig {
    fn default() -> Self {
        Self {
            proxy: ProxyConfig::default(),
            force_ssl: false,
            hsts: "max-age=63072000; includeSubDomains".into(),
            session: SessionConfig::default(),
            forgery_protection_origin_check: true,
            default_headers: rails_default_headers(),
            content_security_policy: None,
            error_pages: ErrorPages::default(),
            max_body_bytes: None,
            request_timeout: None,
        }
    }
}

impl KitConfig {
    /// Campfire's production settings: `assume_ssl` and `force_ssl` unless `DISABLE_SSL` is set,
    /// and HSTS for a year (`ssl_options = { hsts: { expires: 1.year, subdomains: true } }`, where
    /// `1.year` is 365.2425 days) (`reference/config/environments/production.rb`).
    pub fn production(disable_ssl: bool) -> Self {
        let mut config = Self::default();
        config.proxy.assume_ssl = !disable_ssl;
        config.force_ssl = !disable_ssl;
        config.hsts = format!("max-age={HSTS_ONE_YEAR}; includeSubDomains");
        config
    }
}

/// `1.year.to_i`: ActiveSupport's year is 365.2425 days.
pub const HSTS_ONE_YEAR: u64 = 31_556_952;

pub fn rails_default_headers() -> Vec<(HeaderName, HeaderValue)> {
    [
        ("x-frame-options", "SAMEORIGIN"),
        ("x-xss-protection", "0"),
        ("x-content-type-options", "nosniff"),
        ("x-permitted-cross-domain-policies", "none"),
        ("referrer-policy", "strict-origin-when-cross-origin"),
    ]
    .into_iter()
    .map(|(k, v)| (HeaderName::from_static(k), HeaderValue::from_static(v)))
    .collect()
}

/// Axum router state: everything a request needs that outlives it. Cheap to clone.
#[derive(Clone)]
pub struct Kit {
    pub(crate) inner: Arc<KitInner>,
}

pub(crate) struct KitInner {
    pub config: KitConfig,
    pub crypto: SharedCrypto,
    pub clock: SharedClock,
    pub state: Arc<dyn Any + Send + Sync>,
    /// The counters `rate_limit` keeps (Rails' cache store).
    #[cfg(any(test, feature = "test-support"))]
    pub rate_limits: Arc<RateLimitStore>,
    #[cfg(not(any(test, feature = "test-support")))]
    pub rate_limits: RateLimitStore,
}

impl std::fmt::Debug for Kit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Kit").field("config", &self.inner.config).finish()
    }
}

impl Kit {
    /// `state` is the application's own state (database handles etc.), reachable from actions
    /// with [`crate::Ctx::state`].
    pub fn new<S: Send + Sync + 'static>(config: KitConfig, crypto: SharedCrypto, clock: SharedClock, state: S) -> Self {
        Self { inner: Arc::new(KitInner { config, crypto, clock, state: Arc::new(state), rate_limits: Default::default() }) }
    }

    /// The original system helpers change configuration during a browser
    /// session. Their private host rebinds state without losing crypto, clock
    /// or the original middleware's rate-limit counters.
    #[cfg(any(test, feature = "test-support"))]
    pub fn fixture_rebind<S: Send + Sync + 'static>(&self, config: KitConfig, state: S) -> Self {
        Self { inner: Arc::new(KitInner {
            config,
            crypto: self.inner.crypto.clone(),
            clock: self.inner.clock.clone(),
            state: Arc::new(state),
            rate_limits: self.inner.rate_limits.clone(),
        }) }
    }

    pub fn config(&self) -> &KitConfig {
        &self.inner.config
    }

    pub fn crypto(&self) -> &SharedCrypto {
        &self.inner.crypto
    }

    pub fn clock(&self) -> &SharedClock {
        &self.inner.clock
    }

    /// The app's `rate_limit` counters.
    pub fn rate_limits(&self) -> &RateLimitStore {
        &self.inner.rate_limits
    }

    pub(crate) fn error_pages(&self) -> &ErrorPages {
        &self.inner.config.error_pages
    }

    pub fn state<S: Send + Sync + 'static>(&self) -> &S {
        self.inner.state.downcast_ref::<S>().expect("Kit state has a different type")
    }
}

#[cfg(test)]
mod fixture_tests {
    use super::*;
    use crate::clock::{Clock, FrozenClock};
    use crate::crypto::RailsCrypto;
    use jiff::SignedDuration;
    use rails_compat::Secrets;
    use serde_json::json;

    #[test]
    fn fixture_rebind_preserves_live_counters_clock_and_cookies() {
        let now = "2026-03-02T15:55:00Z".parse().unwrap();
        let clock = Arc::new(FrozenClock::new(now));
        let crypto: SharedCrypto = Arc::new(RailsCrypto::new(Arc::new(Secrets::new(&"6".repeat(128)))));
        let original = Kit::new(KitConfig::default(), crypto, clock.clone(), "initial state");
        let session = json!({"session_id": 42});
        let cookie = original.crypto().encrypt_cookie("_campfire_session", &session, None);
        assert_eq!(original.rate_limits().increment("fixture-session", SignedDuration::from_secs(60), now), 1);

        let rebound = original.fixture_rebind(KitConfig { force_ssl: true, ..original.config().clone() }, 19_u64);
        assert!(rebound.config().force_ssl);
        assert!(!original.config().force_ssl);
        assert_eq!(*rebound.state::<u64>(), 19);
        assert_eq!(*original.state::<&str>(), "initial state");
        assert_eq!(rebound.crypto().decrypt_cookie("_campfire_session", &cookie, rebound.clock().now()), Some(session));
        assert_eq!(rebound.rate_limits().increment("fixture-session", SignedDuration::from_secs(60), now), 2);
        assert_eq!(original.rate_limits().increment("fixture-session", SignedDuration::from_secs(60), now), 3);

        clock.advance(SignedDuration::from_secs(61));
        assert_eq!(rebound.clock().now(), clock.now());
        assert_eq!(rebound.rate_limits().increment("fixture-session", SignedDuration::from_secs(60), rebound.clock().now()), 1);
    }
}
