use super::*;
use crate::models::notification_push::MessageNotified;
use crate::rich_text::mention_attachment_for;
use crate::{
    ChannelThread, Involvement, Membership, Message, MessageChanges, NewChannelThread,
    NewMessage, PushSubscription, Room, ThreadInvolvement, ThreadMembership,
};

fn post(t: &TestDb, room: &str, creator: &str, body: String) -> Message {
    let attributes = NewMessage {
        room_id: id(room),
        creator_id: id(creator),
        body: Some(body),
        ..Default::default()
    };
    t.write(move |tx| Message::create(tx, attributes))
}

fn notified(t: &TestDb, message: &Message) -> Vec<i64> {
    t.read(|c| MessageNotified::user_ids(c, &BasicRichText, message, t.now()))
}

fn emitted(t: &TestDb) -> Vec<i64> {
    t.sink
        .take()
        .into_iter()
        .filter_map(|event| match event {
            Event::Broadcast(request) => request.decode::<MessageNotified>()?.ok(),
            _ => None,
        })
        .map(|notified| notified.message_id)
        .collect()
}

#[test]
fn a_room_message_notifies_members_watching_live_whom_web_push_skips() {
    let t = TestDb::new();
    t.write(|tx| {
        tx.conn().execute(
            "UPDATE memberships SET connected_at=? WHERE room_id=? AND user_id=?",
            rusqlite::params![tx.now(), id("designers"), id("jason")],
        )?;
        Ok(())
    });
    let message = post(&t, "designers", "david", "Hello".into());

    let ids = notified(&t, &message);
    assert!(ids.contains(&id("jason")), "{ids:?}");
    assert!(!ids.contains(&id("david")), "never the author: {ids:?}");
    let pushed: Vec<i64> = t.read(|c| {
        Ok(PushSubscription::pushes_for(c, &BasicRichText, &message, t.now())?
            .1
            .into_iter()
            .map(|s| s.user_id)
            .collect())
    });
    assert!(!pushed.contains(&id("jason")), "web push skips a live viewer: {pushed:?}");
}

#[test]
fn a_mentions_room_notifies_a_real_mention_not_an_id_in_a_code_block() {
    let t = TestDb::new();
    t.write(|tx| {
        Membership::find(tx.conn(), id("jason_designers"))?
            .update_involvement(tx, Involvement::Mentions)
    });
    let jason = id("jason");

    let plain = post(&t, "designers", "david", "Hello".into());
    let code = post(&t, "designers", "david", format!(r#"<pre>data-user-id="{jason}"</pre>"#));
    let mention = post(&t, "designers", "david", format!("Hey {}", mention_attachment_for(jason)));

    assert!(!notified(&t, &plain).contains(&jason));
    assert!(!notified(&t, &code).contains(&jason));
    assert!(notified(&t, &mention).contains(&jason));
}

#[test]
fn every_reply_in_a_followed_thread_notifies_its_follower() {
    let t = TestDb::new();
    let thread = t.write(|tx| {
        let thread = ChannelThread::create(
            tx,
            NewChannelThread {
                room_id: id("designers"),
                creator_id: id("jz"),
                name: Some("Followed".into()),
                ..Default::default()
            },
        )?;
        ThreadMembership::join(tx, thread.id, id("jz"))?;
        ThreadMembership::join(tx, thread.id, id("jason"))?
            .update_involvement(tx, ThreadInvolvement::Everything)?;
        Ok(thread)
    });
    let reply = |text: &str| {
        let mut thread = thread.clone();
        let attributes = NewMessage { markdown_source: Some(text.into()), ..Default::default() };
        t.write(move |tx| thread.post_message(tx, id("jz"), attributes))
    };

    let first = reply("First");
    let second = reply("Second, while the thread is still unread");

    assert_eq!(notified(&t, &first), [id("jason")]);
    assert_eq!(notified(&t, &second), [id("jason")]);
}

#[test]
fn out_of_office_keeps_notifying_when_the_person_kept_notifications_on() {
    let t = TestDb::new();
    let away = |keep: bool| {
        t.write(move |tx| {
            tx.conn().execute(
                "UPDATE users SET ooo_until=?, ooo_notify_enabled=? WHERE id=?",
                rusqlite::params![tx.now().since(jiff::SignedDuration::from_hours(1)), keep, id("jason")],
            )?;
            Ok(())
        })
    };

    away(false);
    assert!(!notified(&t, &post(&t, "designers", "david", "Away".into())).contains(&id("jason")));
    away(true);
    assert!(notified(&t, &post(&t, "designers", "david", "Kept".into())).contains(&id("jason")));
}

#[test]
fn a_create_emits_it_and_an_edit_does_not() {
    let t = TestDb::new();
    t.sink.take();
    let mut message = post(&t, "designers", "david", "Hello".into());
    assert_eq!(emitted(&t), [message.id]);

    let changes = MessageChanges {
        body: Some(format!("Hello {}", mention_attachment_for(id("jason")))),
        ..Default::default()
    };
    t.write(move |tx| message.edit(tx, changes));
    assert_eq!(emitted(&t), Vec::<i64>::new());
}

#[test]
fn a_streaming_message_emits_it_once_at_its_finalize() {
    let t = super::channel_thread_test::frozen();
    t.write(|tx| {
        tx.conn().execute("DELETE FROM agent_grants", [])?;
        Room::find(tx.conn(), id("watercooler"))?.grant_to(tx, &[id("bender")])
    });
    let stream = |text: &'static str| {
        t.write(move |tx| {
            Message::create(
                tx,
                NewMessage {
                    room_id: id("watercooler"),
                    creator_id: id("bender"),
                    markdown_source: Some(text.into()),
                    streaming: true,
                    ..Default::default()
                },
            )
        })
    };
    t.sink.take();

    let mut loud = stream("Thinking");
    assert_eq!(emitted(&t), Vec::<i64>::new(), "the placeholder notifies no one");
    let id = loud.id;
    t.write(move |tx| loud.finalize_stream(tx));
    t.write(move |tx| Message::find(tx.conn(), id)?.finalize_stream(tx));
    assert_eq!(emitted(&t), [id], "once, at the finalize");

    let mut quiet = stream("Never mind");
    t.write(move |tx| quiet.finalize_stream_quietly(tx));
    assert_eq!(emitted(&t), Vec::<i64>::new(), "a quiet finalize notifies no one");
}
