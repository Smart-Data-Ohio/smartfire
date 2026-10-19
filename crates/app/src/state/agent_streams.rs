use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Clone, Copy, Debug)]
pub struct Timing {
    pub recheck: Duration,
    pub keepalive: Duration,
    pub lifetime: Duration,
}

impl Timing {
    pub fn from_lookup(get: impl Fn(&str) -> Option<String>) -> Self {
        let duration = |key, default| {
            Duration::from_millis(
                get(key)
                    .and_then(|value| value.parse::<u64>().ok())
                    .filter(|value| *value > 0)
                    .unwrap_or(default)
                    .min(default),
            )
        };
        let mut lifetime = duration("AGENT_EVENT_STREAM_LIFETIME_MS", 240_000);
        let timeout = campfire_kit::front::FrontConfig::from_lookup(&get).http_write_timeout;
        if !timeout.is_zero() {
            lifetime = lifetime.min(timeout.mul_f64(0.8));
        }
        Self {
            recheck: duration("AGENT_EVENT_STREAM_RECHECK_MS", 1_000),
            keepalive: duration("AGENT_EVENT_STREAM_KEEPALIVE_MS", 20_000),
            lifetime,
        }
    }
}

#[derive(Clone, Default)]
pub struct State(Arc<Mutex<HashMap<i64, usize>>>);

impl State {
    pub fn acquire(&self, credential_id: i64) -> Option<Permit> {
        let mut slots = self.0.lock().unwrap();
        let count = slots.entry(credential_id).or_default();
        if *count >= 3 {
            return None;
        }
        *count += 1;
        Some(Permit {
            state: self.clone(),
            credential_id,
        })
    }
}

pub struct Permit {
    state: State,
    credential_id: i64,
}

impl Drop for Permit {
    fn drop(&mut self) {
        let mut slots = self.state.0.lock().unwrap();
        let count = slots.get_mut(&self.credential_id).unwrap();
        *count -= 1;
        if *count == 0 {
            slots.remove(&self.credential_id);
        }
    }
}
