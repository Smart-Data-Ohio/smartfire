use super::*;
use rusqlite::params;
use crate::{Message, NewMessage, Room, User};
use crate::models::bot_webhook_fanout::{self as fanout, HOP_LIMIT};
use super::channel_thread_test::{create_thread, frozen, post_reply};

#[test]
fn ws11_fanout_skips_agents_notes_streams_threads_and_creator() {
    let t = frozen();
    let bot = t.write(|tx| {
        let bot = User::create_bot(tx, "Legacy", Some("https://example.test/hook"))?;
        Room::find(tx.conn(), id("watercooler"))?.grant_to(tx, &[bot.id])?;
        Ok(bot)
    });
    let thread = create_thread(&t, "watercooler", "david", None, Some("Thread"));
    let reply = post_reply(&t, thread.id, "david", "@[Legacy]");
    let r = reply.clone();
    assert!(t.write(move |tx| fanout::recipients(tx, &r)).is_empty());
    for (creator_id, system_note, streaming, expected) in [(id("david"), false, false, 1), (bot.id, false, false, 0), (id("david"), true, false, 0), (id("david"), false, true, 0)] {
        let users = t.write(move |tx| {
            let message = Message::create(tx, NewMessage { room_id: id("watercooler"), creator_id, markdown_source: Some("@[Legacy]".into()), system_note, streaming, ..Default::default() })?;
            fanout::recipients(tx, &message)
        });
        assert_eq!(users.len(), expected);
    }
    t.write(move |tx| {
        tx.conn().execute("UPDATE rooms SET type='Rooms::Direct' WHERE id=?", [id("watercooler")])?;
        let message = Message::create(tx, NewMessage { room_id: id("watercooler"), creator_id: id("david"), body: Some("DM".into()), ..Default::default() })?;
        let recipients = fanout::recipients(tx, &message)?;
        assert_eq!(recipients.iter().map(|bot| bot.id).collect::<Vec<_>>(), [bot.id]);
        Ok(())
    });
}

#[test]
fn ws11_fanout_hops_follow_authorized_agent_events_and_legacy_reply_sources() {
    let t = frozen();
    t.write(|tx| {
        let receiver = User::create_bot(tx, "Receiver", Some("https://example.test/hook"))?;
        let legacy = User::create_bot(tx, "Legacy Sender", None)?;
        Room::find(tx.conn(), id("watercooler"))?.grant_to(tx, &[receiver.id, legacy.id, id("bender")])?;
        let attrs = |creator_id| NewMessage { room_id: id("watercooler"), creator_id, markdown_source: Some("@[Receiver]".into()), ..Default::default() };
        let human = Message::create(tx, attrs(id("david")))?;
        tx.conn().execute("INSERT INTO agent_events (agent_id, message_id, actor_id, event_type, outcome, hop, created_at) VALUES (?, ?, ?, 'mention', 'delivered', ?, ?)", params![id("bender_agent"), human.id, id("david"), HOP_LIMIT-1, tx.now()])?;
        let agent = Message::create(tx, attrs(id("bender")))?;
        assert_eq!(fanout::hop_for_message(tx, &agent)?, HOP_LIMIT);
        assert!(fanout::recipients(tx, &agent)?.is_empty());
        let legacy_reply = Message::create(tx, NewMessage { reply_to_message_id: Some(human.id), ..attrs(legacy.id) })?;
        assert_eq!(fanout::hop_for_message(tx, &legacy_reply)?, HOP_LIMIT);
        assert!(fanout::recipients(tx, &legacy_reply)?.is_empty());
        assert_eq!(fanout::hop_for_message(tx, &human)?, 0);
        // Posted ledger rows, suppression, self-actors and events before the window do not
        // become an agent's authorized trigger, even if their supplied hop is huge.
        tx.conn().execute("DELETE FROM agent_events", [])?;
        for (kind, outcome, actor, age) in [("posted", "delivered", id("david"), 0), ("mention", "suppressed", id("david"), 0), ("mention", "delivered", id("bender"), 0), ("mention", "delivered", id("david"), 301)] {
            tx.conn().execute("INSERT INTO agent_events (agent_id, actor_id, event_type, outcome, hop, created_at) VALUES (?, ?, ?, ?, 99, ?)", params![id("bender_agent"), actor, kind, outcome, tx.now().ago(jiff::SignedDuration::from_secs(age))])?;
        }
        assert_eq!(fanout::hop_for_message(tx, &agent)?, 0);
        assert_eq!(fanout::recipients(tx, &agent)?.len(), 1);
        Ok(())
    });
}
