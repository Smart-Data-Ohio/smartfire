//! Named comparisons from the pinned MessageStreamingTest. Rendered frames,
//! activity fanout, external link synchronization and the sweep stay separate.
use super::*;
use crate::models::{agent_posting::PostResult, agent_streaming as streaming};
use crate::{
    Agent, ChannelThread, Message, MessageChanges, NewChannelThread, NewMessage, Room, User,
};
fn setup() -> TestDb {
    let t = super::channel_thread_test::frozen();
    t.write(|tx| {
        tx.conn().execute("DELETE FROM agent_grants", [])?;
        tx.conn().execute("DELETE FROM agent_events", [])?;
        let legacy = User::create_bot(
            tx,
            "Legacy Stream",
            Some("https://example.test/legacy-stream"),
        )?;
        Room::find(tx.conn(), id("watercooler"))?.grant_to(tx, &[id("bender"), legacy.id])?;
        Ok(())
    });
    t.sink.take();
    t
}
fn create(tx: &mut Tx<'_>, text: &str, thread: Option<i64>) -> Result<Message> {
    Message::create(
        tx,
        NewMessage {
            room_id: id("watercooler"),
            creator_id: id("bender"),
            markdown_source: Some(text.into()),
            thread_id: thread,
            streaming: true,
            ..Default::default()
        },
    )
}
fn thread(tx: &mut Tx<'_>) -> Result<i64> {
    Ok(ChannelThread::create(
        tx,
        NewChannelThread {
            room_id: id("watercooler"),
            creator_id: id("david"),
            name: Some("Stream cases".into()),
            ..Default::default()
        },
    )?
    .id)
}
fn quiet(c: &Connection, mid: i64) -> Result<()> {
    for sql in [
        "SELECT COUNT(*) FROM activity_items WHERE source_type='Message' AND source_id=?",
        "SELECT COUNT(*) FROM agent_events WHERE message_id=?",
        "SELECT COUNT(*) FROM message_search_index WHERE rowid=?",
    ] {
        assert_eq!(c.query_row(sql, [mid], |r| r.get::<_, i64>(0))?, 0);
    }
    Ok(())
}
fn replacements(t: &TestDb, mid: i64) -> usize {
    use crate::broadcasts::{Broadcast, Partial};
    t.events().iter().filter_map(|e|e.as_broadcast()).filter(|b|matches!(b,Broadcast::Turbo(s) if s.partial==Some(Partial::MessageReplace{message_id:mid}))).count()
}
#[test]
fn ws11_stream_case_create_fires_no_side_effects() {
    let t = setup();
    let mid =
        t.write(|tx| Ok(create(tx, "Hey @[David] and @[Legacy Stream] hovercraft", None)?.id));
    assert!(t.events().is_empty());
    t.read(move |c| {
        quiet(c, mid)?;
        assert!(
            crate::Membership::find_by_room_and_user(c, id("watercooler"), id("david"))?
                .unwrap()
                .unread_at
                .is_none()
        );
        Ok(())
    });
}
#[test]
fn ws11_stream_case_thread_finalize_fans_out_no_legacy_webhook() {
    let t = setup();
    let mid = t.write(|tx| {
        let th = thread(tx)?;
        let mut m = create(tx, "Hey @[Legacy Stream]", Some(th))?;
        assert!(m.finalize_stream(tx)?);
        Ok(m.id)
    });
    assert!(
        !t.events()
            .iter()
            .any(|e| matches!(e,Event::Job(j) if j.class=="Bot::WebhookJob"))
    );
    t.read(move |c| {
        assert!(!Message::find(c, mid)?.streaming);
        Ok(())
    });
}
#[test]
fn ws11_stream_case_thread_finalize_sends_no_unread_room_broadcast() {
    let t = setup();
    let mid = t.write(|tx| {
        let th = thread(tx)?;
        Ok(create(tx, "Replying", Some(th))?.id)
    });
    t.sink.take();
    t.write(move |tx| Message::find(tx.conn(), mid)?.finalize_stream(tx));
    assert!(
        !t.events()
            .iter()
            .filter_map(|e| e.as_broadcast())
            .any(|b| b.channel_frame().is_some_and(|(stream, _)| stream.ends_with("_unreads")))
    );
    t.read(move |c| {
        assert!(!Message::find(c, mid)?.streaming);
        Ok(())
    });
}
#[test]
fn ws11_stream_case_broadcasts_coalesce_four_per_second() {
    let t = setup();
    let mid = t.write(|tx| Ok(create(tx, "Ticking", None)?.id));
    t.write(move |tx| {
        let mut m = Message::find(tx.conn(), mid)?;
        assert!(streaming::broadcast_update(tx, &mut m)?);
        for _ in 1..10 {
            assert!(!streaming::broadcast_update(tx, &mut m)?);
        }
        Ok(())
    });
    assert_eq!(replacements(&t, mid), 1);
    t.travel(1);
    t.write(move |tx| {
        assert!(streaming::broadcast_update(
            tx,
            &mut Message::find(tx.conn(), mid)?
        )?);
        Ok(())
    });
    assert_eq!(replacements(&t, mid), 2);
}
#[test]
fn ws11_stream_case_trailing_quiet_after_newer_broadcast() {
    let t = setup();
    let mid = t.write(|tx| Ok(create(tx, "One", None)?.id));
    let job = t.write(move |tx| {
        let mut m = Message::find(tx.conn(), mid)?;
        assert!(streaming::broadcast_update(tx, &mut m)?);
        let job = streaming::StreamTrailingBroadcastJob {
            message_id: mid,
            last_broadcast_at: streaming::stamp(m.stream_broadcast_at.unwrap()),
        };
        assert!(!streaming::broadcast_update(tx, &mut m)?);
        Ok(job)
    });
    t.clock.travel(jiff::SignedDuration::from_millis(350));
    t.write(move |tx| {
        assert!(streaming::broadcast_update(
            tx,
            &mut Message::find(tx.conn(), mid)?
        )?);
        assert!(!streaming::trailing(tx, &job)?);
        Ok(())
    });
    assert_eq!(replacements(&t, mid), 2);
}
#[test]
fn ws11_stream_case_trailing_quiet_after_finalize() {
    let t = setup();
    let mid = t.write(|tx| Ok(create(tx, "One", None)?.id));
    let job = t.write(move |tx| {
        let mut m = Message::find(tx.conn(), mid)?;
        assert!(streaming::broadcast_update(tx, &mut m)?);
        m.update(
            tx,
            MessageChanges {
                markdown_source: Some("One two".into()),
                ..Default::default()
            },
        )?;
        assert!(!streaming::broadcast_update(tx, &mut m)?);
        let job = streaming::StreamTrailingBroadcastJob {
            message_id: mid,
            last_broadcast_at: streaming::stamp(m.stream_broadcast_at.unwrap()),
        };
        assert!(m.finalize_stream(tx)?);
        Ok(job)
    });
    let before = replacements(&t, mid);
    t.write(move |tx| {
        assert!(!streaming::trailing(tx, &job)?);
        Ok(())
    });
    assert_eq!(replacements(&t, mid), before);
}
#[test]
fn ws11_stream_case_start_append_and_replace_stamp_last_activity() {
    let t = setup();
    let mid = t.write(|tx| {
        let m = create(tx, "Draft", None)?;
        assert_eq!(m.streaming_updated_at, Some(tx.now()));
        Ok(m.id)
    });
    t.travel(300);
    t.write(move |tx| {
        let PostResult::Posted(m) =
            streaming::update(tx, id("bender_agent"), mid, Some(" more"), None)?
        else {
            panic!("append");
        };
        assert_eq!(m.streaming_updated_at, Some(tx.now()));
        Ok(())
    });
    t.travel(300);
    t.write(move |tx| {
        let PostResult::Posted(m) =
            streaming::update(tx, id("bender_agent"), mid, None, Some("Replaced"))?
        else {
            panic!("replace");
        };
        assert_eq!(m.streaming_updated_at, Some(tx.now()));
        Ok(())
    });
}
#[test]
fn ws11_stream_case_append_or_replace_after_finalize_refused() {
    let t = setup();
    t.write(|tx| {
        let mut m = create(tx, "Draft", None)?;
        assert!(m.finalize_stream(tx)?);
        for (append, replace) in [(Some("Late"), None), (None, Some("Replaced late"))] {
            let PostResult::Denied(r) =
                streaming::update(tx, id("bender_agent"), m.id, append, replace)?
            else {
                panic!("late update accepted");
            };
            assert_eq!(r.status, 422);
            assert_eq!(r.error.as_deref(), Some("Message is not streaming"));
        }
        let m = Message::find(tx.conn(), m.id)?;
        assert!(!m.streaming);
        assert_eq!(m.markdown_source.as_deref(), Some("Draft"));
        Ok(())
    });
}
#[test]
fn ws11_stream_case_finalized_stream_never_resumes() {
    let t = setup();
    t.write(|tx| {
        let mut m = create(tx, "Draft", None)?;
        assert!(m.finalize_stream(tx)?);
        m.streaming = true;
        let err = m
            .update(
                tx,
                MessageChanges {
                    markdown_source: Some("Illicit resume".into()),
                    ..Default::default()
                },
            )
            .unwrap_err();
        let crate::Error::RecordInvalid(e) = err else {
            panic!("validation error required");
        };
        let vector: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../vectors/agents_stream_resume_contract.json"
        ))
        .unwrap();
        assert_eq!(
            serde_json::json!(
                e.0.iter()
                    .filter(|(k, _)| *k == "streaming")
                    .map(|(_, v)| v)
                    .collect::<Vec<_>>()
            ),
            vector["results"]["errors"]
        );
        let saved = Message::find(tx.conn(), m.id)?;
        assert!(!saved.streaming);
        assert_eq!(saved.markdown_source.as_deref(), Some("Draft"));
        Ok(())
    });
}
#[test]
fn ws11_stream_case_sweep_uses_streaming_activity_index() {
    let t = setup();
    t.write(|tx| {
        let plan = crate::sql::query_all(
            tx.conn(),
            &format!("EXPLAIN QUERY PLAN {}", streaming::OVERDUE_QUERY),
            [tx.now().ago(streaming::FINALIZE_AFTER)],
            |r| r.get::<_, String>(3),
        )?
        .join("\n");
        assert!(
            plan.contains("SEARCH messages USING")
                && plan.contains("INDEX index_messages_on_streaming_updated_at"),
            "{plan}"
        );
        Ok(())
    });
}
#[test]
fn ws11_stream_case_suspended_agent_finalize_quiet() {
    let t = setup();
    t.write(|tx| {
        crate::models::agent_lifecycle::suspend(
            tx,
            id("bender_agent"),
            &crate::models::audit_log::Context::default(),
        )
    });
    let mid =
        t.write(|tx| Ok(create(tx, "Hey @[David] and @[Legacy Stream] hovercraft", None)?.id));
    t.sink.take();
    t.write(move |tx| {
        assert!(Message::find(tx.conn(), mid)?.finalize_stream(tx)?);
        quiet(tx.conn(), mid)?;
        assert!(
            crate::Membership::find_by_room_and_user(tx.conn(), id("watercooler"), id("david"))?
                .unwrap()
                .unread_at
                .is_none()
        );
        Ok(())
    });
    assert!(
        !t.events()
            .iter()
            .any(|e| matches!(e, Event::Job(_) | Event::PushMessage { .. }))
    );
}
#[test]
fn ws11_stream_case_deactivated_agent_finalize_quiet() {
    let t = setup();
    let mid = t.write(|tx| {
        User::find(tx.conn(), id("bender"))?.deactivate(tx)?;
        Ok(create(tx, "Hey @[David] and @[Legacy Stream] hovercraft", None)?.id)
    });
    t.sink.take();
    t.write(move |tx| {
        assert!(Message::find(tx.conn(), mid)?.finalize_stream(tx)?);
        quiet(tx.conn(), mid)
    });
    assert!(
        !t.events()
            .iter()
            .any(|e| matches!(e, Event::Job(_) | Event::PushMessage { .. }))
    );
}
#[test]
fn ws11_stream_case_finalize_clears_working_presence() {
    let t = setup();
    t.write(|tx| {
        let mut a = Agent::find(tx.conn(), id("bender_agent"))?.unwrap();
        a.set_working_presence(tx, Some("Thinking…"))?;
        let mut m = create(tx, "Almost done", None)?;
        assert!(m.finalize_stream(tx)?);
        assert!(
            Agent::find(tx.conn(), a.id)?
                .unwrap()
                .working_presence
                .is_none()
        );
        Ok(())
    });
}
#[test]
fn ws11_stream_case_empty_start_and_finalize_allowed() {
    let t = setup();
    t.write(|tx| {
        let mut m = create(tx, "", None)?;
        assert!(m.streaming);
        assert!(m.finalize_stream(tx)?);
        assert!(!Message::find(tx.conn(), m.id)?.streaming);
        Ok(())
    });
}
