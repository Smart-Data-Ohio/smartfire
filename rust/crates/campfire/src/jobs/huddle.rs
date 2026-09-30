//! Durable presence, join-notice, invitation-push and cleanup jobs.
//! Invitation/join payloads enqueue through WS17's seam. The invitation resolver and
//! stale-stream and cleanup sweeps run in this process; stream render callbacks remain a
//! lifecycle slice.
use campfire_db::Database;
use campfire_db::models::huddle_cleanup::{CleanupJob, HuddleCleanup, Operation};
use campfire_jobs::{Execution, JobKind, JobResult, Outcome, RetryPolicy};
use serde::{Deserialize, Serialize};

use super::Registry;
use crate::app::App;
use crate::huddle::{Config, RoomService};
use campfire_db::CachedStatements;

#[derive(Serialize, Deserialize)]
#[serde(transparent)]
struct Cleanup(CleanupJob);
impl campfire_db::Job for Cleanup {
    const CLASS: &'static str = "Huddle::CleanupJob";
}
impl JobKind for Cleanup {
    // The cleanup row carries its own schedule. Queue retries would bypass it.
    fn retry_policy() -> RetryPolicy {
        RetryPolicy::application_job().attempts(1)
    }
}

#[derive(Serialize, Deserialize)]
#[serde(transparent)]
struct Presence(campfire_db::models::huddle_grant::PresenceJob);
impl campfire_db::Job for Presence {
    const CLASS: &'static str = "Huddle::BroadcastPresenceJob";
}
impl JobKind for Presence {}

#[derive(Serialize, Deserialize)]
#[serde(transparent)]
struct Join(campfire_db::models::huddle_grant::JoinNoticeJob);
impl campfire_db::Job for Join {
    const CLASS: &'static str = "Huddle::JoinNoticeJob";
}
impl JobKind for Join {}

#[derive(Serialize, Deserialize)]
#[serde(transparent)]
struct Invitation(campfire_db::models::huddle_notices::PushInvitationJob);
impl campfire_db::Job for Invitation {
    const CLASS: &'static str = "Huddle::PushInvitationJob";
}
impl JobKind for Invitation {}

async fn join(app: App, job: Join, _: Execution) -> JobResult {
    app.db
        .write(move |tx| campfire_db::models::huddle_notices::notify_join(tx, job.0.grant_id))
        .await?;
    Ok(Outcome::Done)
}
async fn invitation(app: App, job: Invitation, _: Execution) -> JobResult {
    app.db
        .write(move |tx| {
            campfire_db::models::huddle_notices::push_invitation(tx, job.0.activity_item_id)
        })
        .await?;
    Ok(Outcome::Done)
}

async fn presence(app: App, job: Presence, _: Execution) -> JobResult {
    let room = app
        .db
        .read(move |conn| {
            Ok(
                campfire_db::models::huddle_grant::HuddleGrant::find_by_id(conn, job.0.grant_id)?
                    .map(|grant| grant.room_id),
            )
        })
        .await?;
    if let Some(room_id) = room {
        tokio::task::spawn_blocking(move || {
            crate::channels::huddle_effects::presence(&app, room_id)
        })
        .await??;
    }
    Ok(Outcome::Done)
}

/// Previous WS13 builds persisted sightings before the handler was available. Retry only
/// those exact unknown-class failures, preserving genuine rendering/validation failures.
pub(crate) async fn recover_unregistered(db: &Database) -> campfire_db::Result<usize> {
    db.write(|tx| Ok(tx.conn().execute_cached(
        "UPDATE background_jobs SET status='ready',attempts=0,run_at=?1,failed_at=NULL,updated_at=?1 WHERE status='failed' AND job_class IN ('Huddle::BroadcastPresenceJob','Huddle::JoinNoticeJob','Huddle::PushInvitationJob') AND last_error='no handler is registered for ' || job_class",
        [tx.now()],
    )?)).await
}

pub(super) fn register(registry: &mut Registry) {
    registry.register(cleanup);
    registry.register(presence);
    registry.register(join);
    registry.register(invitation);
}

async fn cleanup(app: App, job: Cleanup, _: Execution) -> JobResult {
    perform(
        &app.db,
        RoomService::new(Config::from_env()),
        job.0.cleanup_id,
        true,
    )
    .await?;
    Ok(Outcome::Done)
}

/// Commit the claim before contacting LiveKit, preserving the retry if the process dies.
/// A LiveKit failure leaves the database backoff intact and consumes the queue delivery.
pub(crate) async fn perform(
    db: &Database,
    service: RoomService,
    id: i64,
    from_queue: bool,
) -> anyhow::Result<bool> {
    let configured = service.admin_configured();
    let Some(row) = db
        .write(move |tx| HuddleCleanup::claim(tx, id, from_queue, configured))
        .await?
    else {
        return Ok(false);
    };
    let now = db.env().now().as_second();
    let result = match row.operation {
        Operation::RemoveParticipant => {
            service
                .remove_participant(&row.room_name, row.identity.as_deref().unwrap_or(""), now)
                .await
        }
        Operation::DeleteRoom => service.delete_room(&row.room_name, now).await,
    };
    if let Err(error) = result {
        tracing::warn!(id, attempt = row.attempts, %error, "Huddle cleanup failed");
        return Ok(false);
    }
    db.write(move |tx| HuddleCleanup::complete(tx, id)).await?;
    Ok(true)
}

pub(crate) async fn reconcile(db: &Database, service: RoomService) -> anyhow::Result<usize> {
    if let Err(error) = resolve_invitations(db).await {
        tracing::error!(%error,"Huddle invitation resolution failed");
    }
    if let Err(error) = end_stale_streams(db).await {
        tracing::error!(%error,"Huddle stream reconciliation failed");
    }
    if !service.admin_configured() {
        return Ok(0);
    }
    let now = db.env().now();
    let ids = db
        .read(move |conn| HuddleCleanup::due_ids(conn, now, 100))
        .await?;
    let mut completed = 0;
    for id in ids {
        match perform(db, service.clone(), id, false).await {
            Ok(true) => completed += 1,
            Ok(false) => {}
            Err(error) => tracing::error!(id, %error, "Unexpected huddle cleanup failure"),
        }
    }
    Ok(completed)
}

async fn resolve_invitations(db: &Database) -> campfire_db::Result<()> {
    let now = db.env().now();
    let ids = db
        .read(move |conn| campfire_db::models::huddle_invitations::overdue_ids(conn, now, None))
        .await?;
    for id in ids {
        db.write(move |tx| campfire_db::models::huddle_invitations::resolve_item(tx, id))
            .await?;
    }
    Ok(())
}

async fn end_stale_streams(db: &Database) -> campfire_db::Result<()> {
    let ids = db
        .read(campfire_db::models::huddle_stream_liveness::live_ids)
        .await?;
    for id in ids {
        db.write(move |tx| campfire_db::models::huddle_stream_liveness::end_stale(tx, id))
            .await?;
    }
    Ok(())
}
