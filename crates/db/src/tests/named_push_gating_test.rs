//! Exact owned PushGating titles and forwarded-note sequence from the pinned Rails tests.
use super::*;
use crate::models::notification_push::event_reminder_push;
use crate::rich_text::mention_attachment_for;
use crate::{
    ActivityItem, ChannelThread, DndAllowedUser, Involvement, Membership, Message,
    NewChannelThread, NewMessage, PushPayload, PushSubscription, ThreadInvolvement,
    ThreadMembership,
};

fn room_message(t: &TestDb) -> Message {
    t.write(|tx| {
        Message::create(
            tx,
            NewMessage {
                room_id: id("designers"),
                creator_id: id("david"),
                body: Some(format!("Hey {}", mention_attachment_for(id("jason")))),
                client_message_id: Some("named-gating".into()),
                ..Default::default()
            },
        )
    })
}
fn room_push(t: &TestDb, message: &Message) -> (PushPayload, Vec<i64>) {
    t.read(|conn| {
        let (payload, mut everything, mentions) =
            PushSubscription::pushes_for(conn, &BasicRichText, message, t.now())?;
        everything.extend(mentions);
        Ok((payload, everything.into_iter().map(|s| s.user_id).collect()))
    })
}
fn item(t: &TestDb, message: &Message) -> Option<ActivityItem> {
    t.read(|conn| ActivityItem::find_by_user_and_source(conn, id("jason"), "Message", message.id))
}
#[test]
fn ws17_room_push_skips_dnd_but_records_the_inbox_item() {
    let t = TestDb::new();
    t.write(|tx| {
        tx.conn()
            .execute("UPDATE users SET dnd_enabled=1 WHERE id=?", [id("jason")])?;
        Ok(())
    });
    let message = room_message(&t);
    assert!(!room_push(&t, &message).1.contains(&id("jason")));
    assert_eq!(item(&t, &message).unwrap().event_type, "mention");
}
#[test]
fn ws17_room_push_skips_dnd_presence_but_records_the_inbox_item() {
    let t = TestDb::new();
    t.write(|tx| {
        tx.conn().execute(
            "UPDATE users SET presence_setting='dnd' WHERE id=?",
            [id("jason")],
        )?;
        Ok(())
    });
    let message = room_message(&t);
    assert!(!room_push(&t, &message).1.contains(&id("jason")));
    assert_eq!(item(&t, &message).unwrap().event_type, "mention");
}
#[test]
fn ws17_room_push_reaches_a_starred_senders_recipient_during_dnd() {
    let t = TestDb::new();
    t.write(|tx| {
        tx.conn()
            .execute("UPDATE users SET dnd_enabled=1 WHERE id=?", [id("jason")])?;
        DndAllowedUser::create(tx, id("jason"), id("david"))?;
        Ok(())
    });
    assert!(room_push(&t, &room_message(&t)).1.contains(&id("jason")));
}
#[test]
fn ws17_room_push_skips_a_recipient_inside_quiet_hours() {
    let t = TestDb::new();
    t.clock
        .travel_to(crate::Timestamp::parse_db("2026-09-23 12:00:00").unwrap());
    t.write(|tx|{tx.conn().execute("UPDATE users SET quiet_hours_enabled=1,quiet_hours_start_minute=540,quiet_hours_end_minute=1020 WHERE id=?",[id("jason")])?;Ok(())});
    assert!(!room_push(&t, &room_message(&t)).1.contains(&id("jason")));
}
fn thread(t: &TestDb, name: &str, mode: ThreadInvolvement) -> ChannelThread {
    let name = name.to_string();
    t.write(move |tx| {
        let thread = ChannelThread::create(
            tx,
            NewChannelThread {
                room_id: id("designers"),
                creator_id: id("jz"),
                name: Some(name),
                ..Default::default()
            },
        )?;
        ThreadMembership::join(tx, thread.id, id("jz"))?;
        ThreadMembership::join(tx, thread.id, id("jason"))?.update_involvement(tx, mode)?;
        Ok(thread)
    })
}
fn post(
    t: &TestDb,
    thread: &ChannelThread,
    creator: &str,
    text: &str,
    reply: Option<i64>,
) -> Message {
    let mut thread = thread.clone();
    let creator = id(creator);
    let text = text.to_string();
    t.write(move |tx| {
        thread.post_message(
            tx,
            creator,
            NewMessage {
                markdown_source: Some(text),
                reply_to_message_id: reply,
                ..Default::default()
            },
        )
    })
}
fn thread_push(
    t: &TestDb,
    thread: &ChannelThread,
    message: &Message,
) -> Vec<crate::models::channel_thread::ThreadPush> {
    t.read(|conn| {
        ChannelThread::push_recipients_with_policy(
            conn,
            &BasicRichText,
            thread.id,
            message.id,
            t.now(),
        )
    })
}
#[test]
fn ws17_thread_push_notifies_followers_with_the_thread_payload() {
    let t = TestDb::new();
    let thread = thread(&t, "Gated thread", ThreadInvolvement::Everything);
    let message = post(&t, &thread, "jz", "Open here", None);
    let pushes = thread_push(&t, &thread, &message);
    assert_eq!(pushes.len(), 1);
    assert_eq!(pushes[0].payload.title, "Gated thread");
    assert_eq!(
        pushes[0]
            .subscriptions
            .iter()
            .map(|s| s.user_id)
            .collect::<Vec<_>>(),
        [id("jason")]
    );
}
#[test]
fn ws17_thread_push_skips_a_dnd_follower_but_records_thread_activity() {
    let t = TestDb::new();
    t.write(|tx| {
        tx.conn()
            .execute("UPDATE users SET dnd_enabled=1 WHERE id=?", [id("jason")])?;
        Ok(())
    });
    let thread = thread(&t, "DND thread", ThreadInvolvement::Everything);
    let message = post(&t, &thread, "jz", "Quiet update", None);
    assert!(thread_push(&t, &thread, &message).is_empty());
    assert_eq!(item(&t, &message).unwrap().event_type, "thread_activity");
}
#[test]
fn ws17_thread_reply_pushes_a_followed_author_but_not_an_unfollowed_author() {
    let t = TestDb::new();
    let thread = thread(&t, "Reply gating thread", ThreadInvolvement::Mentions);
    t.write(move |tx| {
        ThreadMembership::find_by_thread_and_user(tx.conn(), thread.id, id("jz"))?
            .unwrap()
            .update_involvement(tx, ThreadInvolvement::Everything)
    });
    let source = post(&t, &thread, "jason", "Original", None);
    let reply = post(&t, &thread, "jz", "Answer", Some(source.id));
    assert!(thread_push(&t, &thread, &reply).is_empty());
    assert!(item(&t, &reply).is_none());
    let thread_id = thread.id;
    t.write(move |tx| {
        ThreadMembership::find_by_thread_and_user(tx.conn(), thread_id, id("jason"))?
            .unwrap()
            .update_involvement(tx, ThreadInvolvement::Everything)
    });
    let source = post(&t, &thread, "jason", "Original 2", None);
    let reply = post(&t, &thread, "jz", "Answer 2", Some(source.id));
    let pushes = thread_push(&t, &thread, &reply);
    assert_eq!(pushes.len(), 1);
    assert_eq!(pushes[0].payload.title, "Reply in Reply gating thread");
    assert_eq!(
        pushes[0]
            .subscriptions
            .iter()
            .map(|s| s.user_id)
            .collect::<Vec<_>>(),
        [id("jason")]
    );
    assert_eq!(item(&t, &reply).unwrap().event_type, "reply");
}
#[test]
fn ws17_reminder_push_skips_a_dnd_attendee() {
    let t = TestDb::new();
    t.write(|tx| {
        tx.conn()
            .execute("UPDATE users SET dnd_enabled=1 WHERE id=?", [id("david")])?;
        Ok(())
    });
    let push = t
        .read(|conn| event_reminder_push(conn, id("launch_party"), t.now()))
        .unwrap();
    assert!(!push.subscriptions.iter().any(|s| s.user_id == id("david")));
}
struct NoAttachments;
impl crate::models::forwarder::BlobCopier for NoAttachments {
    fn copy(&self, _: &mut Tx<'_>, _: &crate::Blob) -> Result<crate::Blob> {
        panic!("this original scenario has no attachment")
    }
    fn discard(&self, blobs: &[crate::Blob]) {
        assert!(blobs.is_empty());
    }
}
#[test]
fn ws17_forwarded_note_uses_mention_push_while_snapshot_does_not() {
    let t = TestDb::new();
    t.write(|tx| {
        Membership::find(tx.conn(), id("jason_watercooler"))?
            .update_involvement(tx, Involvement::Mentions)
    });
    let source = t.write(|tx| {
        Message::create(
            tx,
            NewMessage {
                room_id: id("designers"),
                creator_id: id("david"),
                markdown_source: Some("Snapshot @[Jason]".into()),
                client_message_id: Some("forward-push-source".into()),
                ..Default::default()
            },
        )
    });
    let forward = |note: Option<&str>| {
        let source = source.clone();
        let note = note.map(str::to_string);
        t.write(move |tx| {
            crate::models::forwarder::forward(
                tx,
                &source,
                &[crate::models::forwarder::Destination::room(id(
                    "watercooler",
                ))],
                note.as_deref(),
                id("david"),
                &NoAttachments,
            )
        })
        .unwrap()
        .remove(0)
        .message
    };
    let snapshot = forward(None);
    assert!(room_push(&t, &snapshot).1.is_empty());
    let note = forward(Some("@[Jason] please review"));
    let (payload, users) = room_push(&t, &note);
    assert_eq!(payload.title, "All Talk");
    assert_eq!(users, [id("jason")]);
}
