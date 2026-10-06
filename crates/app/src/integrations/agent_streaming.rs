//! The trailing stream broadcast waits outside the SQLite writer, then checks its stamp.
use crate::{app::App, queue::Registry};
use campfire_db::models::agent_streaming::{self as domain, StreamTrailingBroadcastJob};
use campfire_jobs::{Execution, JobKind, JobResult, Outcome, RetryPolicy};
use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize)]
#[serde(transparent)]
struct Trailing(StreamTrailingBroadcastJob);
impl campfire_db::Job for Trailing {
    const CLASS: &'static str = "Message::StreamTrailingBroadcastJob";
}
impl JobKind for Trailing {
    fn retry_policy() -> RetryPolicy {
        RetryPolicy::application_job()
    }
}
pub fn register(registry: &mut Registry) {
    registry.register(trailing);
}
async fn trailing(app: App, job: Trailing, _: Execution) -> JobResult {
    let last = campfire_db::Timestamp::parse_db(&job.0.last_broadcast_at)
        .ok_or_else(|| anyhow::anyhow!("invalid trailing stream stamp"))?;
    let wait = last.since(domain::BROADCAST_INTERVAL).as_microsecond()
        - app.db.env().now().as_microsecond();
    if wait > 0 {
        tokio::time::sleep(std::time::Duration::from_micros(wait as u64)).await;
    }
    app.db.write(move |tx| domain::trailing(tx, &job.0)).await?;
    Ok(Outcome::Done)
}



#[cfg(any(test, feature = "test-support"))]
pub async fn run_trailing_fixture(app: App, job: StreamTrailingBroadcastJob) {
    let now=app.db.env().now();
    trailing(app,Trailing(job),Execution{id:0,executions:1,enqueued_at:now,scheduled_at:now}).await.unwrap();
}
