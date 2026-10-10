use super::*;
use crate::app::{App, AppState};
use crate::controllers::presenters::test_support::TestApp;
use campfire_db::models::notification_push::*;
use campfire_db::{Event, PushPayload};
use serde_json::Value;
fn vectors() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../vectors/ws17_notification_push.json"
    ))
    .unwrap()
}
fn with_pool(original: &App, pool: Pool) -> App {
    Arc::new(AppState {
        fizzy: crate::integrations::fizzy::State::system(),
        google: original.google.clone(),
        errors: original.errors.clone(),
        ar_encryption: original.ar_encryption.clone(),
        agent_message_payload: Default::default(),
        agent_repositories: Default::default(),
        github_accounts: original.github_accounts.clone(),
        github_app: original.github_app.clone(),
        github_read: original.github_read.clone(),
        sudo: Default::default(),
        two_factor: Default::default(),
        slack_network: crate::net::Network::system(),
        subscription_network: original.subscription_network.clone(),
        config: original.config.clone(),
        secrets: original.secrets.clone(),
        clock: original.clock.clone(),
        db: original.db.clone(),
        storage: original.storage.clone(),
        cable: original.cable.clone(),
        broadcasts: original.broadcasts.clone(),
        jobs: original.jobs.clone(),
        mail: crate::mail::State::new(original.mail.config.clone()),
        web_push: Some(pool),
        fragment_cache: original.fragment_cache.clone(),
    })
}

#[tokio::test(flavor = "multi_thread")]
async fn mention_edits_and_pending_original_jobs_deliver_each_recipient_once() {
    use crate::controllers::presenters::test_support::{DAVID, JASON, KEVIN};
    use campfire_db::{
        ActivityItem, ChannelThread, Message, MessageChanges, NewChannelThread, NewMessage,
        ThreadInvolvement, ThreadMembership,
    };
    for (threaded, promoted, edit_first) in [
        (false, false, false), (false, false, true), (false, true, false), (false, true, true),
        (true, false, false), (true, false, true), (true, true, false), (true, true, true),
    ] {
        let test = TestApp::boot_frozen()
            .await
            .expect("mention edits require seeded app tests");
        let original = test.booted.app.clone();
        test.booted.jobs.shutdown(Duration::from_secs(5)).await;
        let db = original.db.clone();
        let service = push_service(201, "Created").await;
        let receiver = Receiver::new();
        let subscription = receiver.subscription(1, "https://fcm.googleapis.com/fcm/send/abc");
        let follower = TestDb::id("jz");
        let message = db.write(move |tx| {
            tx.conn().execute_batch("DELETE FROM activity_items; UPDATE memberships SET involvement='mentions',connected_at=NULL; UPDATE users SET dnd_enabled=0,quiet_hours_enabled=0,ooo_until=NULL,presence_setting='auto'")?;
            tx.conn().execute("DELETE FROM push_subscriptions WHERE id NOT IN (SELECT MIN(id) FROM push_subscriptions GROUP BY user_id)", [])?;
            tx.conn().execute("UPDATE push_subscriptions SET endpoint=?,p256dh_key=?,auth_key=? WHERE user_id=?",
                rusqlite::params![subscription.endpoint, subscription.p256dh_key, subscription.auth_key, DAVID])?;
            tx.conn().execute("UPDATE push_subscriptions SET endpoint='https://fcm.googleapis.com/fcm/send/123',p256dh_key=?,auth_key=? WHERE user_id=?",
                rusqlite::params![subscription.p256dh_key, subscription.auth_key, JASON])?;
            tx.conn().execute("UPDATE push_subscriptions SET endpoint='https://fcm.googleapis.com/fcm/send/456',p256dh_key=?,auth_key=? WHERE user_id=?",
                rusqlite::params![subscription.p256dh_key, subscription.auth_key, follower])?;
            tx.conn().execute("UPDATE memberships SET involvement='everything' WHERE room_id=654632876 AND user_id=?", [follower])?;
            let question = Message::create(tx, NewMessage { room_id: 654632876, creator_id: DAVID, markdown_source: Some("Question".into()), ..Default::default() })?;
            let thread_id = if threaded {
                let thread = ChannelThread::create(tx, NewChannelThread { room_id: question.room_id, creator_id: KEVIN, name: Some("Mention edits".into()), ..Default::default() })?;
                for user in [DAVID, JASON, KEVIN, follower] {
                    let involvement = if user == DAVID && !promoted { ThreadInvolvement::Mentions } else { ThreadInvolvement::Everything };
                    ThreadMembership::join(tx, thread.id, user)?.update_involvement(tx, involvement)?;
                }
                Some(thread.id)
            } else { None };
            tx.conn().execute("DELETE FROM background_jobs", [])?;
            let message = Message::create(tx, NewMessage {
                room_id: question.room_id, creator_id: KEVIN, thread_id,
                reply_to_message_id: (!threaded && promoted).then_some(question.id),
                markdown_source: Some(format!("Answer <@{JASON}>")), ..Default::default()
            })?;
            let activity = ActivityItem::find_by_user_and_source(tx.conn(), DAVID, "Message", message.id)?;
            assert_eq!(activity.is_some(), promoted);
            if let Some(activity) = activity { activity.mark_handled(tx)?; }
            Ok(message)
        }).await.unwrap();
        let message_id = message.id;
        db.write(move |tx| {
            let mut message = message;
            message.edit(
                tx,
                MessageChanges {
                    markdown_source: Some(format!("Answer <@{JASON}> <@{DAVID}>")),
                    ..Default::default()
                },
            )
        })
        .await
        .unwrap();
        let pool = Pool::new(service.net.clone(), vapid(), |_| Ok::<_, String>(()));
        let app = with_pool(&original, pool.clone());
        let original_class = if threaded { "ChannelThread::PushMessageJob" } else { "Room::PushMessageJob" };
        let classes = if edit_first { ["Message::MentionPushJob", original_class] } else { [original_class, "Message::MentionPushJob"] };
        db.write(|tx| {
            tx.conn().execute("UPDATE background_jobs SET run_at='2099-01-01 00:00:00'", [])?;
            Ok(())
        }).await.unwrap();
        let runner = campfire_jobs::start(
            db.clone(),
            app.jobs.queue.clone(),
            crate::jobs::registry(),
            app.clone(),
            crate::queue::runner_config(&app.config),
        );
        for class in classes {
            db.write(move |tx| {
                assert_eq!(tx.conn().execute("UPDATE background_jobs SET run_at=? WHERE job_class=?", rusqlite::params![tx.now(), class])?, 1);
                Ok(())
            }).await.unwrap();
            app.jobs.queue.wake(class);
            wait_for_jobs_and_deliveries(&db, &pool, &[class]).await;
        }
        runner.shutdown(Duration::from_secs(5)).await;
        pool.shutdown().await;
        let requests = service.server.received();
        assert_eq!(
            requests.len(),
            3,
            "original and edit pushes must not overlap, threaded={threaded}, promoted={promoted}, edit_first={edit_first}"
        );
        for endpoint in ["/fcm/send/abc", "/fcm/send/123", "/fcm/send/456"] {
            assert_eq!(requests.iter().filter(|request| request.target == endpoint).count(), 1);
        }
        for request in requests {
            let delivered: Value = serde_json::from_str(&receiver.open(&request.body)).unwrap();
            if threaded {
                assert_eq!(delivered["title"], "Mention edits");
                assert!(
                    delivered["options"]["data"]["path"]
                        .as_str()
                        .unwrap()
                        .contains(&format!("message_id={message_id}"))
                );
            } else {
                assert_eq!(delivered["options"]["data"]["path"], "/rooms/654632876");
            }
            assert!(
                delivered["options"]["body"]
                    .as_str()
                    .unwrap()
                    .contains("Answer")
            );
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn ws17_durable_event_board_and_huddle_jobs_decrypt_complete_rails_json() {
    let test = TestApp::boot_frozen()
        .await
        .expect("WS17 requires seeded app tests");
    let original = test.booted.app.clone();
    test.booted.jobs.shutdown(Duration::from_secs(5)).await;
    let db = original.db.clone();
    let golden = vectors();
    let service = push_service(201, "Created").await;
    let receiver = Receiver::new();
    let pool = Pool::new(service.net.clone(), vapid(), |_| Ok::<_, String>(()));
    let app = with_pool(&original, pool.clone());
    let mut expected = Vec::<Value>::new();
    for name in [
        "event_attendees_still_members",
        "event_inbox_switch_ignored",
        "event_direct_organizer_title",
        "event_three_minutes",
        "event_singular",
        "event_starting_now",
        "event_recently_started",
        "event_stale",
        "board_nudge",
        "invitation_baseline",
        "join_baseline",
    ] {
        let row = golden["rows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["name"] == name)
            .unwrap()
            .clone();
        let setup = row["setup_sql"].as_str().unwrap().to_owned();
        let sub = receiver.subscription(1, "https://fcm.googleapis.com/fcm/send/abc");
        let row_for_job = row.clone();
        db.write(move |tx| {
            tx.conn().execute_batch(&setup)?;
            tx.conn().execute(
                "UPDATE push_subscriptions SET endpoint=?,p256dh_key=?,auth_key=?",
                rusqlite::params![sub.endpoint, sub.p256dh_key, sub.auth_key],
            )?;
            match row_for_job["kind"].as_str().unwrap() {
                "event" => tx.emit_after_commit(Event::job(&EventReminderJob {
                    event_id: row_for_job["event_id"].as_i64().unwrap(),
                })),
                "board" => tx.emit_after_commit(Event::job(&BoardNudgeJob {
                    nudge_id: row_for_job["nudge_id"].as_i64().unwrap(),
                })),
                kind => {
                    let payload: PushPayload =
                        serde_json::from_value(row_for_job["deliveries"][0]["payload"].clone())
                            .unwrap();
                    let room = row_for_job["room_id"].as_i64().unwrap();
                    let recipient = row_for_job["recipient_id"].as_i64().unwrap();
                    let sender = row_for_job["sender_id"].as_i64().unwrap();
                    assert!(if kind == "join" {
                        enqueue_huddle_join(tx, room, recipient, sender, payload)?
                    } else {
                        enqueue_huddle_invitation(tx, room, recipient, sender, payload)?
                    });
                }
            }
            Ok(())
        })
        .await
        .unwrap();
        expected.extend(
            row["deliveries"]
                .as_array()
                .unwrap()
                .iter()
                .flat_map(|d| d["encoded"].as_array().unwrap())
                .map(|s| serde_json::from_str::<Value>(s.as_str().unwrap()).unwrap()),
        );
        let runner = campfire_jobs::start(
            db.clone(),
            app.jobs.queue.clone(),
            crate::jobs::registry(),
            app.clone(),
            crate::queue::runner_config(&app.config),
        );
        wait_for_jobs_and_deliveries(
            &db,
            &pool,
            &[
                "Event::ReminderPushJob",
                "BoardAutomations::NudgePushJob",
                "Huddle::InvitationDeliveryJob",
                "Huddle::JoinDeliveryJob",
            ],
        )
        .await;
        runner.shutdown(Duration::from_secs(5)).await;
    }
    pool.shutdown().await;
    let mut actual = service
        .server
        .received()
        .iter()
        .map(|request| serde_json::from_str::<Value>(&receiver.open(&request.body)).unwrap())
        .collect::<Vec<_>>();
    actual.sort_by_key(Value::to_string);
    expected.sort_by_key(Value::to_string);
    assert_eq!(
        actual, expected,
        "complete decrypted JSON from every new durable handler"
    );
}
#[tokio::test]
async fn ws17_huddle_join_enqueue_failure_rolls_back_throttle_and_source_write() {
    let test = TestApp::boot()
        .await
        .expect("WS17 requires seeded app tests");
    let original = test.booted.app.clone();
    test.booted.jobs.shutdown(Duration::from_secs(5)).await;
    let db = original.db.clone();
    let golden = vectors();
    let row = golden["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["name"] == "join_baseline")
        .unwrap()
        .clone();
    let setup = row["setup_sql"].as_str().unwrap().to_owned();
    db.write(move|tx|{tx.conn().execute_batch(&setup)?;tx.conn().execute_batch("CREATE TRIGGER ws17_reject_huddle_push BEFORE INSERT ON background_jobs WHEN NEW.job_class='Huddle::JoinDeliveryJob' BEGIN SELECT RAISE(ABORT,'WS17 reject delivery'); END")?;Ok(())}).await.unwrap();
    let room = row["room_id"].as_i64().unwrap();
    let recipient = row["recipient_id"].as_i64().unwrap();
    let sender = row["sender_id"].as_i64().unwrap();
    let p: PushPayload = serde_json::from_value(row["deliveries"][0]["payload"].clone()).unwrap();
    let before = db
        .read(move |c| campfire_db::User::find(c, sender))
        .await
        .unwrap()
        .name;
    assert!(
        db.write(move |tx| {
            tx.conn().execute(
                "UPDATE users SET name='WS17 triggering source write' WHERE id=?",
                [sender],
            )?;
            enqueue_huddle_join(tx, room, recipient, sender, p)?;
            Ok(())
        })
        .await
        .is_err()
    );
    let current=db.read(move|c|Ok((campfire_db::User::find(c,sender)?.name,campfire_db::Membership::find_by_room_and_user(c,room,recipient)?.unwrap().last_huddle_join_push_at,c.query_row::<i64,_,_>("SELECT count(*) FROM background_jobs WHERE job_class='Huddle::JoinDeliveryJob'",[],|r|r.get(0))?))).await.unwrap();
    assert_eq!(current, (before, None, 0));
}

#[tokio::test(flavor = "multi_thread")]
async fn ws17_ws13_wire_requests_deliver_captured_payload_and_claim_join_once() {
    let test = TestApp::boot().await.expect("parity seed");
    test.booted.jobs.shutdown(Duration::from_secs(5)).await;
    let original = test.booted.app.clone();
    let db = original.db.clone();
    let golden = vectors();
    let service = push_service(201, "Created").await;
    let receiver = Receiver::new();
    let pool = Pool::new(service.net.clone(), vapid(), |_| Ok::<_, String>(()));
    let app = with_pool(&original, pool.clone());
    let mut expected = Vec::new();
    for (name, kind) in [
        ("invitation_baseline", "huddle"),
        ("join_baseline", "huddle_join"),
    ] {
        let row = golden["rows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["name"] == name)
            .unwrap()
            .clone();
        let setup = row["setup_sql"].as_str().unwrap().to_owned();
        let sub = receiver.subscription(1, "https://fcm.googleapis.com/fcm/send/abc");
        let fixture = row.clone();
        let request=db.write(move|tx| {
            tx.conn().execute_batch(&setup)?;
            tx.conn().execute("UPDATE push_subscriptions SET endpoint=?,p256dh_key=?,auth_key=?",rusqlite::params![sub.endpoint,sub.p256dh_key,sub.auth_key])?;
            let room=fixture["room_id"].as_i64().unwrap();let recipient=fixture["recipient_id"].as_i64().unwrap();
            let membership=campfire_db::Membership::find_by_room_and_user(tx.conn(),room,recipient)?.unwrap();
            // The exact JSON fields and enum spelling emitted by WS13's PushRequest.
            let wire=serde_json::json!({"kind":kind,"recipient_id":recipient,"sender_id":fixture["sender_id"],"room_id":room,"room_membership_id":if kind=="huddle_join" {Some(membership.id)} else {None},"payload":fixture["deliveries"][0]["payload"]});
            let request:HuddlePushRequest=serde_json::from_value(wire.clone()).unwrap();
            assert_eq!(serde_json::to_value(&request).unwrap(),wire);
            tx.emit_after_commit(Event::job(&request));Ok(request)
        }).await.unwrap();
        expected.extend(
            row["deliveries"]
                .as_array()
                .unwrap()
                .iter()
                .flat_map(|d| d["encoded"].as_array().unwrap())
                .map(|s| serde_json::from_str::<Value>(s.as_str().unwrap()).unwrap()),
        );
        let runner = campfire_jobs::start(
            db.clone(),
            app.jobs.queue.clone(),
            crate::jobs::registry(),
            app.clone(),
            crate::queue::runner_config(&app.config),
        );
        let wait = || {
            wait_for_jobs_and_deliveries(
                &db,
                &pool,
                &[
                    "Notifications::HuddlePushJob",
                    "Huddle::InvitationDeliveryJob",
                    "Huddle::JoinDeliveryJob",
                ],
            )
        };
        wait().await;
        if kind == "huddle_join" {
            db.write(move |tx| {
                tx.emit_after_commit(Event::job(&request));
                Ok(())
            })
            .await
            .unwrap();
            wait().await;
        }
        runner.shutdown(Duration::from_secs(5)).await;
    }
    pool.shutdown().await;
    let mut actual = service
        .server
        .received()
        .iter()
        .map(|r| serde_json::from_str::<Value>(&receiver.open(&r.body)).unwrap())
        .collect::<Vec<_>>();
    actual.sort_by_key(Value::to_string);
    expected.sort_by_key(Value::to_string);
    assert_eq!(
        actual, expected,
        "captured source bytes and no second join delivery"
    );
}

async fn cutover_c_reminder_app() -> (
    crate::controllers::presenters::test_support::TestApp,
    App,
    Pool,
    PushService,
    Receiver,
) {
    let test = crate::app::cutover_c_tests::app().await;
    let service = push_service(201, "Created").await;
    let receiver = Receiver::new();
    let pool = Pool::new(service.net.clone(), vapid(), |_| Ok::<_, String>(()));
    let app = with_pool(&test.booted.app, pool.clone());
    let mut sub = receiver.subscription(1, "https://fcm.googleapis.com/fcm/send/abc");
    sub.user_id = campfire_db::fixtures::identify("david");
    app.db
        .write(move |tx| {
            tx.conn().execute("DELETE FROM push_subscriptions", [])?;
            campfire_db::PushSubscription::create(tx, &sub, &|_| Some(PUBLIC_IP.into()))?;
            Ok(())
        })
        .await
        .unwrap();
    (test, app, pool, service, receiver)
}
async fn cutover_c_deliver_reminder(app: &App, pool: &Pool, event_id: i64) {
    app.db
        .write(move |tx| {
            tx.emit_after_commit(Event::job(&EventReminderJob { event_id }));
            Ok(())
        })
        .await
        .unwrap();
    let runner = campfire_jobs::start(
        app.db.clone(),
        app.jobs.queue.clone(),
        crate::jobs::registry(),
        app.clone(),
        crate::queue::runner_config(&app.config),
    );
    wait_for_jobs_and_deliveries(&app.db, pool, &["Event::ReminderPushJob"]).await;
    runner.shutdown(Duration::from_secs(5)).await;
}
#[tokio::test(flavor = "multi_thread")]
async fn cutover_c_reminder_real_job_delivers_venue_suffix_in_encrypted_push() {
    let (_test, app, pool, service, receiver) = cutover_c_reminder_app().await;
    let event_id = campfire_db::fixtures::identify("launch_party");
    let david = campfire_db::fixtures::identify("david");
    app.db
        .write(move |tx| {
            let voice = campfire_db::Room::create_for(
                tx,
                campfire_db::RoomType::Voice,
                Some("Lounge"),
                david,
                &[david],
            )?;
            campfire_db::CalendarEvent::update(
                tx,
                event_id,
                campfire_db::models::calendar_event::changes::EventChanges {
                    venue_room_id: Some(Some(voice.id)),
                    starts_at: Some(Some(tx.now().since(jiff::SignedDuration::from_mins(15)))),
                    ends_at: Some(Some(tx.now().since(jiff::SignedDuration::from_mins(75)))),
                    ..Default::default()
                },
            )?;
            Ok(())
        })
        .await
        .unwrap();
    cutover_c_deliver_reminder(&app, &pool, event_id).await;
    pool.shutdown().await;
    let requests = service.server.received();
    assert_eq!(requests.len(), 1);
    let payload: Value = serde_json::from_str(&receiver.open(&requests[0].body)).unwrap();
    assert_eq!(
        payload["options"]["body"],
        "Starts in 15 minutes: Launch party planning in Lounge"
    );
}
#[tokio::test(flavor = "multi_thread")]
async fn cutover_c_reminder_real_job_suppresses_ended_events_with_recent_running_control() {
    let (_test, app, pool, service, receiver) = cutover_c_reminder_app().await;
    for (start, end) in [(-120, -1), (-3, -1), (-4, 60)] {
        let event = app
            .db
            .write(move |tx| {
                campfire_db::CalendarEvent::create(
                    tx,
                    campfire_db::NewCalendarEvent {
                        room_id: campfire_db::fixtures::identify("designers"),
                        organizer_id: campfire_db::fixtures::identify("david"),
                        title: if end < 0 { "Missed sync" } else { "Quick sync" }.into(),
                        time_zone: "UTC".into(),
                        starts_at: Some(tx.now().since(jiff::SignedDuration::from_mins(start))),
                        ends_at: Some(tx.now().since(jiff::SignedDuration::from_mins(end))),
                        ..Default::default()
                    },
                )
            })
            .await
            .unwrap();
        cutover_c_deliver_reminder(&app, &pool, event.id).await;
        if end < 0 {
            assert!(service.server.received().is_empty());
        }
    }
    pool.shutdown().await;
    let requests = service.server.received();
    assert_eq!(requests.len(), 1);
    let payload: Value = serde_json::from_str(&receiver.open(&requests[0].body)).unwrap();
    assert_eq!(payload["options"]["body"], "Starting now: Quick sync");
}
