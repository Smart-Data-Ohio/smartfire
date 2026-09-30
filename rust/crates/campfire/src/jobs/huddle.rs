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

#[derive(Serialize, Deserialize)]
#[serde(transparent)]
struct Ring(campfire_db::models::huddle_invitations::RingRequest);
impl campfire_db::Job for Ring {
    const CLASS: &'static str = "Notifications::HuddleRingJob";
}
impl JobKind for Ring {}

async fn ring(app: App, job: Ring, execution: Execution) -> JobResult {
    app.db.write(move |tx| {
        let Some(row) = campfire_jobs::inspect::find(tx.conn(), execution.id)? else { return Ok(()); };
        if row.arguments["cancelled"] == 1 {
            return Ok(());
        }
        campfire_db::models::huddle_invitations::publish_ring_with_policy(tx, &job.0, None)
    }).await?;
    Ok(Outcome::Done)
}

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
    register_sources(registry);
    registry.register(ring);
}

#[cfg(test)]
pub(super) fn source_registry() -> Registry {
    let mut registry = Registry::new();
    register_sources(&mut registry);
    registry
}

fn register_sources(registry: &mut Registry) {
    registry.register(cleanup);
    registry.register(presence);
    registry.register(join);
    registry.register(invitation);
}

#[cfg(test)]
mod review_tests {
    use super::*;
    use campfire_db::models::huddle_grant::HuddleGrant;
    use campfire_db::models::huddle_invitations::RingRequest;
    use campfire_db::{Job, Membership, Session};
    use crate::controllers::presenters::test_support::{TestApp, DAVID, JASON, DIRECT_DAVID_JASON};

    #[tokio::test]
    async fn ws13b_review_queued_and_already_claimed_rings_cannot_follow_explicit_leave() {
        for suppressed in [false, true] {
            let test = TestApp::boot().await.expect("parity seed required");
            test.booted.jobs.shutdown(std::time::Duration::from_secs(2)).await;
            let app = test.booted.app.clone();
            let grant = app.db.write(move |tx| {
                if suppressed { tx.conn().execute(r#"UPDATE users SET inbox_preferences='{"huddle_invitations":false}' WHERE id=?"#, [JASON])?; }
                let session = Session::start(tx, DAVID, None, None)?;
                let member = Membership::find_by_room_and_user(tx.conn(), DIRECT_DAVID_JASON, DAVID)?.unwrap();
                HuddleGrant::issue(tx, session.id, member.id, member.room_id, &campfire_db::models::room_delete::HuddleConfig {api_secret:Some("ws13b-review-fixture-value".into()), admin_configured:false})
            }).await.unwrap();
            let rows = app.db.read(campfire_jobs::inspect::all).await.unwrap();
            let queued = rows.iter().find(|j| j.class == RingRequest::CLASS).unwrap();
            let retained = serde_json::from_value::<RingRequest>(queued.arguments.clone()).unwrap();
            let execution = Execution { id: queued.id, executions:1, enqueued_at:queued.created_at, scheduled_at:queued.run_at };
            let id = grant.id;
            app.db.write(move |tx| {
                let mut grant = HuddleGrant::find_by_id(tx.conn(), id)?.unwrap();
                grant.record_seen(tx)?;
                grant.mark_out_of_call(tx, None)?;
                Ok(())
            }).await.unwrap();
            // Subscribe after the ended frame. A marker detects extra frames
            // deterministically, including a worker retaining its pre-leave payload.
            use campfire_kit::Crypto;
            use tokio_tungstenite::tungstenite::client::IntoClientRequest;
            let session = app.db.write(|tx| Session::start_with(tx, JASON, campfire_db::NewSession { two_factor_verified:true, ..Default::default() })).await.unwrap();
            let signed = campfire_kit::RailsCrypto::new(app.secrets.clone()).sign_cookie("session_token", &session.token, None);
            let listener = crate::channels::tests::support::bind_listener().await;
            let addr = listener.local_addr().unwrap();
            let router = app.cable.router::<()>("/cable");
            let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
            let mut request = format!("ws://{addr}/cable").into_client_request().unwrap();
            request.headers_mut().insert("origin", format!("http://{addr}").parse().unwrap());
            request.headers_mut().insert("cookie", format!("session_token={}", campfire_kit::cookies::escape(&signed)).parse().unwrap());
            request.headers_mut().insert("sec-websocket-protocol", "actioncable-v1-json".parse().unwrap());
            let (socket, _) = tokio_tungstenite::connect_async(request).await.unwrap();
            let mut client = crate::channels::tests::support::Client { socket };
            assert_eq!(serde_json::from_str::<serde_json::Value>(&client.next_text().await).unwrap()["type"], "welcome");
            let identifier = serde_json::json!({"channel":"ActivityChannel"}).to_string();
            client.confirm(&identifier).await;
            let stream = format!("user_{JASON}_activity");
            ring(app.clone(), Ring(retained), execution.clone()).await.unwrap();
            app.cable.broadcast(&stream, &serde_json::json!({"reviewMarker":true}));
            let frame: serde_json::Value = serde_json::from_str(&client.next_text().await).unwrap();
            assert_eq!(frame["message"], serde_json::json!({"reviewMarker":true}), "suppressed={suppressed}: claimed ring restarted an ended call");
            let execution_id = execution.id;
            let queued = app.db.read(move |conn| campfire_jobs::inspect::find(conn, execution_id)).await.unwrap().unwrap();
            ring(app.clone(), Ring(serde_json::from_value(queued.arguments).unwrap()), execution).await.unwrap();
            app.cable.broadcast(&stream, &serde_json::json!({"reviewMarker":true}));
            let frame: serde_json::Value = serde_json::from_str(&client.next_text().await).unwrap();
            assert_eq!(frame["message"], serde_json::json!({"reviewMarker":true}));
            client.socket.close(None).await.unwrap();
            server.abort();
            let _ = server.await;
        }
    }
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
        tracing::error!(
            "{}",
            reconciliation_failure_message("invitation resolution", &error)
        );
    }
    if let Err(error) = end_stale_streams(db).await {
        tracing::error!(
            "{}",
            reconciliation_failure_message("stream reconciliation", &error)
        );
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

/// The Rails reconciler logs the exception class, never its message or SQL.
/// Preserve its observable class names for the corresponding Rust model errors.
pub(crate) fn reconciliation_failure_message(phase: &str, error: &campfire_db::Error) -> String {
    let class = match error {
        error if error.is_record_not_unique() => "ActiveRecord::RecordNotUnique",
        campfire_db::Error::RecordInvalid(_) => "ActiveRecord::RecordInvalid",
        campfire_db::Error::RecordNotFound(_) => "ActiveRecord::RecordNotFound",
        campfire_db::Error::Sqlite(_) => "ActiveRecord::StatementInvalid",
        _ => "StandardError",
    };
    format!("Huddle {phase} failed: {class}")
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
