//! ActiveJob's serialized `exception_executions`, kept with the durable job payload.
//! Unclassified jobs keep their existing payload and execution budget.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

const KEY: &str = "_campfire_retry_metadata_v1";

#[derive(Default, Serialize, Deserialize)]
pub(crate) struct State {
    pub arguments: Value,
    pub counts: BTreeMap<String, u32>,
}

impl State {
    pub fn from_value(value: Value) -> Self {
        if let Some(metadata) = value
            .as_object()
            .filter(|h| h.len() == 1)
            .and_then(|h| h.get(KEY))
            && let Ok(state) = serde_json::from_value(metadata.clone())
        {
            return state;
        }
        Self {
            arguments: value,
            counts: BTreeMap::new(),
        }
    }

    pub fn increment(&mut self, key: &str) -> u32 {
        let count = self.counts.entry(key.to_owned()).or_default();
        *count = count.saturating_add(1);
        *count
    }

    pub fn encode(&self) -> String {
        serde_json::json!({KEY: self}).to_string()
    }

    /// An interrupted execution has no known exception group. Keep its separate bounded
    /// process-loss budget, without consuming completed exception handlers' budgets twice.
    pub fn interrupted_executions(&self, total: u32) -> u32 {
        total.saturating_sub(
            self.counts
                .values()
                .copied()
                .fold(0u32, u32::saturating_add),
        )
    }
}

pub(crate) fn arguments(value: Value) -> Value {
    State::from_value(value).arguments
}
