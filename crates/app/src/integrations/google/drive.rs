//! Viewer-scoped five-minute metadata and the Rails per-user minute counters.
use campfire_db::Timestamp;
use serde_json::Value;
use std::{
    collections::HashMap,
    sync::{Mutex, RwLock},
};
pub struct State {
    picker: RwLock<bool>,
    counts: Mutex<HashMap<(&'static str, i64, i64), u32>>,
    metadata: Mutex<HashMap<(i64, String), (Timestamp, Value)>>,
}
impl Default for State {
    fn default() -> Self {
        let present = |key| {
            std::env::var(key)
                .ok()
                .is_some_and(|v| !super::api::blank(&v))
        };
        Self::new(present("GOOGLE_CLIENT_ID") && present("GOOGLE_PICKER_API_KEY") && present("GOOGLE_CLOUD_PROJECT_NUMBER"))
    }
}
impl State {
    pub fn new(configured: bool) -> Self {
        Self {
            picker: RwLock::new(configured),
            counts: Mutex::new(HashMap::new()),
            metadata: Mutex::new(HashMap::new()),
        }
    }

    pub fn picker_configured(&self) -> bool {
        *self.picker.read().unwrap_or_else(|e| e.into_inner())
    }
    #[cfg(any(test, feature = "test-support"))]
    pub fn install_picker(&self, configured: bool) {
        *self.picker.write().unwrap() = configured;
    }
    pub fn throttled(&self, kind: &'static str, id: i64, limit: u32, now: Timestamp) -> bool {
        let bucket = now.as_second() / 60;
        let mut counts = self.counts.lock().unwrap_or_else(|e| e.into_inner());
        counts.retain(|(_, _, b), _| *b == bucket);
        let count = counts.entry((kind, id, bucket)).or_default();
        *count += 1;
        *count > limit
    }
    pub fn cached(&self, user_id: i64, file_id: &str, now: Timestamp) -> Option<Value> {
        let mut cache = self.metadata.lock().unwrap_or_else(|e| e.into_inner());
        cache.retain(|_, (expiry, _)| *expiry > now);
        cache
            .get(&(user_id, file_id.into()))
            .map(|(_, v)| v.clone())
    }
    pub fn cache(&self, user_id: i64, file_id: &str, now: Timestamp, file: Value) {
        self.metadata
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(
                (user_id, file_id.into()),
                (now.since(jiff::SignedDuration::from_secs(300)), file),
            );
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn google_drive_metadata_expires_after_five_minutes_and_budgets_roll_at_minute_boundaries() {
        let s = State::default();
        let now = Timestamp::from_second(60);
        s.cache(1, "fixture", now, serde_json::json!({"id":"fixture"}));
        assert!(
            s.cached(
                1,
                "fixture",
                now.since(jiff::SignedDuration::from_secs(299))
            )
            .is_some()
        );
        assert!(
            s.cached(
                1,
                "fixture",
                now.since(jiff::SignedDuration::from_secs(300))
            )
            .is_none()
        );
        for _ in 0..30 {
            assert!(!s.throttled("list", 1, 30, now));
        }
        assert!(s.throttled("list", 1, 30, now));
        assert!(!s.throttled("list", 1, 30, Timestamp::from_second(120)));
        assert!(!s.throttled("list", 2, 30, now));
    }
}
