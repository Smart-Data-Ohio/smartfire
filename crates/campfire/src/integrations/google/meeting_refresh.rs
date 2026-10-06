//! Fetch/cache side of Calendar::MeetingRefresh; WS17 dispatches completed interval sets.
use super::{api, calendar};
use crate::{app::App, queue::Registry};
use campfire_db::{
    Timestamp,
    models::{google_calendar::MeetingRefreshJob, google_meeting_cache as cache},
};
use campfire_jobs::{Execution, JobKind, JobResult};
use serde::{Deserialize, Serialize};
use serde_json::json;
#[derive(Serialize, Deserialize)]
#[serde(transparent)]
struct Refresh(MeetingRefreshJob);
impl campfire_db::Job for Refresh {
    const CLASS: &'static str = "Calendar::MeetingRefreshJob";
}
impl JobKind for Refresh {}
pub fn register(registry: &mut Registry) {
    registry.register(perform);
}
async fn perform(app: App, job: Refresh, execution: Execution) -> JobResult {
    calendar::application_result(
        refresh(&app, job.0.user_id, Timestamp::from_jiff(app.clock.now()))
            .await
            .map(|_| ()),
        &execution,
    )
}
#[derive(Debug, PartialEq, Eq)]
pub enum Result {
    Skipped,
    Fresh,
    Ok,
    Error,
}
pub const NOT_CONNECTED: &str = "Connect Google Calendar to show calendar status.";
pub const RECONNECT: &str = "Google Calendar needs reconnecting before calendar status can update.";
pub const UNREACHABLE: &str = "Google Calendar couldn't be reached; calendar status will retry.";
/// The Google::Client#list_events boundary. Production uses Api; original-case
/// tests can inject an escaping client error without changing HTTP classification.
pub trait EventLister: Send + Sync {
    fn list<'a>(
        &'a self,
        app: &'a App,
        user_id: i64,
        start: Timestamp,
        end: Timestamp,
        now: Timestamp,
    ) -> crate::net::BoxFuture<'a, api::Result<serde_json::Value>>;
}
impl EventLister for api::Api {
    fn list<'a>(
        &'a self,
        app: &'a App,
        user_id: i64,
        start: Timestamp,
        end: Timestamp,
        now: Timestamp,
    ) -> crate::net::BoxFuture<'a, api::Result<serde_json::Value>> {
        Box::pin(self.list_events(&app.db, &app.secrets, user_id, start, end, now))
    }
}
pub async fn refresh(app: &App, user_id: i64, now: Timestamp) -> api::Result<Result> {
    refresh_with_client(app, user_id, now, app.google.api().as_ref()).await
}
pub async fn refresh_with_client(
    app: &App,
    user_id: i64,
    now: Timestamp,
    client: &dyn EventLister,
) -> api::Result<Result> {
    let settings=app.db.read(move|conn|{use rusqlite::OptionalExtension;Ok(conn.query_row("SELECT status,meeting_status_enabled,ooo_calendar_enabled,time_zone FROM users WHERE id=?",[user_id],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,bool>(1)?,r.get::<_,bool>(2)?,r.get::<_,Option<String>>(3)?))).optional()?)}).await?;
    let Some((0, meeting, ooo, zone)) = settings.filter(|(_, meeting, ooo, _)| *meeting || *ooo)
    else {
        return Ok(Result::Skipped);
    };
    if !calendar::usable(app, user_id).await? {
        store_error(app, user_id, now, NOT_CONNECTED, false).await?;
        return Ok(Result::Error);
    }
    let existing = app.db.read(move |conn| cache::find(conn, user_id)).await?;
    if existing.is_some_and(|c| {
        c.fetched_at
            .is_some_and(|t| t > now.ago(jiff::SignedDuration::from_secs(60)))
    }) {
        app.db
            .write(move |tx| cache::follow_up(tx, user_id, now))
            .await?;
        return Ok(Result::Fresh);
    }
    match client
        .list(
            app,
            user_id,
            now.ago(jiff::SignedDuration::from_hours(1)),
            now.since(jiff::SignedDuration::from_hours(if ooo {
                30 * 24
            } else {
                24
            })),
            now,
        )
        .await
    {
        Ok(response) => {
            let (busy, out) =
                cache::intervals(&response["items"], zone.as_deref().unwrap_or("UTC"));
            app.db
                .write(move |tx| {
                    cache::complete(
                        tx,
                        user_id,
                        Some(if meeting { busy } else { json!([]) }),
                        Some(if ooo { out } else { json!([]) }),
                        None,
                        now,
                    )
                })
                .await?;
            Ok(Result::Ok)
        }
        Err(api::Error::Unauthorized(_)) => {
            store_error(app, user_id, now, RECONNECT, false).await?;
            Ok(Result::Error)
        }
        Err(api::Error::Storage(e)) => Err(e.into()),
        Err(_) => {
            store_error(app, user_id, now, UNREACHABLE, true).await?;
            Ok(Result::Error)
        }
    }
}
async fn store_error(
    app: &App,
    user_id: i64,
    now: Timestamp,
    message: &str,
    keep_intervals: bool,
) -> api::Result<()> {
    let error = Some(message.to_owned());
    app.db
        .write(move |tx| {
            cache::complete(
                tx,
                user_id,
                (!keep_intervals).then(|| json!([])),
                (!keep_intervals).then(|| json!([])),
                error,
                now,
            )
        })
        .await?;
    Ok(())
}
