//! Run slash dispatch against WS3's SQLite queue, including insert rejection.
use super::*;
use campfire_db::slash_commands::{self, Context};
use campfire_db::{
    ChannelThread, Env, NewChannelThread, NewUser, RecordingSink, Room, RoomType, User,
};
use serde_json::json;
#[derive(Serialize, Deserialize)]
struct RootPush {
    room_id: i64,
    message_id: i64,
}
impl Job for RootPush {
    const CLASS: &'static str = "Room::PushMessageJob";
}
#[derive(Serialize, Deserialize)]
struct LegacyWebhook {
    bot_id: i64,
    message_id: i64,
}
impl Job for LegacyWebhook {
    const CLASS: &'static str = "Bot::WebhookJob";
}
struct SlashSink {
    queue: JobQueue,
    events: RecordingSink,
}
impl EventSink for SlashSink {
    fn emit(&self, event: Event) {
        self.events.emit(event);
    }
    fn persist(&self, tx: &Tx<'_>, event: &Event) -> campfire_db::Result<()> {
        let request = match event {
            Event::Job(request) => Some(request.clone()),
            Event::PushMessage {
                room_id,
                message_id,
            } => Some(JobRequest::new(&RootPush {
                room_id: *room_id,
                message_id: *message_id,
            })),
            Event::DeliverWebhook { bot_id, message_id } => Some(JobRequest::new(&LegacyWebhook {
                bot_id: *bot_id,
                message_id: *message_id,
            })),
            _ => None,
        };
        if let Some(request) = request {
            self.queue.enqueue(tx, &request)?;
        }
        Ok(())
    }
}
fn setup(direct: bool) -> (Database, RecordingSink, Context, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let queue = JobQueue::new(&Registry::<()>::new(), &config()).unwrap();
    let events = RecordingSink::new();
    let sink = Arc::new(SlashSink {
        queue,
        events: events.clone(),
    });
    let db = Database::open(
        campfire_db::Config::new(dir.path().join("slash.sqlite3")),
        Env {
            sink,
            clock: Arc::new(TestClock::frozen_at(at(T0))),
            ..Default::default()
        },
    )
    .unwrap();
    let context = db
        .write_blocking(move |tx| {
            let user = User::create(
                tx,
                NewUser {
                    name: "Invoker".into(),
                    ..Default::default()
                },
            )?;
            let mut members = vec![user.id];
            if direct {
                members.push(User::create_bot(tx, "Legacy", Some("https://example.test/hook"))?.id);
            }
            let room = Room::create_for(
                tx,
                if direct {
                    RoomType::Direct
                } else {
                    RoomType::Closed
                },
                Some("Slash checks"),
                user.id,
                &members,
            )?;
            Ok(Context {
                user_id: user.id,
                room_id: room.id,
                thread_id: None,
                huddles_configured: false,
            })
        })
        .unwrap();
    events.take();
    (db, events, context, dir)
}
fn counts(db: &Database) -> Vec<i64> {
    db.read_blocking(|c| {
        [
            "messages",
            "action_text_rich_texts",
            "saved_items",
            "thread_memberships",
            "background_jobs",
        ]
        .iter()
        .map(|table| Ok(c.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))?))
        .collect()
    })
    .unwrap()
}
fn reject(db: &Database, class: &str) {
    db.write_blocking({let class=class.to_owned();move|tx|{tx.conn().execute_batch(&format!("CREATE TRIGGER reject_slash_job BEFORE INSERT ON background_jobs WHEN NEW.job_class='{class}' BEGIN SELECT RAISE(ABORT,'slash queue failure'); END"))?;Ok(())}}).unwrap();
}
fn allow(db: &Database) {
    db.write_blocking(|tx| {
        tx.conn().execute_batch("DROP TRIGGER reject_slash_job")?;
        Ok(())
    })
    .unwrap();
}
#[test]
fn slash_root_reminder_and_push_are_one_transaction() {
    let (db, events, c, _dir) = setup(false);
    let before = counts(&db);
    reject(&db, "Room::PushMessageJob");
    let action = {
        let c = c.clone();
        move |tx: &mut Tx<'_>| {
            slash_commands::dispatch(tx, &c, "/remind in 20 minutes Review deploy")
        }
    };
    assert!(db.write_blocking(action.clone()).is_err());
    assert_eq!(counts(&db), before);
    assert!(events.events().is_empty());
    allow(&db);
    let result = db.write_blocking(action).unwrap();
    assert_eq!(result.kind, "posted");
    let message = result.message_id.unwrap();
    let jobs = db.read_blocking(inspect::all).unwrap();
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].class, "Room::PushMessageJob");
    assert_eq!(
        jobs[0].arguments,
        json!({"room_id":c.room_id,"message_id":message})
    );
    assert_eq!(counts(&db), vec![1, 1, 1, 0, 1]);
}
#[test]
fn slash_thread_membership_message_and_push_roll_back_together() {
    let (db, events, mut c, _dir) = setup(false);
    let room = c.room_id;
    let user = c.user_id;
    let thread = db
        .write_blocking(move |tx| {
            let thread = ChannelThread::create(
                tx,
                NewChannelThread {
                    room_id: room,
                    creator_id: user,
                    name: Some("Slash thread".into()),
                    ..Default::default()
                },
            )?;
            tx.conn().execute(
                "DELETE FROM thread_memberships WHERE thread_id=?",
                [thread.id],
            )?;
            tx.conn().execute(
                "UPDATE channel_threads SET closed_at=? WHERE id=?",
                rusqlite::params![tx.now(), thread.id],
            )?;
            ChannelThread::find(tx.conn(), thread.id)
        })
        .unwrap();
    c.thread_id = Some(thread.id);
    events.take();
    let before = counts(&db);
    reject(&db, "ChannelThread::PushMessageJob");
    let action = {
        let c = c.clone();
        move |tx: &mut Tx<'_>| slash_commands::dispatch(tx, &c, "/me reviews deploy")
    };
    assert!(db.write_blocking(action.clone()).is_err());
    assert_eq!(counts(&db), before);
    assert!(events.events().is_empty());
    assert_eq!(
        db.read_blocking(|conn| ChannelThread::find(conn, thread.id))
            .unwrap(),
        thread
    );
    allow(&db);
    let result = db.write_blocking(action).unwrap();
    let jobs = db.read_blocking(inspect::all).unwrap();
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].class, "ChannelThread::PushMessageJob");
    assert_eq!(
        jobs[0].arguments,
        json!({"thread_id":thread.id,"message_id":result.message_id.unwrap()})
    );
}
#[test]
fn slash_legacy_webhook_rejection_rolls_back_the_root_post_and_push() {
    let (db, events, c, _dir) = setup(true);
    let before = counts(&db);
    reject(&db, "Bot::WebhookJob");
    let action = move |tx: &mut Tx<'_>| slash_commands::dispatch(tx, &c, "/shrug ship it");
    assert!(db.write_blocking(action.clone()).is_err());
    assert_eq!(counts(&db), before);
    assert!(events.events().is_empty());
    allow(&db);
    let result = db.write_blocking(action).unwrap();
    assert_eq!(result.kind, "posted");
    let mut jobs = db
        .read_blocking(inspect::all)
        .unwrap()
        .into_iter()
        .map(|j| j.class)
        .collect::<Vec<_>>();
    jobs.sort();
    assert_eq!(jobs, vec!["Bot::WebhookJob", "Room::PushMessageJob"]);
}
