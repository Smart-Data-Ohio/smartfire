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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformAgentActionJob {
    pub approval_id: i64,
}
impl Job for PerformAgentActionJob {
    const CLASS: &'static str = "Github::PerformAgentActionJob";
}
impl JobKind for PerformAgentActionJob {
    fn retry_policy() -> RetryPolicy {
        RetryPolicy::no_retries()
    }
}

pub fn register(registry: &mut crate::queue::Registry) {
    registry.register(fetch_pull_request);
    registry.register(perform_agent_action);
    registry.register(deliver_subscription_event);
}

async fn fetch_pull_request(
    app: crate::app::App,
    job: FetchPullRequestJob,
    _: campfire_jobs::Execution,
) -> campfire_jobs::JobResult {
    super::fetcher::fetch(&app.db, &app.github_read, job.pull_request_id)
        .await
        .map_err(crate::queue::discard_missing)?;
    Ok(campfire_jobs::Outcome::Done)
}

async fn perform_agent_action(
    app: crate::app::App,
    job: PerformAgentActionJob,
    _: campfire_jobs::Execution,
) -> campfire_jobs::JobResult {
    super::agent_actions::perform(&app.db, &app.github_accounts, job.approval_id)
        .await
        .map_err(crate::queue::discard_missing)?;
    Ok(campfire_jobs::Outcome::Done)
}

async fn deliver_subscription_event(
    app: crate::app::App,
    job: DeliverSubscriptionEventJob,
    _: campfire_jobs::Execution,
) -> campfire_jobs::JobResult {
    super::notifier::deliver(&app.db, job.event, job.payload).await?;
    Ok(campfire_jobs::Outcome::Done)
}
