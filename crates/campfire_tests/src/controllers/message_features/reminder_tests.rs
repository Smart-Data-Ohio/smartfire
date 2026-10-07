//! Reminder job policy and transport-boundary parity; physical tagged push sending is WS17's.
use crate::controllers::presenters::test_support::*;
use campfire_db::{Message, NewMessage, NewSavedItem, SavedItem};
async fn app() -> TestApp {
    TestApp::boot_with_test_clock(std::sync::Arc::new(campfire_kit::clock::FrozenClock::new(
        SEED_NOW.parse().unwrap(),
    )))
    .await
    .expect("WS8bm2 requires default seed")
}
#[tokio::test]
async fn real_runner_completes_suppressed_reminder_jobs_and_keeps_the_inbox_item() {
    let app = app().await;
    let saved = app
        .db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE users SET dnd_enabled=1,dnd_until=NULL WHERE id=?",
                [DAVID],
            )?;
            let m = Message::create(
                tx,
                NewMessage {
                    room_id: ALL_TALK,
                    creator_id: DAVID,
                    markdown_source: Some("Remind me".into()),
                    ..Default::default()
                },
            )?;
            let item = SavedItem::create(
                tx,
                NewSavedItem {
                    user_id: DAVID,
                    message_id: m.id,
                    ..Default::default()
                },
            )?;
            tx.conn().execute(
                "UPDATE saved_items SET remind_at=? WHERE id=?",
                (tx.now(), item.id),
            )?;
            SavedItem::dispatch_reminder(tx, item.id, tx.now())?;
            assert_eq!(tx.conn().query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class='SavedItem::ReminderPushJob'",[],|r|r.get::<_,i64>(0))?,1);
            Ok(item.id)
        })
        .await
        .unwrap();
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        let statuses=app.db().read(|conn|Ok(conn.prepare("SELECT status FROM background_jobs WHERE job_class='SavedItem::ReminderPushJob'")?.query_map([],|r|r.get::<_,String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?)).await.unwrap();
        if statuses.is_empty() {
            break;
        }
        assert!(
            !statuses.iter().any(|s| s == "failed"),
            "reminder job failed: {statuses:?}"
        );
        assert!(
            tokio::time::Instant::now() < deadline,
            "reminder runner did not finish: {statuses:?}"
        );
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    let count = app
        .db()
        .read(move |conn| {
            Ok(conn.query_row(
                "SELECT COUNT(*) FROM activity_items WHERE source_type='SavedItem' AND source_id=?",
                [saved],
                |r| r.get::<_, i64>(0),
            )?)
        })
        .await
        .unwrap();
    assert_eq!(count, 1);
}
fn fixture() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../../../../../vectors/messaging/reminder_push.json"
    ))
    .unwrap()
}
#[tokio::test]
async fn reminder_policy_matches_pinned_rails_for_every_quiet_gate_and_boundary() {
    let app = app().await;
    let now = campfire_db::Timestamp::from_jiff(SEED_NOW.parse().unwrap());
    for case in fixture()["vectors"].as_array().unwrap() {
        let attrs = case["attrs"].clone();
        let intervals = case["intervals"].to_string();
        let expected = case["allowed"].as_bool().unwrap();
        let attrs_log = attrs.clone();
        app.db().write(move|tx|{
 let fields=attrs.as_object().unwrap();let sql=fields.keys().map(|k|format!("\"{k}\"=?")).collect::<Vec<_>>().join(",");
 let mut values=fields.values().map(|v|match v {serde_json::Value::Null=>rusqlite::types::Value::Null,serde_json::Value::Bool(v)=>rusqlite::types::Value::Integer(i64::from(*v)),serde_json::Value::Number(n)=>rusqlite::types::Value::Integer(n.as_i64().unwrap()),serde_json::Value::String(s)=>rusqlite::types::Value::Text(s.clone()),_=>panic!("fixture {v}")}).collect::<Vec<_>>();values.push(rusqlite::types::Value::Integer(DAVID));tx.conn().execute(&format!("UPDATE users SET {sql} WHERE id=?"),rusqlite::params_from_iter(values))?;
 tx.conn().execute("INSERT INTO calendar_meeting_caches(user_id,busy_intervals,ooo_intervals,created_at,updated_at) VALUES(?,?,?,?,?) ON CONFLICT(user_id) DO UPDATE SET busy_intervals=excluded.busy_intervals,ooo_intervals=excluded.ooo_intervals",rusqlite::params![DAVID,intervals,intervals,tx.now(),tx.now()])?;Ok(())
 }).await.unwrap();
        let allowed = app
            .db()
            .read(move |conn| campfire_db::reminder_policy::allows(conn, DAVID, now))
            .await
            .unwrap();
        assert_eq!(allowed, expected, "{attrs_log}");
    }
}
async fn load_payload(app: &TestApp) -> i64 {
    let data = fixture();
    let rows = data["rows"].clone();
    app.db()
        .write(move |tx| {
            for table in ["messages", "action_text_rich_texts", "saved_items"] {
                for row in rows[table].as_array().unwrap() {
                    let row = row.as_object().unwrap();
                    let columns = row
                        .keys()
                        .map(|k| format!("\"{k}\""))
                        .collect::<Vec<_>>()
                        .join(",");
                    let placeholders = vec!["?"; row.len()].join(",");
                    let values = row.values().map(|v| match v {
                        serde_json::Value::Null => rusqlite::types::Value::Null,
                        serde_json::Value::Number(n) => {
                            rusqlite::types::Value::Integer(n.as_i64().unwrap())
                        }
                        serde_json::Value::String(s) => rusqlite::types::Value::Text(s.clone()),
                        _ => panic!("fixture {v}"),
                    });
                    tx.conn().execute(
                        &format!("INSERT INTO {table} ({columns}) VALUES ({placeholders})"),
                        rusqlite::params_from_iter(values),
                    )?;
                }
            }
            Ok(())
        })
        .await
        .unwrap();
    data["saved_item_id"].as_i64().unwrap()
}
#[tokio::test]
async fn reminder_job_hands_off_the_exact_rails_payload_tag_and_subscriptions() {
    let app = app().await;
    let id = load_payload(&app).await;
    let expected = fixture()["payload"].clone();
    let captured = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let seen = captured.clone();
    let result=crate::jobs::reminders::deliver_with(app.booted.app.clone(),id,move|_,push|{
 let payload=serde_json::json!({"title":push.payload.title,"body":push.payload.body,"path":push.payload.path,"tag":push.tag});
 assert_eq!(payload,expected["payload"]);assert_eq!(push.subscriptions.iter().map(|s|s.id).collect::<Vec<_>>(),expected["subscription_ids"].as_array().unwrap().iter().map(|id|id.as_i64().unwrap()).collect::<Vec<_>>());
 seen.store(true,std::sync::atomic::Ordering::SeqCst);Ok(campfire_jobs::Outcome::Done)
 }).await.unwrap();
    assert!(matches!(result, campfire_jobs::Outcome::Done));
    assert!(captured.load(std::sync::atomic::Ordering::SeqCst));
}
#[tokio::test]
async fn reminder_delivery_rechecks_membership_before_policy_or_source_data() {
    let app = app().await;
    let id = load_payload(&app).await;
    app.db().write(|tx|{tx.conn().execute("DELETE FROM memberships WHERE user_id=? AND room_id=?",(DAVID,ALL_TALK))?;tx.conn().execute("UPDATE users SET quiet_hours_enabled=1,quiet_hours_start_minute=1,quiet_hours_end_minute=2,time_zone='invalid-zone' WHERE id=?",[DAVID])?;Ok(())}).await.unwrap();
    let result = crate::jobs::reminders::deliver_with(app.booted.app.clone(), id, |_, _| {
        panic!("private source leaked to transport")
    })
    .await
    .unwrap();
    assert!(matches!(result, campfire_jobs::Outcome::Done));
}
#[tokio::test]
async fn missing_saved_item_discards_and_transport_errors_do_not_acknowledge_delivery() {
    let app = app().await;
    let id = load_payload(&app).await;
    assert!(matches!(
        crate::jobs::reminders::deliver_with(app.booted.app.clone(), 9999999999, |_, _| panic!(
            "missing saved item delivered"
        ))
        .await,
        Err(campfire_jobs::JobError::Discard(_))
    ));
    let result = crate::jobs::reminders::deliver_with(app.booted.app.clone(), id, |_, _| {
        Err(campfire_db::Error::Other("queue rejected".into()))
    })
    .await;
    assert!(result.is_err());
}
#[tokio::test]
async fn real_runner_completes_allowed_pushes_when_web_push_is_unconfigured() {
    let app = app().await;
    let id = load_payload(&app).await;
    app.db()
        .write(move |tx| {
            tx.emit_after_commit(campfire_db::Event::job(
                &campfire_db::saved_item::ReminderPushJob { saved_item_id: id },
            ));
            Ok(())
        })
        .await
        .unwrap();
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        let rows = app.db().read(campfire_jobs::inspect::all).await.unwrap();
        let Some(row) = rows.iter().find(|row| row.class == "SavedItem::ReminderPushJob") else {
            assert!(app.booted.app.web_push.is_none(), "this case exercises disabled transport");
            assert!(app.db().read(move |conn| SavedItem::find(conn,id)).await.is_ok(), "delivery never removes the saved item");
            break;
        };
        assert_ne!(row.status, "failed", "{row:?}");
        assert!(tokio::time::Instant::now() < deadline, "disabled transport did not finish: {row:?}");
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
}
#[tokio::test]
async fn reminder_payload_preserves_the_complete_unicode_room_title() {
    let app = app().await;
    let id = load_payload(&app).await;
    app.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE rooms SET name=? WHERE id=?",
                ("😀".repeat(80), ALL_TALK),
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let expected = fixture()["long_title_payload"]["payload"].clone();
    crate::jobs::reminders::deliver_with(app.booted.app.clone(),id,move|_,push|{assert_eq!(serde_json::json!({"title":push.payload.title,"body":push.payload.body,"path":push.payload.path,"tag":push.tag}),expected);Ok(campfire_jobs::Outcome::Done)}).await.unwrap();
}
