use std::sync::Mutex as StdMutex;

use campfire_jobs::inspect::{self, JobRow};
use tokio::sync::Notify;

use super::*;
use campfire_db::Job;
use campfire_jobs::Execution;
use campfire_jobs::JobKind;
use campfire_jobs::JobResult;
use campfire_jobs::Outcome;
use campfire_jobs::RunnerConfig;
use crate::app::App;
use crate::queue::PurgeJob;
use crate::queue::Registry;
use crate::queue::discard_missing;
use serde::Deserialize;
use serde::Serialize;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use campfire_db::{Event, EventSink as _, JobRequest, Tx};
use campfire_jobs::JobQueue;
use futures_util::future::BoxFuture;
use crate::cable::Cable;
use crate::config::Config;
use crate::queue::{Jobs, PUSH_QUEUE, PushMessageJob, WEBHOOK_HOLD, WEBHOOKS_QUEUE, WebhookJob, request_for, runner_config};
use crate::server::{Booted, boot_with_services};
use crate::test_support::{WAIT, eventually, wait};

/// An app booted over an empty storage directory.
async fn app_in(dir: &std::path::Path) -> Booted {
    let root = dir.to_string_lossy().into_owned();
    let config = Config::from_lookup(|name| match name {
        "SECRET_KEY_BASE_DUMMY" => Some("1".into()),
        "CAMPFIRE_STORAGE_PATH" => Some(root.clone()),
        "DISABLE_SSL" => Some("true".into()), // plain HTTP requests, for the controller tests
        _ => None,
    })
    .unwrap();
    // Worker tests own their queue entries. Periodic-host tests start their loops
    // explicitly; an automatic retention tick must not race these queue assertions.
    boot_with_services(
        config,
        campfire_kit::clock::from_env().unwrap(),
        crate::net::Network::system(),
        periodic::Intervals { periodic: None, huddle: None },
    ).await.unwrap()
}

async fn app() -> (Booted, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    (app_in(dir.path()).await, dir)
}

#[tokio::test]
async fn huddle_presence_worker_discards_missing_grants_successfully() {
    let (booted, _dir) = app().await;
    booted.app.db.write(|tx| {
        tx.emit_after_commit(Event::job(&campfire_db::models::huddle_grant::PresenceJob { grant_id: -1 }));
        Ok(())
    }).await.unwrap();
    wait_for(&booted.app, "the presence worker", |rows| rows.iter().all(|job| job.class != "Huddle::BroadcastPresenceJob")).await;
    booted.jobs.shutdown(Duration::from_secs(2)).await;
}

#[tokio::test]
async fn huddle_join_and_invitation_workers_discard_missing_sources_successfully() {
    let (booted, _dir) = app().await;
    booted.app.db.write(|tx| {
        for (class, arguments) in [("Huddle::JoinNoticeJob", serde_json::json!({"grant_id":-1})), ("Huddle::PushInvitationJob", serde_json::json!({"activity_item_id":-1}))] {
            tx.emit_after_commit(Event::Job(JobRequest { class, arguments, wait: None }));
        }
        Ok(())
    }).await.unwrap();
    wait_for(&booted.app, "the join and invitation workers", |rows| rows.iter().all(|job| !["Huddle::JoinNoticeJob", "Huddle::PushInvitationJob"].contains(&job.class.as_str()))).await;
    booted.jobs.shutdown(Duration::from_secs(2)).await;
}

#[tokio::test]
async fn huddle_join_and_invitation_workers_persist_the_payload_for_ws17() {
    use crate::controllers::presenters::test_support::{TestApp, DAVID, JASON, DIRECT_DAVID_JASON};
    use campfire_db::models::huddle_grant::HuddleGrant;
    use campfire_db::models::huddle_notices::{PushInvitationJob, PushRequest};
    let Some(test) = TestApp::boot().await else { return; };
    test.booted.jobs.shutdown(Duration::from_secs(2)).await;
    let app = test.booted.app.clone();
    let registry = huddle::source_registry();
    let producer = campfire_jobs::start(app.db.clone(), app.jobs.queue.clone(), registry, app.clone(), runner_config(&app.config));
    let grant = app.db.write(|tx| {
        let session = campfire_db::Session::start(tx, DAVID, None, None)?;
        let membership = campfire_db::Membership::find_by_room_and_user(tx.conn(), DIRECT_DAVID_JASON, DAVID)?.unwrap();
        // Set up a quiet grant directly: issuance now rings the recipient, which correctly
        // suppresses the join notice tested here. The actual sighting and workers run below.
        let id = tx.conn().query_row("INSERT INTO huddle_grants(identity,room_name,session_id,user_id,membership_id,room_id,last_issued_at,created_at,updated_at) VALUES('ws13-worker-grant','ws13-worker-room',?,?,?,?,?,?,?) RETURNING id",rusqlite::params![session.id,DAVID,membership.id,membership.room_id,tx.now(),tx.now(),tx.now()],|r|r.get::<_,i64>(0))?;
        let mut grant = HuddleGrant::find_by_id(tx.conn(),id)?.unwrap();
        grant.record_seen(tx)?;
        Ok(grant)
    }).await.unwrap();
    let rows = wait_for(&app, "the durable join push request", |rows| rows.iter().any(|job| job.class == PushRequest::CLASS && job.arguments["kind"] == "huddle_join")).await;
    let request = rows.iter().find(|job| job.class == PushRequest::CLASS).unwrap();
    assert_eq!(request.arguments["recipient_id"],JASON);
    assert_eq!(request.arguments["sender_id"],DAVID);
    assert_eq!(request.arguments["payload"],serde_json::json!({"title":"David joined your huddle", "body":"Join from the conversation", "path":format!("/rooms/{DIRECT_DAVID_JASON}"), "tag":format!("huddle-{DIRECT_DAVID_JASON}")}));
    app.db.write(move |tx| {
        let item = campfire_db::ActivityItem::refresh_unread(tx,JASON,"HuddleGrant",grant.id,"huddle_started")?;
        tx.emit_after_commit(Event::job(&PushInvitationJob {activity_item_id:item.id}));
        Ok(())
    }).await.unwrap();
    let rows = wait_for(&app, "the durable invitation push request", |rows| rows.iter().any(|job| job.class == PushRequest::CLASS && job.arguments["kind"] == "huddle")).await;
    let request = rows.iter().find(|job| job.class == PushRequest::CLASS && job.arguments["kind"] == "huddle").unwrap();
    assert_eq!(request.arguments["recipient_id"],JASON);
    assert_eq!(request.arguments["payload"]["title"],"David started a huddle");
    assert!(request.arguments["room_membership_id"].as_i64().is_some());
    // This producer-only registry retains the wire intents for inspection. The full
    // registered WS17 policy/delivery path is exercised by the transport integration tests.
    producer.shutdown(Duration::from_secs(2)).await;
}

#[tokio::test]
async fn huddle_issuance_rings_and_pushes_once_and_suppresses_the_join_notice() {
    use crate::controllers::presenters::test_support::{TestApp,DAVID,JASON,DIRECT_DAVID_JASON};
    use campfire_db::models::{huddle_grant::HuddleGrant,huddle_invitations::RingRequest,huddle_notices::PushRequest};
    let Some(test)=TestApp::boot().await else { return; };
    test.booted.jobs.shutdown(Duration::from_secs(2)).await;
    let app=test.booted.app.clone();
    let registry = huddle::source_registry();
    let producer = campfire_jobs::start(app.db.clone(), app.jobs.queue.clone(), registry, app.clone(), runner_config(&app.config));
    let grant=app.db.write(|tx| {
        let session=campfire_db::Session::start(tx,DAVID,None,None)?;
        let member=campfire_db::Membership::find_by_room_and_user(tx.conn(),DIRECT_DAVID_JASON,DAVID)?.unwrap();
        let mut grant=HuddleGrant::issue(tx,session.id,member.id,member.room_id,&campfire_db::models::room_delete::HuddleConfig {api_secret:Some("ws13-fixture-value".into()),admin_configured:false})?;
        grant.record_seen(tx)?;
        Ok(grant)
    }).await.unwrap();
    let rows=wait_for(&app,"issuance invitation and ring requests",|rows| rows.iter().any(|j|j.class==PushRequest::CLASS && j.arguments["kind"]=="huddle") && rows.iter().any(|j|j.class==RingRequest::CLASS) && rows.iter().all(|j|j.class!="Huddle::JoinNoticeJob")).await;
    assert_eq!(rows.iter().filter(|j|j.class==RingRequest::CLASS).count(),1);
    let ring=rows.iter().find(|j|j.class==RingRequest::CLASS).unwrap();
    assert_eq!(ring.arguments["recipient_id"],JASON);
    assert_eq!(ring.arguments["sender_id"],DAVID);
    assert_eq!(ring.arguments["invitation"]["roomId"],DIRECT_DAVID_JASON);
    assert!(ring.arguments["invitation"].get("silent").is_none(),"a ring request invented a WS17 policy decision");
    assert!(!rows.iter().any(|j|j.class==PushRequest::CLASS && j.arguments["kind"]=="huddle_join"));
    app.db.write(move |tx|HuddleGrant::issue(tx,grant.session_id,grant.membership_id,grant.room_id,&campfire_db::models::room_delete::HuddleConfig {api_secret:Some("ws13-fixture-value".into()),admin_configured:false}).map(drop)).await.unwrap();
    assert_eq!(jobs(&app).iter().filter(|j|j.class==RingRequest::CLASS).count(),1,"a reuse inside the dedupe window rang again");
    producer.shutdown(Duration::from_secs(2)).await;
}

#[tokio::test]
async fn huddle_in_process_loop_resolves_invitations_without_livekit_admin_configuration() {
    use crate::controllers::presenters::test_support::{TestApp,DAVID,DIRECT_DAVID_JASON};
    use campfire_db::models::huddle_grant::HuddleGrant;
    let Some(test)=TestApp::boot().await else{return;};
    test.booted.jobs.shutdown(Duration::from_secs(2)).await;
    let app=test.booted.app.clone();
    let grant=app.db.write(|tx| {
        let session=campfire_db::Session::start(tx,DAVID,None,None)?;
        let member=campfire_db::Membership::find_by_room_and_user(tx.conn(),DIRECT_DAVID_JASON,DAVID)?.unwrap();
        HuddleGrant::issue(tx,session.id,member.id,member.room_id,&campfire_db::models::room_delete::HuddleConfig {api_secret:Some("ws13-fixture-value".into()),admin_configured:false})
    }).await.unwrap();
    let (item,stream)=app.db.write(move |tx| {
        let item=tx.conn().query_row("SELECT id FROM activity_items WHERE source_type='HuddleGrant' AND source_id=?",[grant.id],|r|r.get::<_,i64>(0))?;
        tx.conn().execute("UPDATE activity_items SET created_at=? WHERE id=?",rusqlite::params![tx.now().ago(jiff::SignedDuration::from_secs(46)),item])?;
        let stage=campfire_db::Room::create_for(tx,campfire_db::RoomType::Stage,Some("WS13 loop stage"),DAVID,&[DAVID])?;
        let presenter=campfire_db::Membership::find_by_room_and_user(tx.conn(),stage.id,DAVID)?.unwrap();
        let stream=tx.conn().query_row("INSERT INTO streams(room_id,membership_id,user_id,quality,started_at,created_at,updated_at) VALUES(?,?,?,'1080p15',?,?,?) RETURNING id",rusqlite::params![stage.id,presenter.id,DAVID,tx.now(),tx.now(),tx.now()],|r|r.get::<_,i64>(0))?;
        Ok((item,stream))
    }).await.unwrap();
    let config=runner_config(&app.config);
    let (_,ad_hoc)=Jobs::new(&registry(),&config).unwrap();
    let loops=periodic::Loops {periodic:None,huddle:Some(periodic::huddle_reconciler(Duration::from_millis(20)))};
    let runner=start(app.clone(),registry(),ad_hoc,config,loops);
    let deadline=tokio::time::Instant::now()+Duration::from_secs(5);
    loop {
        let done=app.db.read(move |conn|Ok(campfire_db::ActivityItem::find(conn,item)?.event_type=="huddle_missed" && conn.query_row("SELECT ended_at IS NOT NULL FROM streams WHERE id=?",[stream],|r|r.get::<_,bool>(0))?)).await.unwrap();
        if done {break;}
        assert!(tokio::time::Instant::now()<deadline,"the actual in-process huddle task did not resolve the invitation");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    runner.shutdown(Duration::from_secs(2)).await;
}

#[tokio::test]
async fn huddle_invitation_enqueue_failure_keeps_grant_and_rolls_back_invitation() {
    use crate::controllers::presenters::test_support::{TestApp,DAVID,DIRECT_DAVID_JASON};
    use campfire_db::models::huddle_grant::HuddleGrant;
    for class in ["Notifications::HuddleRingJob", "Huddle::PushInvitationJob"] {
        let Some(test)=TestApp::boot().await else {return;};
        test.booted.jobs.shutdown(Duration::from_secs(2)).await;
        let app=test.booted.app;
        let (session,member)=app.db.write(move |tx| {
            let session=campfire_db::Session::start(tx,DAVID,None,None)?;
            let member=campfire_db::Membership::find_by_room_and_user(tx.conn(),DIRECT_DAVID_JASON,DAVID)?.unwrap();
            tx.conn().execute_batch(&format!("CREATE TRIGGER ws13_reject_invitation_job BEFORE INSERT ON background_jobs WHEN NEW.job_class='{class}' BEGIN SELECT RAISE(ABORT,'ws13 reject invitation job'); END"))?;
            Ok((session.id,member.id))
        }).await.unwrap();
        let failed=app.db.write(move |tx|HuddleGrant::issue(tx,session,member,DIRECT_DAVID_JASON,&campfire_db::models::room_delete::HuddleConfig {api_secret:Some("ws13-fixture-value".into()),admin_configured:false})).await;
        assert!(failed.is_err());
        assert_eq!(app.db.read(move |conn|Ok(conn.query_row("SELECT COUNT(*) FROM huddle_grants WHERE session_id=?",[session],|r|r.get::<_,i64>(0))?)).await.unwrap(),1);
        assert_eq!(app.db.read(|conn|Ok(conn.query_row("SELECT COUNT(*) FROM activity_items WHERE source_type='HuddleGrant'",[],|r|r.get::<_,i64>(0))?)).await.unwrap(),0);
        assert!(!jobs(&app).iter().any(|j|j.class=="Notifications::HuddleRingJob" || j.class=="Huddle::PushInvitationJob"));
    }
}

#[tokio::test]
async fn huddle_push_enqueue_failure_rolls_back_the_notice_transaction() {
    use crate::controllers::presenters::test_support::{TestApp, DAVID, JASON, DIRECT_DAVID_JASON};
    use campfire_db::models::huddle_grant::HuddleGrant;
    use campfire_db::models::huddle_notices::{PushKind, PushPayload, PushRequest, enqueue_huddle_push, prepare_push};
    let Some(test) = TestApp::boot().await else { return; };
    test.booted.jobs.shutdown(Duration::from_secs(2)).await;
    let app = test.booted.app;
    let (id, membership_id) = app.db.write(|tx| {
        let session = campfire_db::Session::start(tx,DAVID,None,None)?;
        let member = campfire_db::Membership::find_by_room_and_user(tx.conn(),DIRECT_DAVID_JASON,DAVID)?.unwrap();
        let grant = HuddleGrant::issue(tx,session.id,member.id,member.room_id,&campfire_db::models::room_delete::HuddleConfig {api_secret:Some("ws13-fixture-value".into()),admin_configured:false})?;
        let item = campfire_db::ActivityItem::refresh_unread(tx,JASON,"HuddleGrant",grant.id,"huddle_started")?;
        tx.conn().execute_batch("CREATE TRIGGER ws13_reject_push BEFORE INSERT ON background_jobs WHEN NEW.job_class='Notifications::HuddlePushJob' BEGIN SELECT RAISE(ABORT,'ws13 reject push intent'); END")?;
        let recipient = campfire_db::Membership::find_by_room_and_user(tx.conn(),DIRECT_DAVID_JASON,JASON)?.unwrap();
        tx.conn().execute("UPDATE memberships SET connected_at=NULL,last_huddle_join_push_at=NULL WHERE id=?",[recipient.id])?;
        Ok((item.id, recipient.id))
    }).await.unwrap();
    let failed = app.db.write(move |tx| {
        // WS17 must enqueue the actual delivery in the transaction that owns this claim.
        tx.conn().execute("UPDATE activity_items SET read_at='2026-03-02 16:00:01' WHERE id=?",[id])?;
        let request = PushRequest {
            kind: PushKind::HuddleJoin, recipient_id:JASON, sender_id:DAVID,
            room_id:DIRECT_DAVID_JASON, room_membership_id:Some(membership_id),
            payload:PushPayload { title:"David joined a huddle".into(),body:"Join the call".into(),path:format!("/rooms/{DIRECT_DAVID_JASON}"),tag:format!("huddle-room-{DIRECT_DAVID_JASON}") },
        };
        assert!(prepare_push(tx,&request,true)?.is_some(),"the subscription and throttle claim did not run");
        enqueue_huddle_push(tx,&request);
        Ok(())
    }).await;
    assert!(failed.is_err(),"the failed durable enqueue did not fail the triggering write");
    assert!(app.db.read(move |conn|Ok(campfire_db::ActivityItem::find(conn,id)?.read_at.is_none())).await.unwrap());
    assert!(app.db.read(move |conn|Ok(campfire_db::Membership::find(conn,membership_id)?.last_huddle_join_push_at.is_none())).await.unwrap());
    assert!(!jobs(&app).iter().any(|job|job.class=="Notifications::HuddlePushJob"));
}

#[tokio::test]
async fn huddle_presence_recovers_only_the_previous_unknown_class_failures() {
    let (booted, _dir) = app().await;
    booted.jobs.shutdown(Duration::from_secs(2)).await;
    booted.app.db.write(|tx| {
        for (class, error) in [("Huddle::BroadcastPresenceJob", "no handler is registered for Huddle::BroadcastPresenceJob"), ("Huddle::JoinNoticeJob", "no handler is registered for Huddle::JoinNoticeJob"), ("Huddle::PushInvitationJob", "no handler is registered for Huddle::PushInvitationJob"), ("Huddle::BroadcastPresenceJob", "real rendering failure"), ("WS13OtherJob", "no handler is registered for WS13OtherJob")] {
            let id = boot_insert_failed_job(tx, class, error)?;
            assert!(id > 0);
        }
        Ok(())
    }).await.unwrap();
    assert_eq!(super::huddle::recover_unregistered(&booted.app.db).await.unwrap(), 3);
    let rows = jobs(&booted.app);
    assert_eq!(rows.iter().filter(|row| row.status == "ready").count(), 3);
    assert_eq!(rows.iter().filter(|row| row.status == "failed").count(), 2);
    assert_eq!(super::huddle::recover_unregistered(&booted.app.db).await.unwrap(), 0);
}

fn boot_insert_failed_job(tx: &Tx<'_>, class: &str, error: &str) -> campfire_db::Result<i64> {
    use campfire_db::CachedStatements;
    let arguments = if class=="Huddle::PushInvitationJob" {"{\"activity_item_id\":-1}"} else {"{\"grant_id\":-1}"};
    Ok(tx.conn().query_row_cached("INSERT INTO background_jobs(job_class,queue_name,arguments,payload_version,status,attempts,run_at,last_error,failed_at,created_at,updated_at) VALUES(?1,'default',?4,1,'failed',1,?3,?2,?3,?3,?3) RETURNING id", rusqlite::params![class, error, tx.now(), arguments], |row| row.get(0))?)
}

fn jobs(app: &App) -> Vec<JobRow> {
    app.db.read_blocking(inspect::all).unwrap()
}

async fn wait_for(app: &App, what: &str, condition: impl Fn(&[JobRow]) -> bool) -> Vec<JobRow> {
    let mut rows = Vec::new();
    let result = tokio::time::timeout(WAIT, async {
        loop {
            rows = app.db.read(inspect::all).await.unwrap();
            if condition(&rows) {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }).await;
    assert!(result.is_ok(), "timed out after {WAIT:?} waiting for {what}: {rows:#?}");
    rows
}

/// Log lines written while `f` runs on this thread (the test runtime's only thread).
struct Logs(Arc<StdMutex<Vec<u8>>>);

impl Logs {
    fn capture() -> (Self, tracing::subscriber::DefaultGuard) {
        let buffer = Arc::new(StdMutex::new(Vec::new()));
        let writer = buffer.clone();
        let subscriber = tracing_subscriber::fmt().with_ansi(false).with_writer(move || LogWriter(writer.clone())).finish();
        (Self(buffer), tracing::subscriber::set_default(subscriber))
    }

    fn text(&self) -> String {
        String::from_utf8_lossy(&self.0.lock().unwrap()).into_owned()
    }
}

struct LogWriter(Arc<StdMutex<Vec<u8>>>);

impl std::io::Write for LogWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

// --- The sink ------------------------------------------------------------------------------------------

#[test]
fn job_events_ask_for_their_rails_job_classes() {
    let request = |event| request_for(&event).map(|request| (request.class, request.arguments));
    assert_eq!(request(Event::PushMessage { room_id: 1, message_id: 2 }), Some(("Room::PushMessageJob", serde_json::json!({"room_id": 1, "message_id": 2}))));
    assert_eq!(request(Event::DeliverWebhook { bot_id: 3, message_id: 4 }), Some(("Bot::WebhookJob", serde_json::json!({"bot_id": 3, "message_id": 4}))));
    assert_eq!(request(Event::RemoveBannedContent { user_id: 5 }), Some(("RemoveBannedContentJob", serde_json::json!({"user_id": 5}))));
    assert_eq!(request(Event::PurgeBlob { blob_id: 6 }), Some(("ActiveStorage::PurgeJob", serde_json::json!({"blob_id": 6}))));
    assert_eq!(request(Event::DisconnectUser { user_id: 7, reconnect: false }), None);
}

/// A job event is a row written in its write's transaction, on its class's queue: it commits,
/// or rolls back, with the write.
#[tokio::test]
async fn job_events_are_enqueued_with_their_write() {
    let (booted, _dir) = app().await;
    let app = booted.app.clone();
    booted.jobs.shutdown(WAIT).await; // leave the rows be

    let rolled_back = app
        .db
        .write(|tx| {
            tx.emit_after_commit(Event::DeliverWebhook { bot_id: 1, message_id: 2 });
            Err::<(), _>(campfire_db::Error::Other("validation failed".into()))
        })
        .await;
    assert!(rolled_back.is_err());
    assert!(jobs(&app).is_empty());

    app.db
        .write(|tx| {
            tx.emit_after_commit(Event::DeliverWebhook { bot_id: 1, message_id: 2 });
            tx.emit_after_commit(Event::PushMessage { room_id: 3, message_id: 2 });
            tx.emit_now(Event::DisconnectUser { user_id: 1, reconnect: true });
            Ok(())
        })
        .await
        .unwrap();
    let rows: Vec<_> = jobs(&app).into_iter().map(|job| (job.queue, job.class, job.arguments, job.status)).collect();
    assert_eq!(
        rows,
        [
            (WEBHOOKS_QUEUE.to_string(), "Bot::WebhookJob".to_string(), serde_json::json!({"bot_id": 1, "message_id": 2}), "ready".to_string()),
            (PUSH_QUEUE.to_string(), "Room::PushMessageJob".to_string(), serde_json::json!({"room_id": 3, "message_id": 2}), "ready".to_string()),
        ]
    );
}

/// Runs `f` in a write while a trigger rejects every new `background_jobs` row.
async fn rejecting_jobs<T: Send + 'static>(app: &App, f: impl FnOnce(&mut Tx<'_>) -> campfire_db::Result<T> + Send + 'static) -> campfire_db::Result<T> {
    const TRIGGER: &str = "CREATE TRIGGER ws3_reject_jobs BEFORE INSERT ON background_jobs BEGIN SELECT RAISE(ABORT, 'jobs rejected'); END";
    app.db.write(|tx| Ok(tx.conn().execute_batch(TRIGGER)?)).await.unwrap();
    let result = app.db.write(f).await;
    app.db.write(|tx| Ok(tx.conn().execute_batch("DROP TRIGGER ws3_reject_jobs")?)).await.unwrap();
    result
}

fn count(app: &App, sql: &'static str) -> i64 {
    app.db.read_blocking(move |conn| Ok(conn.query_row(sql, [], |row| row.get(0))?)).unwrap()
}

/// The classes of the queued jobs, which are then cleared.
async fn take_jobs(app: &App) -> Vec<String> {
    let classes = jobs(app).into_iter().map(|job| job.class).collect();
    app.db.write(|tx| Ok(tx.conn().execute_batch("DELETE FROM background_jobs")?)).await.unwrap();
    classes
}

/// Every path that enqueues one of the migrated jobs, through the app's real sink, writes the
/// job's row in the write's own transaction: when the row can't be written the write fails and
/// none of it commits; when it can, the row commits with the write.
#[tokio::test]
async fn a_job_that_cant_be_enqueued_fails_the_write_that_asks_for_it() {
    use campfire_db::{Message, NewMessage, NewUser, Room, RoomType, User};

    let (booted, _dir) = app().await;
    let app = booted.app.clone();
    booted.jobs.shutdown(WAIT).await; // leave the rows be
    let (storage, now) = (app.storage.clone(), app.clock.now());
    let (author, bot, room, blob) = app
        .db
        .write(move |tx| {
            let author = User::create(tx, NewUser { name: "Author".into(), ..Default::default() })?;
            let bot = User::create_bot(tx, "Bender", Some("https://example.com/hook"))?;
            let room = Room::create_for(tx, RoomType::Closed, Some("Jobs"), author.id, &[author.id, bot.id])?;
            let blob = storage
                .create_and_upload(tx.conn(), b"hello", campfire_storage::Filename::new("hello.txt"), None, now)
                .map_err(|e| campfire_db::Error::Other(e.to_string()))?;
            Ok((author.id, bot.id, room.id, blob.id))
        })
        .await
        .unwrap();
    take_jobs(&app).await;
    let post = move |attachment_blob_id| move |tx: &mut Tx<'_>| {
        Message::create(tx, NewMessage { room_id: room, creator_id: author, body: Some("<div>Hi</div>".into()), attachment_blob_id, ..Default::default() }).map(|message| message.id)
    };

    // Message#receive_in_conversation → Room#push_later
    assert!(rejecting_jobs(&app, post(None)).await.is_err(), "posting a message");
    assert_eq!(count(&app, "SELECT count(*) FROM messages"), 0);
    let message = app.db.write(post(Some(blob))).await.unwrap();
    assert_eq!(count(&app, "SELECT count(*) FROM messages"), 1);
    assert_eq!(take_jobs(&app).await, ["Room::PushMessageJob"]);

    // User::Bot#deliver_webhook_later
    let webhook = move |tx: &mut Tx<'_>| User::find(tx.conn(), bot)?.deliver_webhook_later(tx, message);
    assert!(rejecting_jobs(&app, webhook).await.is_err(), "a webhook");
    app.db.write(webhook).await.unwrap();
    assert_eq!(take_jobs(&app).await, ["Bot::WebhookJob"]);

    // Message#destroy → the attachment's `dependent: :purge_later`
    let destroy = move |tx: &mut Tx<'_>| Message::find(tx.conn(), message)?.destroy(tx);
    assert!(rejecting_jobs(&app, destroy).await.is_err(), "destroying a message");
    assert_eq!(count(&app, "SELECT count(*) FROM messages"), 1);
    app.db.write(destroy).await.unwrap();
    assert_eq!(count(&app, "SELECT count(*) FROM messages"), 0);
    assert_eq!(take_jobs(&app).await, ["ActiveStorage::PurgeJob"]);

    // User::Bannable#ban → apply_ban
    let ban = move |tx: &mut Tx<'_>| User::find(tx.conn(), author)?.ban(tx);
    assert!(rejecting_jobs(&app, ban).await.is_err(), "a ban");
    assert_eq!(count(&app, "SELECT count(*) FROM users WHERE status = 0"), 2, "nobody banned");
    app.db.write(ban).await.unwrap();
    assert_eq!(take_jobs(&app).await, ["RemoveBannedContentJob"]);
}

/// A browser on the booted app's router: a cookie jar, Chrome, and Rails' CSRF tokens.
struct Browser {
    router: axum::Router,
    cookies: std::collections::BTreeMap<String, String>,
    secrets: Arc<rails_compat::Secrets>,
}

impl Browser {
    fn new(router: &axum::Router, secrets: Arc<rails_compat::Secrets>) -> Self {
        Self { router: router.clone(), cookies: Default::default(), secrets }
    }

    async fn get(&mut self, path: &str) -> (axum::http::StatusCode, String) {
        self.send(axum::http::Request::get(path).header("accept", "text/html"), String::new()).await
    }

    async fn post(&mut self, path: &str, accept: &str, fields: &[(&str, &str)]) -> (axum::http::StatusCode, String) {
        let body = fields.iter().map(|(k, v)| format!("{}={}", campfire_presentation::helpers::url::cgi_escape(k), campfire_presentation::helpers::url::cgi_escape(v))).collect::<Vec<_>>().join("&");
        let session = self.cookies.get(campfire_kit::session::SESSION_KEY).expect("a page established the browser's session");
        let token = crate::controllers::presenters::test_support::masked_session_token(&self.secrets, session).expect("the page gave the session a CSRF token");
        let request = axum::http::Request::post(path)
            .header(campfire_kit::csrf::HEADER, token)
            .header("accept", accept)
            .header("content-type", "application/x-www-form-urlencoded");
        self.send(request, body).await
    }

    async fn send(&mut self, request: axum::http::request::Builder, body: String) -> (axum::http::StatusCode, String) {
        use tower::ServiceExt as _;
        let cookie = self.cookies.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join("; ");
        let request = request
            .header("host", "campfire.test")
            .header("user-agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0.0.0 Safari/537.36")
            .header("cookie", cookie)
            .body(axum::body::Body::from(body))
            .unwrap();
        let response = self.router.clone().oneshot(request).await.unwrap();
        for cookie in response.headers().get_all("set-cookie") {
            let (name, value) = cookie.to_str().unwrap().split(';').next().unwrap().split_once('=').unwrap();
            self.cookies.insert(name.to_string(), value.to_string());
        }
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        (status, String::from_utf8_lossy(&body).into_owned())
    }
}

/// Posting a message through `MessagesController#create` enqueues its push and its bots'
/// webhooks in the message's own transaction: a webhook that can't be enqueued fails the post
/// with nothing committed. Committed, the webhooks are held until the message has been broadcast,
/// then released; if releasing them fails (as if the process died), they're delivered when the
/// hold runs out rather than lost.
#[tokio::test]
async fn a_posted_message_and_its_webhooks_commit_together() {
    use campfire_db::{FirstRun, PasswordDigest, Room, RoomType, User};

    let (booted, _dir) = app().await;
    let (app, router) = (booted.app.clone(), booted.router.clone());
    booted.jobs.shutdown(WAIT).await; // leave the rows be
    let digest = PasswordDigest::create("secret123456", 4).unwrap();
    let room = app
        .db
        .write(move |tx| {
            let person = FirstRun::create(tx, "Person", "person@example.com", digest)?;
            let bot = User::create_bot(tx, "Bender", Some("https://example.com/hook"))?;
            Ok(Room::create_for(tx, RoomType::Direct, None, person.id, &[person.id, bot.id])?.id)
        })
        .await
        .unwrap();
    take_jobs(&app).await;
    let mut browser = Browser::new(&router, app.secrets.clone());
    let (status, body) = browser.get("/session/new").await;
    assert_eq!(status, axum::http::StatusCode::OK, "sign-in page: {body}");
    let (status, body) = browser.post("/session", "text/html", &[("email_address", "person@example.com"), ("password", "secret123456")]).await;
    assert_eq!(status, axum::http::StatusCode::FOUND, "signed in: {body}");
    let (status, _) = browser.get("/two_factor_setup").await;
    assert_eq!(status, axum::http::StatusCode::OK);
    let encryption = rails_compat::ar_encryption::ArEncryption::new(&app.secrets);
    let enrollment_now = campfire_db::Timestamp::from_jiff(app.clock.now());
    let secret = app.db.read(move |conn| {
        let session_id = conn.query_row("SELECT session_id FROM two_factor_setup_secrets", [], |r| r.get(0))?;
        campfire_db::TwoFactorSetupSecret::valid_for(conn, session_id, enrollment_now)?.expect("live enrollment").secret(&encryption)
    }).await.unwrap();
    let code = rails_compat::totp::at(&secret, app.clock.now().as_second()).unwrap();
    let (status, body) = browser.post("/two_factor_setup", "text/html", &[("code", &code)]).await;
    assert_eq!(status, axum::http::StatusCode::OK, "completed enrollment: {body}");
    let path = format!("/rooms/{room}/messages");
    let post = |n: &'static str| [("message[body]", "<p>Hello bot</p>"), ("message[client_message_id]", n)];
    let messages = || count(&app, "SELECT count(*) FROM messages");

    const REJECT: &str = "CREATE TRIGGER ws3_reject_webhooks BEFORE INSERT ON background_jobs WHEN NEW.job_class = 'Bot::WebhookJob' BEGIN SELECT RAISE(ABORT, 'webhooks rejected'); END";
    app.db.write(|tx| Ok(tx.conn().execute_batch(REJECT)?)).await.unwrap();
    let (status, _) = browser.post(&path, "text/vnd.turbo-stream.html", &post("rejected")).await;
    assert!(status.is_server_error(), "{status}");
    assert_eq!((messages(), jobs(&app).len()), (0, 0), "the message rolled back with its webhook");
    app.db.write(|tx| Ok(tx.conn().execute_batch("DROP TRIGGER ws3_reject_webhooks")?)).await.unwrap();

    let (status, body) = browser.post(&path, "text/vnd.turbo-stream.html", &post("posted")).await;
    assert_eq!(status, axum::http::StatusCode::CREATED, "{body}");
    assert_eq!(messages(), 1);
    let now = app.db.env().now();
    let mut queued: Vec<_> = jobs(&app).into_iter().map(|job| (job.class, job.run_at <= now)).collect();
    queued.sort();
    assert_eq!(queued, [("Bot::WebhookJob".to_string(), true), ("Room::PushMessageJob".to_string(), true)], "released once broadcast");
    take_jobs(&app).await;

    const KEEP_HELD: &str = "CREATE TRIGGER ws3_keep_webhooks_held BEFORE UPDATE OF run_at ON background_jobs WHEN NEW.job_class = 'Bot::WebhookJob' BEGIN SELECT RAISE(ABORT, 'release rejected'); END";
    app.db.write(|tx| Ok(tx.conn().execute_batch(KEEP_HELD)?)).await.unwrap();
    let (status, body) = browser.post(&path, "text/vnd.turbo-stream.html", &post("held")).await;
    assert_eq!(status, axum::http::StatusCode::CREATED, "{body}");
    assert_eq!(messages(), 2);
    let webhook = jobs(&app).into_iter().find(|job| job.class == "Bot::WebhookJob").expect("the webhook is queued");
    let held = webhook.run_at.as_microsecond() - app.db.env().now().as_microsecond();
    assert!(held > 60_000_000 && held <= WEBHOOK_HOLD.as_micros() as i64, "held for {held} µs");
}

// --- The handlers --------------------------------------------------------------------------------------

/// `discard_on ActiveJob::DeserializationError`: jobs whose records are gone are discarded, not
/// failed or retried.
#[tokio::test]
async fn jobs_whose_records_are_gone_are_discarded() {
    let (booted, _dir) = app().await;
    let (logs, _guard) = Logs::capture();
    let app = booted.app.clone();
    app.db
        .write(|tx| {
            tx.emit_after_commit(Event::DeliverWebhook { bot_id: 404, message_id: 404 });
            tx.emit_after_commit(Event::RemoveBannedContent { user_id: 404 });
            Ok(())
        })
        .await
        .unwrap();
    wait_for(&app, "the jobs to go", |jobs| jobs.is_empty()).await;
    booted.jobs.shutdown(WAIT).await;
    let logs = logs.text();
    assert!(logs.contains(r#"discarded job="Bot::WebhookJob""#), "{logs}");
    assert!(logs.contains(r#"discarded job="RemoveBannedContentJob""#), "{logs}");
}

#[tokio::test]
async fn purging_a_blob_later_deletes_it_and_its_file() {
    let (booted, _dir) = app().await;
    let app = booted.app.clone();
    let (storage, now) = (app.storage.clone(), app.clock.now());
    let blob = app
        .db
        .write(move |tx| {
            let blob = storage
                .create_and_upload(tx.conn(), b"hello", campfire_storage::Filename::new("hello.txt"), None, now)
                .map_err(|e| campfire_db::Error::Other(e.to_string()))?;
            tx.emit_after_commit(Event::PurgeBlob { blob_id: blob.id });
            Ok(blob)
        })
        .await
        .unwrap();
    let path = app.storage.path_for(&blob);
    wait_for(&app, "the purge", |jobs| jobs.is_empty()).await;
    eventually("purged blob file to disappear", || async { !path.exists() }).await;
    let id = blob.id;
    let found = app.db.read(move |conn| Ok(conn.query_row("SELECT count(*) FROM active_storage_blobs WHERE id = ?", [id], |row| row.get::<_, i64>(0))?)).await.unwrap();
    assert_eq!(found, 0);
    booted.jobs.shutdown(WAIT).await;
}

// --- Queues ---------------------------------------------------------------------------------------------

/// A handler that reports each job it performs, after `gate` lets it through (when given).
fn reporting<J: Send + 'static>(performed: mpsc::UnboundedSender<String>, gate: Option<Arc<Notify>>) -> impl Fn(App, J, Execution) -> BoxFuture<'static, JobResult> + Send + Sync + 'static {
    move |_app: App, _job: J, _: Execution| {
        let (performed, gate) = (performed.clone(), gate.clone());
        Box::pin(async move {
            if let Some(gate) = gate {
                gate.notified().await;
            }
            let _ = performed.send(std::any::type_name::<J>().rsplit("::").next().unwrap().to_string());
            Ok(Outcome::Done)
        })
    }
}

async fn next_performed(performed: &mut mpsc::UnboundedReceiver<String>) -> String {
    wait("a job to report execution", performed.recv()).await.unwrap()
}

/// Webhooks to a slow bot fill their own queue's workers, not the ones notifications and the
/// rest run on.
#[tokio::test]
async fn a_busy_queue_doesnt_hold_up_the_others() {
    let (booted, _dir) = app().await;
    let app = booted.app.clone();
    booted.jobs.shutdown(WAIT).await;

    let (performed, mut performed_rx) = mpsc::unbounded_channel();
    let slow_bot = Arc::new(Notify::new());
    let mut registry = Registry::new();
    registry.register(reporting::<WebhookJob>(performed.clone(), Some(slow_bot.clone())));
    registry.register(reporting::<PushMessageJob>(performed.clone(), None));
    registry.register(reporting::<PurgeJob>(performed, None));
    let config = runner_config(&app.config);
    let queue = JobQueue::new(&registry, &config).unwrap();
    let runner = campfire_jobs::start(app.db.clone(), queue.clone(), registry, app.clone(), config);

    for message_id in 1..=10 {
        queue.perform_later(&app.db, JobRequest::new(&WebhookJob { bot_id: 1, message_id })).await.unwrap();
    }
    let workers = app.config.job_concurrency;
    wait_for(&app, "the webhooks queue to fill", |jobs| jobs.iter().filter(|job| job.status == "running").count() == workers).await;
    queue.perform_later(&app.db, JobRequest::new(&PushMessageJob { room_id: 1, message_id: 1 })).await.unwrap();
    queue.perform_later(&app.db, JobRequest::new(&PurgeJob { blob_id: 1 })).await.unwrap();
    let mut others = [next_performed(&mut performed_rx).await, next_performed(&mut performed_rx).await];
    others.sort();
    assert_eq!(others, ["PurgeJob", "PushMessageJob"]);

    for _ in 0..3 {
        slow_bot.notify_one();
        assert_eq!(next_performed(&mut performed_rx).await, "WebhookJob");
    }
    runner.shutdown(Duration::from_millis(100)).await;
    let waiting = jobs(&app);
    assert_eq!(waiting.len(), 7, "the rest wait for the next process");
    assert!(waiting.iter().all(|job| job.status == "ready" && job.class == "Bot::WebhookJob"));
}

// --- Ad hoc jobs ----------------------------------------------------------------------------------------

#[tokio::test]
async fn a_panicking_ad_hoc_job_is_logged_and_its_worker_carries_on() {
    let (booted, _dir) = app().await;
    let (logs, _guard) = Logs::capture();
    let app = booted.app.clone();

    let (done, finished) = tokio::sync::oneshot::channel();
    app.jobs.perform_later("Exploding", async { panic!("kaboom") });
    app.jobs.perform_later("After", async move {
        let _ = done.send(());
        Ok(())
    });
    wait("the ad hoc job after the panic", finished).await.unwrap();
    booted.jobs.shutdown(WAIT).await;

    let logs = logs.text();
    assert!(logs.contains("job panicked job=\"Exploding\" panic=\"kaboom\""), "{logs}");
}

#[tokio::test]
async fn shutdown_performs_the_queued_ad_hoc_jobs_and_then_takes_no_more() {
    let (booted, _dir) = app().await;
    let app = booted.app.clone();
    let (performed, mut performed_rx) = mpsc::unbounded_channel();
    for n in 1..=5 {
        let performed = performed.clone();
        app.jobs.perform_later("Counting", async move {
            let _ = performed.send(n);
            Ok(())
        });
    }
    booted.jobs.shutdown(WAIT).await;
    app.jobs.perform_later("Late", async move {
        let _ = performed.send(6);
        Ok(())
    });
    let mut ns: Vec<i64> = std::iter::from_fn(|| performed_rx.try_recv().ok()).collect();
    ns.sort();
    assert_eq!(ns, [1, 2, 3, 4, 5]);
}

/// Records whether a task finished, or was dropped (aborted) before it did.
#[derive(Clone, Default)]
struct Fate {
    finished: Arc<std::sync::atomic::AtomicBool>,
    dropped: Arc<std::sync::atomic::AtomicBool>,
    release: Arc<Notify>,
}

struct DropFlag(Arc<std::sync::atomic::AtomicBool>);

impl Drop for DropFlag {
    fn drop(&mut self) {
        self.0.store(true, std::sync::atomic::Ordering::SeqCst);
    }
}

impl Fate {
    /// Reports readiness, then waits for an explicit release instead of racing a fixed sleep.
    fn work(&self, started: mpsc::UnboundedSender<()>) -> impl Future<Output = anyhow::Result<()>> + Send + use<> {
        let (finished, guard, release) = (self.finished.clone(), DropFlag(self.dropped.clone()), self.release.clone());
        async move {
            let _guard = guard;
            let released = release.notified();
            tokio::pin!(released);
            released.as_mut().enable();
            let _ = started.send(());
            released.await;
            finished.store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        }
    }

    fn get(&self) -> (bool, bool) {
        (self.finished.load(std::sync::atomic::Ordering::SeqCst), self.dropped.load(std::sync::atomic::Ordering::SeqCst))
    }
}

/// Periodic tasks and ad hoc jobs still running when the grace period ends are aborted, and gone
/// by the time shutdown returns: none carries on detached.
#[tokio::test]
async fn shutdown_aborts_what_outlasts_the_grace_period() {
    let (booted, _dir) = app().await;
    let app = booted.app.clone();
    booted.jobs.shutdown(WAIT).await;

    let (periodic_fate, ad_hoc_fate) = (Fate::default(), Fate::default());
    let (started, mut starts) = mpsc::unbounded_channel();
    let mut loops = periodic::Loops::new(periodic::Intervals::from_lookup(|_| None));
    let (fate, started_by_task) = (periodic_fate.clone(), started.clone());
    loops.periodic.as_mut().unwrap().task(campfire_jobs::periodic::Task::new("slow", Duration::from_secs(60), move |_: App| fate.work(started_by_task.clone())));
    let config = runner_config(&app.config);
    let (jobs, ad_hoc) = Jobs::new(&registry(), &config).unwrap();
    let runner = start(app.clone(), registry(), ad_hoc, config, loops);
    jobs.perform_later("Slow", ad_hoc_fate.work(started));
    for _ in 0..2 {
        wait("slow shutdown task to start", starts.recv()).await.unwrap();
    }

    runner.shutdown(Duration::from_millis(20)).await;
    assert_eq!(periodic_fate.get(), (false, true), "the periodic task was aborted");
    assert_eq!(ad_hoc_fate.get(), (false, true), "the ad hoc job was aborted");
    // The futures have been dropped by shutdown, so releasing them cannot resume work.
    periodic_fate.release.notify_waiters();
    ad_hoc_fate.release.notify_waiters();
    assert_eq!(periodic_fate.get(), (false, true), "and didn't carry on");
    assert_eq!(ad_hoc_fate.get(), (false, true), "and didn't carry on");
}

// --- Periodic -------------------------------------------------------------------------------------------

async fn insert_user(app: &App, name: &str, bot_token: Option<&str>, digest: Option<&str>) -> i64 {
    let (name, bot_token, digest, now) = (name.to_string(), bot_token.map(str::to_string), digest.map(str::to_string), app.db.env().now());
    app.db
        .write(move |tx| {
            Ok(tx.conn().query_row(
                "INSERT INTO users (name, role, bot_token, bot_token_digest, created_at, updated_at) VALUES (?1, 2, ?2, ?3, ?4, ?4) RETURNING id",
                rusqlite::params![name, bot_token, digest, now],
                |row| row.get(0),
            )?)
        })
        .await
        .unwrap()
}

async fn tokens(app: &App, id: i64) -> (Option<String>, Option<String>) {
    app.db.read(move |conn| Ok(conn.query_row("SELECT bot_token, bot_token_digest FROM users WHERE id = ?", [id], |row| Ok((row.get(0)?, row.get(1)?)))?)).await.unwrap()
}

/// `Bots::ClearPlaintextTokens.run!`: the digest recomputed from the plaintext, which is nulled;
/// rows without one untouched.
#[tokio::test]
async fn clearing_plaintext_bot_tokens_heals_their_digests() {
    let (booted, _dir) = app().await;
    let app = booted.app.clone();
    booted.jobs.shutdown(WAIT).await;
    let stale = insert_user(&app, "Stale", Some("BenderToken1"), Some("stale digest")).await;
    let current = insert_user(&app, "Current", None, Some("kept digest")).await;

    assert_eq!(periodic::clear_plaintext_bot_tokens(&app.db).await.unwrap(), 1);
    assert_eq!(tokens(&app, stale).await, (None, Some(campfire_db::user::digest_bot_token("BenderToken1"))));
    assert_eq!(tokens(&app, current).await, (None, Some("kept digest".into())));
    assert_eq!(periodic::clear_plaintext_bot_tokens(&app.db).await.unwrap(), 0, "idempotent");
}

/// The periodic loops run beside the job runner, until it shuts down; the plaintext token task
/// does its work once.
#[tokio::test]
async fn the_periodic_loops_run_with_the_jobs() {
    let (booted, _dir) = app().await;
    let app = booted.app.clone();
    booted.jobs.shutdown(WAIT).await;
    let stale = insert_user(&app, "Stale", Some("BenderToken1"), None).await;

    let (ticked, mut ticks) = mpsc::unbounded_channel();
    let mut loops = periodic::Loops::new(periodic::Intervals::from_lookup(|_| None));
    loops.huddle.as_mut().unwrap().task(campfire_jobs::periodic::Task::new("reconcile", Duration::from_millis(20), move |_: App| {
        let ticked = ticked.clone();
        async move {
            let _ = ticked.send(());
            Ok(())
        }
    }));
    let config = runner_config(&app.config);
    let (_, ad_hoc) = Jobs::new(&registry(), &config).unwrap();
    let runner = start(app.clone(), registry(), ad_hoc, config, loops);
    for _ in 0..3 {
        wait("the huddle loop to tick", ticks.recv()).await.unwrap();
    }
    eventually("periodic plaintext-token cleanup", || async { tokens(&app, stale).await.0.is_none() }).await;
    runner.shutdown(WAIT).await;
    while ticks.try_recv().is_ok() {}
    assert_eq!(wait("huddle sender to close with the runner", ticks.recv()).await, None);
}

#[test]
fn periodic_intervals_come_from_the_environment() {
    let intervals = periodic::Intervals::from_lookup(|name| (name == "EVENT_REMINDERS_INTERVAL").then(|| "10".into()));
    assert_eq!(
        intervals,
        periodic::Intervals {
            periodic: Some(periodic::PeriodicIntervals { reminders: Duration::from_secs(10), retention: Duration::from_secs(86_400) }),
            huddle: Some(Duration::from_secs(5)),
        }
    );
    let intervals = periodic::Intervals::from_lookup(|name| (name == "HUDDLE_RECONCILE_INTERVAL").then(|| "0.5".into()));
    assert_eq!(intervals.huddle, Some(Duration::from_millis(500)));
}

/// An invalid interval disables the loop that reads it, as it aborts the Rails script that
/// reads it: the error is logged, and jobs and the other loop keep running.
#[tokio::test]
async fn an_invalid_interval_disables_only_its_loop() {
    let (booted, _dir) = app().await;
    let app = booted.app.clone();
    booted.jobs.shutdown(WAIT).await;
    let (logs, _guard) = Logs::capture();

    for (name, value) in [("EVENT_REMINDERS_INTERVAL", "0"), ("RETENTION_PRUNE_INTERVAL", "daily"), ("RETENTION_PRUNE_INTERVAL", "inf")] {
        let intervals = periodic::Intervals::from_lookup(|wanted| (wanted == name).then(|| value.into()));
        assert_eq!(intervals.periodic, None, "{name}={value}");
        assert_eq!(intervals.huddle, Some(Duration::from_secs(5)), "{name}={value}");
        let text = logs.text();
        assert!(text.contains(&format!("ERROR campfire::jobs::periodic: Periodic disabled error={name} must be a positive finite number")), "{text}");
    }
    let intervals = periodic::Intervals::from_lookup(|name| (name == "HUDDLE_RECONCILE_INTERVAL").then(|| "-5".into()));
    assert_eq!(intervals.huddle, None);
    assert!(intervals.periodic.is_some());
    let text = logs.text();
    assert!(text.contains("Huddle reconciliation disabled error=HUDDLE_RECONCILE_INTERVAL must be a positive finite number"), "{text}");

    // The huddle loop and the jobs run beside the disabled periodic loop.
    let mut loops = periodic::Loops::new(periodic::Intervals::from_lookup(|name| (name == "RETENTION_PRUNE_INTERVAL").then(|| "0".into())));
    assert!(loops.periodic.is_none());
    let (ticked, mut ticks) = mpsc::unbounded_channel();
    loops.huddle.as_mut().expect("the huddle loop runs").task(campfire_jobs::periodic::Task::new("reconcile", Duration::from_millis(20), move |_: App| {
        let ticked = ticked.clone();
        async move {
            let _ = ticked.send(());
            Ok(())
        }
    }));
    let config = runner_config(&app.config);
    let (_, ad_hoc) = Jobs::new(&registry(), &config).unwrap();
    let runner = start(app.clone(), registry(), ad_hoc, config, loops);
    wait("the huddle loop to tick", ticks.recv()).await.unwrap();
    app.db
        .write(|tx| {
            tx.emit_after_commit(Event::RemoveBannedContent { user_id: 404 });
            Ok(())
        })
        .await
        .unwrap();
    wait_for(&app, "the job to run", |jobs| jobs.is_empty()).await;
    runner.shutdown(WAIT).await;
    let logs = logs.text();
    assert!(logs.contains(r#"discarded job="RemoveBannedContentJob""#), "{logs}");
}

// --- Latency ------------------------------------------------------------------------------------------

#[test]
fn ws8_periodic_tasks_match_rails_names_and_intervals() {
    let golden: serde_json::Value =
        serde_json::from_str(include_str!("../../../runtime/src/ws8_runtime_vectors.json")).unwrap();
    let periodic = periodic::periodic(periodic::PeriodicIntervals {
        reminders: Duration::from_secs(17),
        retention: Duration::from_secs(123),
    });
    let tasks: Vec<_> = periodic
        .tasks()
        .filter(|t| !["clear plaintext bot tokens", "stranded agent webhooks", "streaming messages"].contains(&t.name()))
        .map(|t| serde_json::json!({"name":t.name(),"seconds":t.interval().as_secs()}))
        .collect();
    let ws17: serde_json::Value = serde_json::from_str(include_str!("../../../db/src/tests/ws17_vectors.json")).unwrap();
    let mut expected = golden["tasks"].as_array().unwrap().clone();
    expected.insert(0, serde_json::json!({"name":"event reminders","seconds":17}));
    // Preserve the relative order in the pinned Periodic::Runner for all registered tasks.
    let retention = expected.pop().unwrap();
    expected.push(serde_json::json!({"name":"stuck GitHub claims","seconds":30}));
    expected.push(serde_json::json!({"name":"stuck Fizzy claims","seconds":30}));
    expected.push(serde_json::json!({"name":"slack imports","seconds":30}));
    expected.push(retention);
    expected.push(ws17["presence_task"].clone());
    let calendar: serde_json::Value = serde_json::from_str(include_str!("../../../../vectors/ws17_calendar_dispatch.json")).unwrap();
    expected.extend(calendar["tasks"].as_array().unwrap().iter().filter(|task| matches!(task["name"].as_str(), Some("meeting status" | "out of office"))).cloned());
    let board:serde_json::Value=serde_json::from_str(include_str!("../../../../vectors/board_automations.json")).unwrap();
    expected.extend(board["cadence"].as_array().unwrap().iter().cloned());
    assert_eq!(serde_json::json!(tasks), serde_json::json!(expected));
    let events = periodic.tasks().find(|task| task.name() == "event reminders").unwrap();
    assert_eq!(events.interval(), Duration::from_secs(17));
    let recovery = periodic.tasks().find(|t| t.name() == "stranded agent webhooks").expect("WS11 Rails recovery task");
    assert_eq!(recovery.interval(), Duration::from_secs(30));
    // WS11 tasks have their own fresh, pinned Rails roster, rather than the WS8 subset.
    let ws11:serde_json::Value=serde_json::from_str(include_str!("../../../../vectors/agents_streaming_contract.json")).unwrap();
    let mut tasks:Vec<_>=periodic.tasks().filter(|t|["clear plaintext bot tokens","stranded agent webhooks","streaming messages"].contains(&t.name())).map(|t|serde_json::json!({"name":t.name(),"seconds":t.interval().as_secs()})).collect();
    tasks.sort_by_key(|t|t["name"].as_str().unwrap().to_owned());
    assert_eq!(serde_json::json!(tasks),ws11["results"]["tasks"]);
}

#[tokio::test]
async fn ws8_quote_refresh_jobs_execute_in_the_real_app_runner() {
    let (booted, _dir) = app().await;
    let app = booted.app.clone();
    app.db.write(|tx|{tx.emit_after_commit(Event::job(&campfire_db::models::message_reference::QuoteCardsRefreshJob{source_message_id:999}));assert_eq!(tx.conn().query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class='Message::QuoteCardsRefreshJob'",[],|r|r.get::<_,i64>(0))?,1);Ok(())}).await.unwrap();
    // Wait for the whole queue asserted below, including callbacks from the quote refresh.
    let rows = wait_for(&app, "quote refresh and boot-time maintenance execution", |rows| {
        rows.is_empty() || rows.iter().any(|row| row.status == "failed")
    }).await;
    assert!(rows.is_empty(), "{rows:?}");
    booted.jobs.shutdown(WAIT).await;
}

#[tokio::test]
async fn ws8_quote_refresh_does_not_wait_for_future_maintenance() {
    let (booted, _dir) = app().await;
    let app = booted.app.clone();
    app.db.write(|tx| {
        // Other periodic work can remain queued while the quote job completes.
        tx.emit_after_commit(Event::job_in(Duration::from_secs(3600), &campfire_db::models::retention::PruneJob {}));
        tx.emit_after_commit(Event::job(&campfire_db::models::message_reference::QuoteCardsRefreshJob { source_message_id: 999 }));
        assert_eq!(tx.conn().query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class='Message::QuoteCardsRefreshJob'", [], |row| row.get::<_, i64>(0))?, 1);
        Ok(())
    }).await.unwrap();
    let rows = wait_for(&app, "quote refresh execution", |rows| {
        // Future retention work remains queued by design. Only the quote job
        // must finish; the assertions below also verify the future job survives.
        rows.iter().all(|row| row.class != "Message::QuoteCardsRefreshJob")
            || rows
                .iter()
                .any(|row| row.class == "Message::QuoteCardsRefreshJob" && row.status == "failed")
    })
    .await;
    assert!(rows.iter().all(|row| row.class != "Message::QuoteCardsRefreshJob"), "{rows:?}");
    assert!(rows.iter().any(|row| row.class == "Retention::PruneJob" && row.run_at > campfire_db::Timestamp::from_jiff(app.clock.now())), "{rows:?}");
    booted.jobs.shutdown(WAIT).await;

}



#[test]
fn ws8_thread_unread_broadcasts_match_real_rails_callbacks() {
    use campfire_db::{ChannelThread, Config, Database, Env, Message, NewChannelThread, NewMessage, RecordingSink, ThreadMembership, fixtures};
    let golden: serde_json::Value = serde_json::from_str(include_str!("../../../runtime/src/ws8_runtime_vectors.json")).unwrap();
    let setup = golden["thread_broadcast"].clone();
    let sink = RecordingSink::new();
    let dir = tempfile::tempdir().unwrap();
    let now = campfire_db::Timestamp::parse_db("2026-03-10 12:00:00").unwrap();
    let db = Database::open(Config::new(dir.path().join("thread.sqlite3")), Env { sink: std::sync::Arc::new(sink.clone()), ..Env::default() }).unwrap();
    db.write_blocking(move |tx| {
        fixtures::load(tx.conn(), &fixtures::reference_dir(), &fixtures::Options { now, bcrypt_cost: 4 })?;
        let thread = ChannelThread::create(tx, NewChannelThread {
            room_id: setup["room_id"].as_i64().unwrap(), creator_id: setup["creator_id"].as_i64().unwrap(),
            name: Some(setup["name"].as_str().unwrap().into()), ..Default::default()
        })?;
        ThreadMembership::join(tx, thread.id, setup["recipient_id"].as_i64().unwrap())?;
        Message::create(tx, NewMessage {
            room_id: thread.room_id, creator_id: thread.creator_id, thread_id: Some(thread.id),
            markdown_source: Some(setup["source"].as_str().unwrap().into()), ..Default::default()
        })?;
        Ok(())
    }).unwrap();
    let actual: Vec<_> = sink.events().iter().filter_map(|event| match event.as_broadcast()? {
        event @ campfire_db::broadcasts::Broadcast::Cable { .. } if event.channel_frame().is_some_and(|(stream, _)| stream.ends_with("_unread_threads")) => {
            let (stream, payload) = event.channel_frame().unwrap();
            Some(serde_json::json!({"kind":"cable", "stream":stream, "payload":payload}))
        }
        _ => None,
    }).collect();
    let expected: Vec<_> = golden["broadcasts"].as_array().unwrap().iter().filter(|row| row["kind"] == "cable").cloned().collect();
    assert_eq!(actual, expected);
}

#[tokio::test]
async fn ws8_room_and_retention_workers_run_in_the_real_app() {
    let (booted, _dir) = app().await;
    let app = booted.app.clone();
    app.db.write(|tx| {
        tx.emit_after_commit(Event::job(&campfire_db::models::room_delete::DestroyJob{room_id:999}));
        tx.emit_after_commit(Event::job(&campfire_db::models::retention::PruneJob{}));
        assert_eq!(tx.conn().query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class IN ('Room::DestroyJob','Retention::PruneJob')",[],|r|r.get::<_,i64>(0))?,2);
        Ok(())
    }).await.unwrap();
    let rows = wait_for(&app, "maintenance jobs", |r| {
        r.is_empty() || r.iter().any(|r| r.status == "failed")
    })
    .await;
    assert!(rows.is_empty(), "{rows:?}");
    booted.jobs.shutdown(WAIT).await;
}

#[tokio::test]
async fn ws8_storage_copies_match_rails_and_rollback_on_durable_enqueue_failure() {
    use campfire_db::models::forwarder::{self, Destination};
    use campfire_db::{Message, NewMessage, NewUser, Room, RoomType, User};
    let (booted, _dir) = app().await;
    let app = booted.app.clone();
    booted.jobs.shutdown(WAIT).await;
    let g: serde_json::Value =
        serde_json::from_str(include_str!("../../../runtime/src/ws8_runtime_vectors.json")).unwrap();
    let storage = app.storage.clone();
    let staged = storage
        .stage_bytes(
            b"WS8 copy\n",
            campfire_storage::Filename::new("ws8.txt"),
            Some("text/plain"),
        )
        .unwrap();
    // The source carries metadata that must survive copying. Use the Rails blob fields.
    let mut source = staged.blob().clone();
    source.metadata =
        campfire_storage::Json::parse(&g["attachment_copy"]["metadata"].to_string()).unwrap();
    let source_key = source.key.clone();
    let (message, dest) = app
        .db
        .write(move |tx| {
            let u = User::create(
                tx,
                NewUser {
                    name: "Copy sender".into(),
                    ..Default::default()
                },
            )?;
            let room = Room::create_for(tx, RoomType::Closed, Some("Copy source"), u.id, &[u.id])?;
            let dest = Room::create_for(
                tx,
                RoomType::Closed,
                Some("Copy destination"),
                u.id,
                &[u.id],
            )?;
            let saved = source
                .insert(tx.conn(), tx.now().jiff())
                .map_err(|e| campfire_db::Error::Other(e.to_string()))?;
            crate::active_storage::keep_after_commit(tx, staged);
            let message = Message::create(
                tx,
                NewMessage {
                    creator_id: u.id,
                    room_id: room.id,
                    body: Some("Attachment".into()),
                    attachment_blob_id: Some(saved.id),
                    ..Default::default()
                },
            )?;
            tx.conn().execute_batch("DELETE FROM background_jobs")?;
            Ok((message, dest.id))
        })
        .await
        .unwrap();
    let service = app.storage.service.clone();
    let copier = crate::messaging::ForwarderCopier::new(app.storage.clone());
    let msg = message.clone();
    let copy = app
        .db
        .write(move |tx| {
            Ok(forwarder::forward(
                tx,
                &msg,
                &[Destination::room(dest)],
                None,
                msg.creator_id,
                &copier,
            )?
            .unwrap()
            .remove(0)
            .message)
        })
        .await
        .unwrap();
    let blob = app
        .db
        .read(move |c| Ok(copy.attachment(c)?.unwrap().1))
        .await
        .unwrap();
    assert_ne!(blob.key, source_key);
    assert_eq!(
        service.download(&blob.key).unwrap(),
        service.download(&source_key).unwrap()
    );
    let expected = &g["attachment_copy"];
    assert_eq!(
        serde_json::json!({"filename":blob.filename,"content_type":blob.content_type,"byte_size":blob.byte_size,"checksum":blob.checksum,"metadata":serde_json::from_str::<serde_json::Value>(blob.metadata.as_deref().unwrap()).unwrap()}),
        serde_json::json!({"filename":expected["filename"],"content_type":expected["content_type"],"byte_size":expected["byte_size"],"checksum":expected["checksum"],"metadata":expected["metadata"]})
    );
    let before = app
        .db
        .read(|c| {
            Ok(
                c.query_row::<i64, _, _>("SELECT COUNT(*) FROM active_storage_blobs", [], |r| {
                    r.get(0)
                })?,
            )
        })
        .await
        .unwrap();
    app.db.write(|tx|{tx.conn().execute_batch("DELETE FROM background_jobs; CREATE TRIGGER reject_ws8_copy_job BEFORE INSERT ON background_jobs BEGIN SELECT RAISE(ABORT,'queue unavailable'); END")?;Ok(())}).await.unwrap();
    let copier = crate::messaging::ForwarderCopier::new(app.storage.clone());
    assert!(
        app.db
            .write(move |tx| forwarder::forward(
                tx,
                &message,
                &[Destination::room(dest)],
                None,
                message.creator_id,
                &copier
            ))
            .await
            .is_err()
    );
    let count = app
        .db
        .read(|c| {
            Ok(
                c.query_row::<i64, _, _>("SELECT COUNT(*) FROM active_storage_blobs", [], |r| {
                    r.get(0)
                })?,
            )
        })
        .await
        .unwrap();
    assert_eq!(count, before);
    // The failed copy has no row whose key can be queried; inspect real storage files.
    fn file_count(path: &std::path::Path) -> usize {
        std::fs::read_dir(path)
            .unwrap()
            .map(|e| e.unwrap().path())
            .map(|p| if p.is_dir() { file_count(&p) } else { 1 })
            .sum()
    }
    assert_eq!(file_count(service.root()), before as usize);
    assert!(service.exist(&source_key));
}

#[tokio::test]
async fn ws8_periodic_row_failures_continue_like_rails() {
    let (booted, _dir) = app().await;
    let app = booted.app.clone();
    booted.jobs.shutdown(WAIT).await;
    let g: serde_json::Value =
        serde_json::from_str(include_str!("../ws8_loop_vectors.json")).unwrap();
    let now = campfire_db::Timestamp::parse_db(g["now"].as_str().unwrap()).unwrap();
    let sql = g["setup_sql"].as_str().unwrap().to_owned();
    app.db
        .write(move |tx| {
            campfire_db::fixtures::load(
                tx.conn(),
                &campfire_db::fixtures::reference_dir(),
                &campfire_db::fixtures::Options {
                    now,
                    bcrypt_cost: 4,
                },
            )?;
            tx.conn().execute_batch("PRAGMA defer_foreign_keys=ON")?;
            tx.conn().execute_batch(&sql)?;
            Ok(())
        })
        .await
        .unwrap();
    periodic::saved_item_reminders(&app.db).await.unwrap();
    periodic::scheduled_messages(&app.db).await.unwrap();
    periodic::poll_closing(&app.db).await.unwrap();
    for check in g["checks"].as_array().unwrap() {
        let sql = check["sql"].as_str().unwrap().to_owned();
        let rows = app
            .db
            .read(move |c| {
                let mut stmt = c.prepare(&sql)?;
                let n = stmt.column_count();
                Ok(stmt
                    .query_map([], |r| {
                        (0..n)
                            .map(|i| r.get::<_, i64>(i))
                            .collect::<rusqlite::Result<Vec<_>>>()
                    })?
                    .collect::<rusqlite::Result<Vec<_>>>()?)
            })
            .await
            .unwrap();
        assert_eq!(
            serde_json::to_value(rows).unwrap(),
            check["rows"],
            "{}",
            check["sql"]
        );
    }
}

mod event_tests;

#[test]
fn event_sink_does_not_keep_a_stopped_cable_server_alive() {
    use std::sync::atomic::{AtomicBool, Ordering};
    struct Auth(Arc<AtomicBool>);
    impl Drop for Auth {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }
    #[async_trait::async_trait]
    impl campfire_cable::Authenticate<crate::channels::CableUser> for Auth {
        async fn connect(
            &self,
            _: &campfire_cable::ConnectRequest,
        ) -> Option<crate::channels::CableUser> {
            None
        }
    }
    let dropped = Arc::new(AtomicBool::new(false));
    let config = RunnerConfig::new(vec![campfire_jobs::QueueConfig::new("default", 1)]);
    let (sink, _pending) = Jobs::new(&Registry::new(), &config).unwrap();
    let cable = Cable::builder(campfire_cable::Config::default(), Auth(dropped.clone())).build();
    sink.set_cable(cable.clone());
    assert!(!dropped.load(Ordering::SeqCst));
    drop(cable);
    assert!(
        dropped.load(Ordering::SeqCst),
        "the event sink retained Cable and its database-owning authenticator"
    );
    // A database may finish teardown after the server; publishing then is a no-op.
    sink.emit(Event::DisconnectUser { user_id: -1, reconnect: false });
}
