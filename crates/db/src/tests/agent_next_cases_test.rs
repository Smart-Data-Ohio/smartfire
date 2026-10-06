//! Deferred delivery-hop and kill-switch cases using WS12's installed producers.
use super::*;
use crate::models::{agent_delivery, agent_lifecycle, audit_log};
use crate::{
    Agent, AgentKind, ChannelThread, Message, NewAgent, NewChannelThread, NewMessage, Room,
    RoomType, User,
};
use serde_json::{Value, json};

fn oracle(key: &str) -> Value {
    let value: Value =
        serde_json::from_str(include_str!("../../../../vectors/agents_next_named.json")).unwrap();
    value["results"][key].clone()
}
fn setup() -> TestDb {
    let t = super::channel_thread_test::frozen();
    t.clock
        .travel_to(crate::Timestamp::parse_db("2026-03-02 16:00:00").unwrap());
    t.write(|tx| {
        tx.conn().execute("DELETE FROM agent_events", [])?;
        tx.conn().execute("DELETE FROM agent_grants", [])?;
        Room::find(tx.conn(), id("watercooler"))?.grant_to(tx, &[id("bender")])?;
        Ok(())
    });
    t
}
fn post(tx: &mut Tx<'_>, actor: i64, markdown: &str, key: &str) -> Result<()> {
    Message::create(
        tx,
        NewMessage {
            room_id: id("watercooler"),
            creator_id: actor,
            markdown_source: Some(markdown.into()),
            client_message_id: Some(key.into()),
            ..Default::default()
        },
    )?;
    Ok(())
}
#[test]
fn ws11_next_delivery_self_assigned_work_does_not_raise_hop() {
    let t = setup();
    let result = t.write(|tx| {
        let user = User::create_bot(tx, "Self Hop Bot B", None)?;
        let receiver = Agent::create(tx, NewAgent {user_id: user.id, owner_id: Some(id("david")), kind: AgentKind::Workspace, ..Default::default()})?;
        Room::find(tx.conn(), id("watercooler"))?.grant_to(tx, &[user.id])?;
        let board = Room::create_for(tx, RoomType::Board, Some("Self Hop Board"), id("david"), &[id("david"), id("bender")])?;
        post(tx, id("david"), "Hey @[Bender Bot]", "next-self-hop-trigger")?;
        let event: i64 = tx.conn().query_row("SELECT id FROM agent_events WHERE agent_id=? AND event_type='mention' ORDER BY id DESC LIMIT 1", [id("bender_agent")], |row| row.get(0))?;
        agent_delivery::perform_delivery(tx, event)?;
        let initial_hop = agent_delivery::AgentEvent::find(tx.conn(), event)?.unwrap().hop;
        for index in 0..2 {
            ChannelThread::create_board_post(tx, NewChannelThread {room_id: board.id, creator_id: id("bender"),
                name: Some(format!("Self post {index}")), work_status: Some("in_progress".into()), work_owner_id: Some(id("bender")), ..Default::default()}, None)?;
        }
        post(tx, id("bender"), "Hey @[Self Hop Bot B] help", "next-self-hop-handoff")?;
        let received: i64 = tx.conn().query_row("SELECT id FROM agent_events WHERE agent_id=? AND event_type='mention' ORDER BY id DESC LIMIT 1", [receiver.id], |row| row.get(0))?;
        let received = agent_delivery::AgentEvent::find(tx.conn(), received)?.unwrap();
        Ok(json!({"initial_hop": initial_hop,
            "self_assignments": tx.conn().query_row("SELECT COUNT(*) FROM agent_events WHERE agent_id=? AND event_type='work_assigned'", [id("bender_agent")], |row| row.get::<_,i64>(0))?,
            "receiver_hop": received.hop, "receiver_outcome": received.outcome,
            "suppressed": tx.conn().query_row("SELECT COUNT(*) FROM agent_events WHERE agent_id=? AND event_type='delivery_suppressed_hop_limit'", [receiver.id], |row| row.get::<_,i64>(0))?
        }))
    });
    assert_eq!(result, oracle("self_hop"));
}
#[test]
fn ws11_next_kill_switch_preserves_owned_work_and_history() {
    let t = setup();
    let thread = t.write(|tx| {
        let board = Room::create_for(
            tx,
            RoomType::Board,
            Some("Work Board"),
            id("david"),
            &[id("david"), id("bender")],
        )?;
        crate::AgentGrant::create(
            tx,
            crate::NewGrant {
                agent_id: id("bender_agent"),
                capability: "post_messages".into(),
                room_id: Some(board.id),
                granted_by_id: id("david"),
                ..Default::default()
            },
        )?;
        ChannelThread::create_board_post(
            tx,
            NewChannelThread {
                room_id: board.id,
                creator_id: id("david"),
                name: Some("Ship it".into()),
                work_status: Some("in_progress".into()),
                work_owner_id: Some(id("bender")),
                ..Default::default()
            },
            None,
        )
    });
    let history = t.read(move |conn| {
        Ok(conn.query_row(
            "SELECT COUNT(*) FROM work_thread_events WHERE channel_thread_id=?",
            [thread.id],
            |row| row.get::<_, i64>(0),
        )?)
    });
    t.write(|tx| {
        agent_lifecycle::kill_switch(tx, id("bender_agent"), &audit_log::Context::default())
    });
    let result = t.read(move |conn| {
        let owned = ChannelThread::find(conn, thread.id)?;
        let active = ChannelThread::board_owner_active_map(conn, owned.room_id, [&owned])?[&id("bender")];
        Ok(json!({"owner": owned.work_owner_id, "status": owned.work_status, "active": active,
            "history_before": history, "history_after": conn.query_row("SELECT COUNT(*) FROM work_thread_events WHERE channel_thread_id=?", [thread.id], |row| row.get::<_,i64>(0))?,
            "suspended": Agent::find(conn, id("bender_agent"))?.unwrap().suspended()
        }))
    });
    assert_eq!(result, oracle("kill_owned"));
}

#[test]
fn ws11_next_delivery_rate_check_and_insert_share_writer_lock() {
    let t = setup();
    t.write(|tx| {
        for _ in 0..19 {
            agent_delivery::AgentEvent::create(
                tx,
                agent_delivery::NewEvent {
                    agent_id: id("bender_agent"),
                    room_id: Some(id("watercooler")),
                    actor_id: Some(id("david")),
                    event_type: "mention".into(),
                    outcome: Some("delivered".into()),
                    ..Default::default()
                },
            )?;
        }
        Ok(())
    });
    t.sink.take();
    let other = t.another_process();
    let (first, second) = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            tokio::join!(
                t.db.write(|tx| {
                    let locked = !tx.conn().is_autocommit();
                    post(tx, id("david"), "Hey @[Bender Bot]", "next-rate-lock-0")?;
                    Ok(locked && !tx.conn().is_autocommit())
                }),
                other.write(|tx| {
                    let locked = !tx.conn().is_autocommit();
                    post(tx, id("david"), "Hey @[Bender Bot]", "next-rate-lock-1")?;
                    Ok(locked && !tx.conn().is_autocommit())
                })
            )
        });
    let locked = first.unwrap() && second.unwrap();
    let jobs = t
        .events()
        .iter()
        .filter(|event| event.as_job::<agent_delivery::DeliveryJob>().is_some())
        .count();
    let result = t.read(move |conn| Ok(json!({
        "checks": 2, "locked": locked, "jobs": jobs,
        "eligible": conn.query_row("SELECT COUNT(*) FROM agent_events WHERE agent_id=? AND event_type='mention' AND outcome IN ('pending','delivered','acknowledged')", [id("bender_agent")], |row| row.get::<_,i64>(0))?,
        "suppressed": conn.query_row("SELECT COUNT(*) FROM agent_events WHERE agent_id=? AND event_type='delivery_suppressed_rate_limit'", [id("bender_agent")], |row| row.get::<_,i64>(0))?
    })));
    assert_eq!(result, oracle("rate_lock"));
}
