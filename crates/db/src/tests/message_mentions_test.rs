use super::channel_thread_test::frozen;
use super::*;
use crate::models::activity_item::message_recorder::MentionPushJob;
use crate::rich_text::mention_attachment_for;
use crate::{
    ActivityItem, ChannelThread, KeywordAlert, Message, MessageChanges, NewChannelThread,
    NewMessage, ThreadMembership,
};

fn message(t: &TestDb, event_type: &'static str) -> Message {
    t.write(move |tx| {
        tx.conn().execute_batch(
            "DELETE FROM activity_items; UPDATE memberships SET involvement='mentions',connected_at=NULL; UPDATE users SET dnd_enabled=0,quiet_hours_enabled=0,ooo_until=NULL,presence_setting='auto'",
        )?;
        let reply_to_message_id = if event_type == "reply" {
            Some(Message::create(tx, NewMessage {
                room_id: id("designers"), creator_id: id("david"),
                body: Some("Question".into()), ..Default::default()
            })?.id)
        } else {
            None
        };
        let thread_id = if event_type == "thread_activity" {
            let thread = ChannelThread::create(tx, NewChannelThread {
                room_id: id("designers"), creator_id: id("jz"), name: Some("Mention edits".into()),
                ..Default::default()
            })?;
            for user in [id("jz"), id("david"), id("jason")] {
                ThreadMembership::join(tx, thread.id, user)?.update_involvement(tx, crate::ThreadInvolvement::Everything)?;
            }
            Some(thread.id)
        } else {
            None
        };
        if event_type == "keyword_alert" {
            KeywordAlert::create(tx, id("david"), "deploy")?;
        }
        Message::create(tx, NewMessage {
            room_id: id("designers"), creator_id: id("jz"), thread_id, reply_to_message_id,
            body: Some(format!("deploy {}", mention_attachment_for(id("jason")))),
            ..Default::default()
        })
    })
}

fn item(t: &TestDb, message: &Message, user: &str) -> ActivityItem {
    t.read(|conn| ActivityItem::find_by_user_and_source(conn, id(user), "Message", message.id))
        .unwrap()
}

fn state(t: &TestDb, item: ActivityItem, state: &str) -> ActivityItem {
    match state {
        "read" => t.write(move |tx| item.mark_read(tx)),
        "handled" => t.write(move |tx| item.mark_handled(tx)),
        _ => item,
    }
}

fn edit(t: &TestDb, message: &Message, mention: bool) -> Message {
    let mut message = message.clone();
    t.write(move |tx| {
        let body = format!(
            "deploy {} {}",
            mention_attachment_for(id("jason")),
            if mention {
                mention_attachment_for(id("david"))
            } else {
                String::new()
            }
        );
        message.edit(
            tx,
            MessageChanges {
                body: Some(body),
                ..Default::default()
            },
        )?;
        Ok(message)
    })
}

fn assert_update(t: &TestDb, item: &ActivityItem) {
    let frames: Vec<_> = t
        .events()
        .into_iter()
        .filter_map(|event| match event.as_broadcast()? {
            crate::broadcasts::Broadcast::Cable { stream, payload }
                if stream.ends_with("_activity") =>
            {
                Some((stream, payload))
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        frames,
        [(
            format!("user_{}_activity", item.user_id),
            serde_json::json!({"activityItemId": item.id})
        )]
    );
    assert!(!t.events().iter().any(|event| matches!(event, Event::Broadcast(request) if request.kind == "ActivityItem#sync_removed")));
}

fn mention_job(t: &TestDb) -> MentionPushJob {
    let jobs: Vec<_> = t
        .events()
        .iter()
        .filter_map(|event| event.as_job::<MentionPushJob>())
        .collect();
    assert_eq!(jobs.len(), 1);
    jobs.into_iter().next().unwrap()
}

#[test]
fn mention_edits_reserve_recipients_on_claimed_original_jobs_and_retries() {
    for event_type in ["reply", "thread_activity"] {
        for retry in [false, true] {
            let t = frozen();
            let message = message(&t, event_type);
            let mut arguments = serde_json::json!({"message_id": message.id});
            let class = if let Some(thread_id) = message.thread_id {
                arguments["thread_id"] = serde_json::json!(thread_id);
                "ChannelThread::PushMessageJob"
            } else {
                arguments["room_id"] = serde_json::json!(message.room_id);
                "Room::PushMessageJob"
            };
            if retry {
                arguments = serde_json::json!({"_campfire_retry_metadata_v1": {
                    "arguments": arguments, "counts": {"transient": 1}
                }});
            }
            let stored = arguments.to_string();
            let job_id: i64 = t.write(move |tx| {
                Ok(tx.conn().query_row(
                    "INSERT INTO background_jobs (job_class,arguments,queue_name,status,created_at,updated_at,run_at) VALUES (?,?,'push','running',?,?,?) RETURNING id",
                    rusqlite::params![class, stored, tx.now(), tx.now(), tx.now()],
                    |row| row.get(0),
                )?)
            });
            t.sink.take();
            edit(&t, &message, true);
            assert_eq!(mention_job(&t).recipient_ids, [id("david")]);
            assert_eq!(
                t.read(|conn| MentionPushJob::original_push_exclusions(conn, job_id)),
                [id("david")]
            );
            let recipients = serde_json::json!([id("david")]);
            if retry {
                arguments["_campfire_retry_metadata_v1"]["arguments"]["mention_push_recipient_ids"] = recipients;
            } else {
                arguments["mention_push_recipient_ids"] = recipients;
            }
            let stored: String = t.read(|conn| {
                Ok(conn.query_row(
                    "SELECT arguments FROM background_jobs WHERE id=? AND status='running'",
                    [job_id],
                    |row| row.get(0),
                )?)
            });
            assert_eq!(
                serde_json::from_str::<serde_json::Value>(&stored).unwrap(),
                arguments
            );
        }
    }
}

#[test]
fn mention_edits_promote_existing_activity_and_only_notify_added_recipients() {
    for event_type in ["reply", "thread_activity", "keyword_alert"] {
        for previous_state in ["unread", "read", "handled"] {
            let t = frozen();
            let message = message(&t, event_type);
            let before = item(&t, &message, "david");
            assert_eq!(before.event_type, event_type);
            let before = state(&t, before, previous_state);
            let kept = state(&t, item(&t, &message, "jason"), "handled");
            let unread = t.read(|conn| ActivityItem::unread_snapshot(conn, id("david")));
            t.sink.take();
            t.travel(1);
            let message = edit(&t, &message, true);
            let promoted = item(&t, &message, "david");
            assert_eq!(promoted.id, before.id);
            assert_eq!(promoted.created_at, before.created_at);
            assert_eq!(promoted.updated_at, t.now());
            assert_eq!(promoted.event_type, "mention");
            assert!(promoted.unread());
            assert_eq!(item(&t, &message, "jason"), kept);
            let after = t.read(|conn| ActivityItem::unread_snapshot(conn, id("david")));
            assert_eq!(after.count, unread.count + i64::from(!before.unread()));
            assert!(after.revision > unread.revision);
            assert_update(&t, &promoted);
            let job = mention_job(&t);
            assert_eq!(job.message_id, message.id);
            assert_eq!(job.recipient_ids, [id("david")]);
            let pushes = t.read(|conn| job.deliveries(conn, &BasicRichText, t.now()));
            assert_eq!(pushes.len(), 1);
            assert_eq!(
                pushes[0]
                    .subscriptions
                    .iter()
                    .map(|sub| sub.user_id)
                    .collect::<Vec<_>>(),
                [id("david")]
            );
            t.sink.take();
            t.travel(1);
            let mut again = message.clone();
            t.write(move |tx| {
                again.edit(
                    tx,
                    MessageChanges {
                        body: Some(format!(
                            "Another deploy {} {}",
                            mention_attachment_for(id("david")),
                            mention_attachment_for(id("jason"))
                        )),
                        ..Default::default()
                    },
                )
            });
            assert_eq!(item(&t, &message, "david"), promoted);
            assert_eq!(item(&t, &message, "jason"), kept);
            assert!(!t.events().iter().any(|event| matches!(event.as_broadcast(), Some(crate::broadcasts::Broadcast::Cable { stream, .. }) if stream.ends_with("_activity"))));
            assert!(
                !t.events()
                    .iter()
                    .any(|event| event.as_job::<MentionPushJob>().is_some())
            );
        }
    }
}

#[test]
fn mention_edits_demote_promoted_activity_preserving_current_state() {
    for event_type in ["reply", "thread_activity", "keyword_alert"] {
        for current_state in ["unread", "read", "handled"] {
            let t = frozen();
            let message = message(&t, event_type);
            let message = edit(&t, &message, true);
            let mention = state(&t, item(&t, &message, "david"), current_state);
            let kept = item(&t, &message, "jason");
            let unread = t.read(|conn| ActivityItem::unread_snapshot(conn, id("david")));
            t.sink.take();
            t.travel(1);
            let message = edit(&t, &message, false);
            let demoted = item(&t, &message, "david");
            assert_eq!(
                demoted,
                ActivityItem {
                    event_type: event_type.into(),
                    updated_at: t.now(),
                    ..mention
                }
            );
            assert_eq!(item(&t, &message, "jason"), kept);
            assert_eq!(
                t.read(|conn| ActivityItem::unread_snapshot(conn, id("david")))
                    .count,
                unread.count
            );
            assert_update(&t, &demoted);
            assert!(
                !t.events()
                    .iter()
                    .any(|event| event.as_job::<MentionPushJob>().is_some())
            );
        }
    }
}

#[test]
fn mention_edits_remove_activity_when_its_keyword_no_longer_applies() {
    let t = frozen();
    let message = message(&t, "keyword_alert");
    let mut message = edit(&t, &message, true);
    let mention = item(&t, &message, "david");
    let job = mention_job(&t);
    t.sink.take();
    t.write(move |tx| {
        message.edit(
            tx,
            MessageChanges {
                body: Some("Nothing matching".into()),
                ..Default::default()
            },
        )
    });
    assert!(
        t.read(|conn| ActivityItem::find_by_user_and_source(
            conn,
            id("david"),
            "Message",
            mention.source_id
        ))
        .is_none()
    );
    assert!(
        t.read(|conn| job.deliveries(conn, &BasicRichText, t.now()))
            .is_empty()
    );
}

#[test]
fn mention_edits_push_uses_current_room_and_thread_policy() {
    for event_type in ["reply", "thread_activity"] {
        let t = frozen();
        let message = message(&t, event_type);
        t.sink.take();
        let message = edit(&t, &message, true);
        let job = mention_job(&t);
        t.write(|tx| {
            tx.conn()
                .execute("UPDATE users SET dnd_enabled=1 WHERE id=?", [id("david")])?;
            Ok(())
        });
        assert!(
            t.read(|conn| job.deliveries(conn, &BasicRichText, t.now()))
                .is_empty()
        );
        assert!(item(&t, &message, "david").unread());
        t.write(|tx| crate::DndAllowedUser::create(tx, id("david"), id("jz")));
        assert_eq!(
            t.read(|conn| job.deliveries(conn, &BasicRichText, t.now()))
                .len(),
            1
        );
    }
}
