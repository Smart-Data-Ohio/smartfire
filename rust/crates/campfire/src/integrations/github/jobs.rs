//! GitHub durable jobs, with Rails' class-specific retry policies.
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

pub fn register(registry: &mut crate::jobs::Registry) {
    registry.register(fetch_pull_request);
}

async fn fetch_pull_request(
    app: crate::app::App,
    job: FetchPullRequestJob,
    _: campfire_jobs::Execution,
) -> campfire_jobs::JobResult {
    super::fetcher::fetch(&app.db, &app.github_read, job.pull_request_id)
        .await
        .map_err(crate::jobs::discard_missing)?;
    Ok(campfire_jobs::Outcome::Done)
}
