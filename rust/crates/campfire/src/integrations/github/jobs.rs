//! Durable job arguments. Handlers belong to the fetcher and notifier domain slices.
use campfire_db::Job;
use campfire_jobs::{JobKind, RetryPolicy};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchPullRequestJob {
    pub pull_request_id: i64,
}
impl Job for FetchPullRequestJob {
    const CLASS: &'static str = "Github::FetchPullRequestJob";
}
impl JobKind for FetchPullRequestJob {
    fn retry_policy() -> RetryPolicy {
        RetryPolicy::no_retries()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliverSubscriptionEventJob {
    pub event: String,
    pub payload: Value,
}
impl Job for DeliverSubscriptionEventJob {
    const CLASS: &'static str = "Github::DeliverSubscriptionEventJob";
}
impl JobKind for DeliverSubscriptionEventJob {}
