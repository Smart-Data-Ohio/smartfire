//! `ActionController::RateLimiting`: `rate_limit to:, within:, by:, with:, store:, name:, scope:`.
//!
//! Rails counts in its cache store: `store.increment("rate-limit:#{scope}:#{name}:#{by}", 1,
//! expires_in: within)`, and the request is limited once the count passes `to`. The expiry is set
//! when the counter starts and not moved by later increments (Redis' `EXPIRE ... NX`, and
//! `MemoryStore` keeps the entry's `expires_at`), so each key gets a fixed window from its first
//! request. A limited request still counts.
//!
//! The kit keeps one [`RateLimitStore`] for the app (what `Rails.cache`, one Redis for every Puma
//! worker, is to Rails; the port is one process). A limit can bring its own store, as
//! `csp_reports` does with its per-process `MemoryStore`.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use jiff::{SignedDuration, Timestamp};

/// Counters with a fixed expiry each: `ActiveSupport::Cache::Store#increment` with `expires_in`.
#[derive(Debug, Default)]
pub struct RateLimitStore {
    counters: Mutex<HashMap<String, (u64, Timestamp)>>,
}

impl RateLimitStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// `increment(key, 1, expires_in: within)`: the count after this request. An expired counter
    /// starts over at 1 with a new window.
    pub fn increment(&self, key: &str, within: SignedDuration, now: Timestamp) -> u64 {
        let mut counters = self.counters.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        counters.retain(|_, (_, expires_at)| *expires_at > now);
        let counter = counters.entry(key.to_string()).or_insert((0, now + within));
        counter.0 += 1;
        counter.0
    }

    /// `Rails.cache.clear`, for tests.
    pub fn clear(&self) {
        self.counters.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).clear();
    }
}

/// One `rate_limit` declaration. `scope` defaults to the controller path and `by` (given when
/// checking) to `request.remote_ip`.
#[derive(Debug, Clone)]
pub struct RateLimit {
    pub to: u64,
    pub within: SignedDuration,
    pub scope: &'static str,
    pub name: Option<&'static str>,
    /// `store:`; `None` is the app's store.
    pub store: Option<Arc<RateLimitStore>>,
}

impl RateLimit {
    /// `rate_limit to:, within:` in the controller at `scope` (its `controller_path`).
    pub fn new(scope: &'static str, to: u64, within: SignedDuration) -> Self {
        Self { to, within, scope, name: None, store: None }
    }

    /// `name:`, for a second limit in the same controller.
    pub fn named(mut self, name: &'static str) -> Self {
        self.name = Some(name);
        self
    }

    /// `store:`
    pub fn store(mut self, store: Arc<RateLimitStore>) -> Self {
        self.store = Some(store);
        self
    }

    /// `["rate-limit", scope, name, by].compact.join(":")`
    pub fn cache_key(&self, by: &str) -> String {
        [Some("rate-limit"), Some(self.scope), self.name, Some(by)].into_iter().flatten().collect::<Vec<_>>().join(":")
    }

    /// Counts this request in `store` (or the limit's own) and says whether it's over the limit.
    pub fn exceeded(&self, store: &RateLimitStore, by: &str, now: Timestamp) -> bool {
        let store = self.store.as_deref().unwrap_or(store);
        store.increment(&self.cache_key(by), self.within, now) > self.to
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(seconds: i64) -> Timestamp {
        Timestamp::from_second(1_767_268_800 + seconds).unwrap()
    }

    #[test]
    fn cache_keys_match_rails() {
        let limit = RateLimit::new("sessions", 10, SignedDuration::from_mins(3));
        assert_eq!(limit.cache_key("10.3.0.1"), "rate-limit:sessions:10.3.0.1");
        assert_eq!(limit.clone().named("google").cache_key("10.3.0.1"), "rate-limit:sessions:google:10.3.0.1");
    }

    #[test]
    fn limits_after_to_requests_until_the_window_from_the_first_ends() {
        let store = RateLimitStore::new();
        let limit = RateLimit::new("sessions", 3, SignedDuration::from_mins(3));
        let hits: Vec<bool> = (0..5).map(|i| limit.exceeded(&store, "10.0.0.1", at(i * 30))).collect();
        assert_eq!(hits, [false, false, false, true, true]);
        assert!(!limit.exceeded(&store, "10.0.0.2", at(150)), "each `by` has its own counter");
        // Later hits didn't move the expiry: the window ends 3 minutes after the first.
        assert!(limit.exceeded(&store, "10.0.0.1", at(179)));
        assert!(!limit.exceeded(&store, "10.0.0.1", at(180)), "reset");
        assert!(!limit.exceeded(&store, "10.0.0.1", at(181)));
    }

    #[test]
    fn a_limit_with_its_own_store_counts_there() {
        let (app, own) = (RateLimitStore::new(), Arc::new(RateLimitStore::new()));
        let limit = RateLimit::new("csp_reports", 1, SignedDuration::from_mins(1)).store(own.clone());
        assert!(!limit.exceeded(&app, "ip", at(0)));
        assert!(limit.exceeded(&app, "ip", at(1)));
        assert_eq!(app.increment("rate-limit:csp_reports:ip", SignedDuration::from_mins(1), at(2)), 1);
        own.clear();
        assert!(!limit.exceeded(&app, "ip", at(3)));
    }
}
