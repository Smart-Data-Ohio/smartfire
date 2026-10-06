use super::*;
use crate::models::{agent_posting::PostResult, agent_streaming as streaming};
use crate::{
    Agent, AgentGrant, ChannelThread, Message, NewChannelThread, NewGrant, NewMessage, Room,
    Timestamp,
};
use rusqlite::params;
use serde_json::{Value, json};
fn setup() -> TestDb {
    let t = super::channel_thread_test::frozen();
    t.clock
        .travel_to(Timestamp::parse_db("2026-03-02 16:00:00").unwrap());
    t.write(|tx| {
        tx.conn().execute("DELETE FROM agent_grants", [])?;
        tx.conn().execute("DELETE FROM agent_events", [])?;
        Room::find(tx.conn(), id("watercooler"))?.grant_to(tx, &[id("bender")])?;
        tx.conn().execute(
            "UPDATE agents SET daily_message_cap=NULL,suspended_at=NULL WHERE id=?",
            [id("bender_agent")],
        )?;
        Ok(())
    });
    t.sink.take();
    t
}
fn attrs(text: &str, client: &str) -> NewMessage {
    NewMessage {
        room_id: id("watercooler"),
        markdown_source: Some(text.into()),
        client_message_id: Some(client.into()),
        ..Default::default()
    }
}
fn capture(key: &str, result: PostResult) -> Option<Message> {
    let (value, message) = match result {
        PostResult::Denied(r) => (
            json!({"status":r.status,"payload":r.payload,"error":r.error}),
            None,
        ),
        PostResult::Posted(m) => (
            json!({"status":if ["start","replay"].contains(&key) {201} else {200},"payload":{"source":m.markdown_source,"streaming":m.streaming,"creator_id":m.creator_id,"edited_at":m.edited_at.map(crate::models::agent_payloads::json_time)},"error":null}),
            Some(*m),
        ),
    };
    let gold: Value = serde_json::from_str(include_str!(
        "../../../../vectors/agents_streaming_contract.json"
    ))
    .unwrap();
    assert_eq!(value, gold["results"][key], "{key}");
    message
}
#[test]
fn ws11_stream_services_precedence_update_replay_and_normal_finalize_match_rails() {
    let t = setup();
    t.write(|tx| {
        let agent=id("bender_agent");let m=capture("start",streaming::start(tx,agent,attrs("","ws11-stream"))?).unwrap();
        capture("append",streaming::update(tx,agent,m.id,Some("First"),None)?);
        capture("replace",streaming::update(tx,agent,m.id,None,Some("Replacement"))?);
        capture("append_wins",streaming::update(tx,agent,m.id,Some("!"),Some("ignored"))?);
        capture("required",streaming::update(tx,agent,m.id,None,None)?);
        capture("replay",streaming::start(tx,agent,attrs("ignored","ws11-stream"))?);
        let thread=ChannelThread::create(tx,NewChannelThread {room_id:m.room_id,creator_id:id("david"),name:Some("Locked".into()),..Default::default()})?;
        tx.conn().execute("UPDATE messages SET thread_id=? WHERE id=?",params![thread.id,m.id])?;
        tx.conn().execute("UPDATE channel_threads SET locked_at=? WHERE id=?",params![tx.now(),thread.id])?;
        capture("locked",streaming::update(tx,agent,m.id,Some("late"),None)?);
        capture("locked_finalize",streaming::finalize(tx,agent,m.id)?);
        let mut grant=AgentGrant::create(tx,NewGrant {agent_id:agent,room_id:Some(m.room_id),capability:"post_messages".into(),granted_by_id:id("david"),..Default::default()})?;grant.revoke(tx)?;
        capture("revoked",streaming::update(tx,agent,m.id,Some("late"),None)?);
        tx.conn().execute("DELETE FROM memberships WHERE room_id=? AND user_id=?",params![m.room_id,id("bender")])?;
        capture("nonmember",streaming::update(tx,agent,m.id,Some("late"),None)?);
        tx.conn().execute("DELETE FROM agent_grants",[])?;Room::find(tx.conn(),m.room_id)?.grant_to(tx,&[id("bender")])?;
        let PostResult::Posted(final_message)=streaming::start(tx,agent,attrs("Final","ws11-final"))? else {panic!("start");};
        Agent::find(tx.conn(),agent)?.unwrap().set_working_presence(tx,Some("Finishing"))?;
        capture("finalized",streaming::finalize(tx,agent,final_message.id)?);
        capture("finalized_again",streaming::finalize(tx,agent,final_message.id)?);
        let effects=json!({"posted":tx.conn().query_row("SELECT COUNT(*) FROM agent_events WHERE agent_id=? AND event_type='posted' AND message_id=?",params![agent,final_message.id],|r|r.get::<_,i64>(0))?,"indexed":tx.conn().query_row("SELECT COUNT(*) FROM message_search_index WHERE rowid=?",[final_message.id],|r|r.get::<_,i64>(0))?,"presence":Agent::find(tx.conn(),agent)?.unwrap().working_presence});
        let gold:Value=serde_json::from_str(include_str!("../../../../vectors/agents_streaming_contract.json")).unwrap();assert_eq!(effects,gold["results"]["final_side_effects"]);Ok(())
    });
}
#[test]
fn ws11_stream_trailing_uses_latest_text_stamp_and_quiet_finalize_stales_jobs() {
    let t = setup();
    let message_id = t.write(|tx| {
        let PostResult::Posted(m) =
            streaming::start(tx, id("bender_agent"), attrs("", "ws11-trailing"))?
        else {
            panic!()
        };
        Ok(m.id)
    });
    t.sink.take();
    t.write(move |tx| {
        streaming::update(tx, id("bender_agent"), message_id, Some("A"), None)?;
        streaming::update(tx, id("bender_agent"), message_id, Some("B"), None)?;
        streaming::update(tx, id("bender_agent"), message_id, Some("C"), None)?;
        Ok(())
    });
    let jobs = t
        .sink
        .take()
        .into_iter()
        .filter_map(|e| match e {
            Event::Job(j) if j.class == "Message::StreamTrailingBroadcastJob" => Some(
                serde_json::from_value::<streaming::StreamTrailingBroadcastJob>(j.arguments)
                    .unwrap(),
            ),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(jobs.len(), 2);
    assert_eq!(jobs[0].last_broadcast_at, jobs[1].last_broadcast_at);
    let gold: Value = serde_json::from_str(include_str!(
        "../../../../vectors/agents_stream_trailing_contract.json"
    ))
    .unwrap();
    assert_eq!(jobs[0].last_broadcast_at, gold["results"]["jobs"][0][1]);
    t.clock
        .travel_to(Timestamp::parse_db("2026-03-02 16:00:00.250000").unwrap());
    t.write(move |tx| {
        assert!(streaming::trailing(tx, &jobs[0])?);
        assert!(!streaming::trailing(tx, &jobs[1])?);
        let mut m = Message::find(tx.conn(), message_id)?;
        assert_eq!(m.markdown_source.as_deref(), Some("ABC"));
        assert!(m.edited_at.is_none());
        m.finalize_stream_quietly(tx)?;
        assert!(!streaming::trailing(tx, &jobs[0])?);
        Ok(())
    });
    assert!(
        !t.sink.take().iter().any(|e| matches!(e, Event::Job(_))),
        "stale jobs cannot enqueue more trailing work"
    );
}
#[test]
fn ws11_stream_overdue_query_backfills_old_rows_and_has_strict_ten_minute_boundary() {
    let t = setup();
    t.write(|tx| {
        for (client, created) in [
            (
                "before",
                tx.now()
                    .ago(streaming::FINALIZE_AFTER)
                    .ago(jiff::SignedDuration::from_micros(1)),
            ),
            ("boundary", tx.now().ago(streaming::FINALIZE_AFTER)),
        ] {
            let PostResult::Posted(m) =
                streaming::start(tx, id("bender_agent"), attrs("", client))?
            else {
                panic!()
            };
            tx.conn().execute(
                "UPDATE messages SET created_at=?,streaming_updated_at=NULL WHERE id=?",
                params![created, m.id],
            )?;
        }
        let ids = streaming::overdue_ids(tx, tx.now())?;
        assert_eq!(ids.len(), 1);
        assert_eq!(
            Message::find(tx.conn(), ids[0])?.client_message_id,
            "before"
        );
        Ok(())
    });
}

#[test]
fn ws11_stream_trailing_frames_and_job_arguments_match_the_actual_rails_job() {
    let t = setup();
    let message_id = 900100001;
    t.write(move |tx| {
        tx.conn().execute("INSERT INTO messages(id,room_id,creator_id,client_message_id,markdown_source,streaming,streaming_updated_at,created_at,updated_at) VALUES (?,?,?,?,?,1,?,?,?)",params![message_id,id("watercooler"),id("bender"),"ws11-trailing-vector","",tx.now(),tx.now(),tx.now()])?;Ok(())
    });
    let mut frames = Vec::new();
    let mut jobs = Vec::new();
    let mut returns = Vec::new();
    for (time, text) in [
        ("2026-03-02 16:00:00", "A"),
        ("2026-03-02 16:00:00.100000", "AB"),
        ("2026-03-02 16:00:00.249000", "ABC"),
    ] {
        t.clock.travel_to(Timestamp::parse_db(time).unwrap());
        returns.push(t.write(move |tx| {
            let mut m = Message::find(tx.conn(), message_id)?;
            m.update(
                tx,
                crate::MessageChanges {
                    markdown_source: Some(text.into()),
                    ..Default::default()
                },
            )?;
            streaming::broadcast_update(tx, &mut m)
        }));
        for event in t.sink.take() {
            match event {
                Event::Broadcast(r) if r.decode::<crate::broadcasts::Broadcast>().is_some() => {
                    frames.push(t.read(move |conn| {
                        let m = Message::find(conn, message_id)?;
                        Ok(json!({"source":m.markdown_source,"streaming":m.streaming}))
                    }))
                }
                Event::Job(j) if j.class == "Message::StreamTrailingBroadcastJob" => jobs.push(
                    serde_json::from_value::<streaming::StreamTrailingBroadcastJob>(j.arguments)
                        .unwrap(),
                ),
                _ => {}
            }
        }
    }
    t.clock
        .travel_to(Timestamp::parse_db("2026-03-02 16:00:00.250000").unwrap());
    let first = jobs[0].clone();
    let last = jobs[1].clone();
    t.write(move |tx| {
        assert!(streaming::trailing(tx, &first)?);
        assert!(!streaming::trailing(tx, &last)?);
        Ok(())
    });
    assert_eq!(t.sink.take().len(), 1);
    frames.push(t.read(move |conn| {
        let m = Message::find(conn, message_id)?;
        Ok(json!({"source":m.markdown_source,"streaming":m.streaming}))
    }));
    let first = jobs[0].clone();
    t.write(move |tx| {
        Message::find(tx.conn(), message_id)?.finalize_stream_quietly(tx)?;
        assert!(!streaming::trailing(tx, &first)?);
        Ok(())
    });
    assert_eq!(t.sink.take().len(), 1);
    frames.push(t.read(move |conn| {
        let m = Message::find(conn, message_id)?;
        Ok(json!({"source":m.markdown_source,"streaming":m.streaming}))
    }));
    let actual = json!({"first":returns[0],"second":returns[1],"third":returns[2],"jobs":jobs.iter().map(|j|json!([j.message_id,j.last_broadcast_at])).collect::<Vec<_>>(),"frames":frames});
    let gold: Value = serde_json::from_str(include_str!(
        "../../../../vectors/agents_stream_trailing_contract.json"
    ))
    .unwrap();
    assert_eq!(actual, gold["results"]);
}
