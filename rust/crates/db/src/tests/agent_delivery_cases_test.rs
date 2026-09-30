//! Named domain comparisons from pinned test/jobs/agent/delivery_job_test.rb.
//! Six HTTP/race probes and the WS12 self-assigned-work producer remain deferred.
use super::*;
use crate::models::agent_delivery::{
    self as delivery, AgentEvent, DeliveryJob, EventWebhookJob, NewEvent,
};
use crate::{Agent, AgentGrant, AgentKind, Message, NewAgent, NewGrant, NewMessage, Room, User};
use rusqlite::params;
fn setup() -> TestDb {
    let t = super::channel_thread_test::frozen();
    t.write(|tx| {
        tx.conn().execute("DELETE FROM agent_events", [])?;
        tx.conn().execute("DELETE FROM agent_grants", [])?;
        Room::find(tx.conn(), id("watercooler"))?.grant_to(tx, &[id("bender")])?;
        Ok(())
    });
    t.sink.take();
    t
}
fn post(
    tx: &mut Tx<'_>,
    creator: i64,
    room: i64,
    text: &str,
    reply: Option<i64>,
) -> Result<Message> {
    Message::create(
        tx,
        NewMessage {
            room_id: room,
            creator_id: creator,
            markdown_source: Some(text.into()),
            reply_to_message_id: reply,
            ..Default::default()
        },
    )
}
fn mention(tx: &mut Tx<'_>) -> Result<Message> {
    post(
        tx,
        id("david"),
        id("watercooler"),
        "Hey @[Bender Bot]",
        None,
    )
}
fn last(tx: &Tx<'_>, agent: i64) -> Result<AgentEvent> {
    let eid=tx.conn().query_row("SELECT id FROM agent_events WHERE agent_id=? AND event_type IN ('mention','reply','direct_message') ORDER BY id DESC LIMIT 1",[agent],|r|r.get::<_,i64>(0))?;
    Ok(AgentEvent::find(tx.conn(), eid)?.unwrap())
}
fn count(tx: &Tx<'_>, agent: i64, kind: &str) -> Result<i64> {
    Ok(tx.conn().query_row(
        "SELECT COUNT(*) FROM agent_events WHERE agent_id=? AND event_type=?",
        params![agent, kind],
        |r| r.get(0),
    )?)
}
fn other(tx: &mut Tx<'_>, name: &str) -> Result<(i64, i64)> {
    let mut u = User::create_email_bot(tx)?;
    u.update(
        tx,
        crate::UserChanges {
            name: Some(name.into()),
            ..Default::default()
        },
    )?;
    let a = Agent::create(
        tx,
        NewAgent {
            user_id: u.id,
            owner_id: Some(id("david")),
            kind: AgentKind::Workspace,
            ..Default::default()
        },
    )?;
    Room::find(tx.conn(), id("watercooler"))?.grant_to(tx, &[u.id])?;
    Ok((a.id, u.id))
}
fn chain(tx: &mut Tx<'_>) -> Result<(i64, i64, i64)> {
    let (a, b) = other(tx, "Hop Bot B")?;
    let m1 = mention(tx)?;
    delivery::perform_delivery(tx, last(tx, id("bender_agent"))?.id)?;
    assert_eq!(last(tx, id("bender_agent"))?.hop, 0);
    let m2 = post(
        tx,
        id("bender"),
        m1.room_id,
        "Hey @[Hop Bot B]",
        Some(m1.id),
    )?;
    delivery::perform_delivery(tx, last(tx, a)?.id)?;
    assert_eq!(last(tx, a)?.hop, 1);
    let m3 = post(tx, b, m1.room_id, "Hey @[Bender Bot]", Some(m2.id))?;
    delivery::perform_delivery(tx, last(tx, id("bender_agent"))?.id)?;
    assert_eq!(last(tx, id("bender_agent"))?.hop, 2);
    Ok((a, m1.id, m3.id))
}

#[test]
fn ws11_delivery_case_mentioning_an_agent_enqueues_delivery_and_performing_marks_it_delivered() {
    let t = setup();
    let event = t.write(|tx| {
        mention(tx)?;
        let e = last(tx, id("bender_agent"))?;
        assert_eq!(e.event_type, "mention");
        assert_eq!(e.outcome.as_deref(), Some("pending"));
        assert_eq!(e.hop, 0);
        Ok(e.id)
    });
    assert_eq!(
        t.events()
            .iter()
            .filter_map(|e| e.as_job::<DeliveryJob>())
            .count(),
        1
    );
    t.write(move |tx| {
        delivery::perform_delivery(tx, event)?;
        assert_eq!(
            AgentEvent::find(tx.conn(), event)?
                .unwrap()
                .outcome
                .as_deref(),
            Some("delivered")
        );
        Ok(())
    });
}

#[test]
fn ws11_delivery_case_delivery_without_a_webhook_still_marks_the_row_delivered() {
    let t = setup();
    t.write(|tx| {
        tx.conn()
            .execute("DELETE FROM webhooks WHERE user_id=?", [id("bender")])?;
        mention(tx)?;
        let e = last(tx, id("bender_agent"))?;
        delivery::perform_delivery(tx, e.id)?;
        let e = AgentEvent::find(tx.conn(), e.id)?.unwrap();
        assert_eq!(e.outcome.as_deref(), Some("delivered"));
        assert_eq!(e.webhook_status, "none");
        Ok(())
    });
    assert_eq!(
        t.events()
            .iter()
            .filter_map(|e| e.as_job::<EventWebhookJob>())
            .count(),
        0
    );
}

#[test]
fn ws11_delivery_case_revoked_at_perform_time_writes_a_suppression_row() {
    let t = setup();
    t.write(|tx| {
        let mut g = AgentGrant::create(
            tx,
            NewGrant {
                agent_id: id("bender_agent"),
                room_id: Some(id("watercooler")),
                granted_by_id: id("david"),
                capability: "read_messages".into(),
                ..Default::default()
            },
        )?;
        let m = mention(tx)?;
        let e = last(tx, id("bender_agent"))?;
        assert_eq!(e.outcome.as_deref(), Some("pending"));
        g.revoke(tx)?;
        delivery::perform_delivery(tx, e.id)?;
        assert_eq!(last(tx, e.agent_id)?.outcome.as_deref(), Some("suppressed"));
        assert_eq!(count(tx, e.agent_id, "delivery_suppressed_revoked")?, 1);
        let source: i64 = tx.conn().query_row(
            "SELECT message_id FROM agent_events WHERE event_type='delivery_suppressed_revoked'",
            [],
            |r| r.get(0),
        )?;
        assert_eq!(source, m.id);
        Ok(())
    });
}

#[test]
fn ws11_delivery_case_membership_removed_before_delivery_writes_a_suppression_row() {
    let t = setup();
    t.write(|tx| {
        mention(tx)?;
        let e = last(tx, id("bender_agent"))?;
        tx.conn().execute(
            "DELETE FROM memberships WHERE user_id=? AND room_id=?",
            params![id("bender"), id("watercooler")],
        )?;
        delivery::perform_delivery(tx, e.id)?;
        assert_eq!(last(tx, e.agent_id)?.outcome.as_deref(), Some("suppressed"));
        assert_eq!(count(tx, e.agent_id, "delivery_suppressed_revoked")?, 1);
        Ok(())
    });
}

#[test]
fn ws11_delivery_case_a_delivery_claimed_inside_a_rolled_back_transaction_enqueues_no_webhook() {
    let t = setup();
    let eid = t.write(|tx| {
        mention(tx)?;
        Ok(last(tx, id("bender_agent"))?.id)
    });
    t.sink.take();
    assert!(
        t.try_write(move |tx| {
            delivery::perform_delivery(tx, eid)?;
            Err::<(), _>(crate::Error::Other("rollback".into()))
        })
        .is_err()
    );
    assert!(
        t.events()
            .iter()
            .all(|e| e.as_job::<EventWebhookJob>().is_none())
    );
    t.read(move |c| {
        assert_eq!(
            AgentEvent::find(c, eid)?.unwrap().outcome.as_deref(),
            Some("pending")
        );
        Ok(())
    });
}

#[test]
fn ws11_delivery_case_room_soft_deleted_before_delivery_suppresses_without_a_new_suppression_row() {
    let t = setup();
    t.write(|tx| {
        mention(tx)?;
        let e = last(tx, id("bender_agent"))?;
        let before: i64 = tx.conn().query_row(
            "SELECT COUNT(*) FROM agent_events WHERE agent_id=?",
            [e.agent_id],
            |r| r.get(0),
        )?;
        Room::find(tx.conn(), id("watercooler"))?.begin_destroy(tx)?;
        delivery::perform_delivery(tx, e.id)?;
        assert_eq!(last(tx, e.agent_id)?.outcome.as_deref(), Some("suppressed"));
        assert_eq!(
            tx.conn().query_row(
                "SELECT COUNT(*) FROM agent_events WHERE agent_id=?",
                [e.agent_id],
                |r| r.get::<_, i64>(0)
            )?,
            before
        );
        Ok(())
    });
}

#[test]
fn ws11_delivery_case_message_deleted_before_delivery_suppresses_without_crashing_or_posting() {
    let t = setup();
    t.write(|tx| {
        let m = mention(tx)?;
        let e = last(tx, id("bender_agent"))?;
        m.destroy(tx)?;
        delivery::perform_delivery(tx, e.id)?;
        assert_eq!(last(tx, e.agent_id)?.outcome.as_deref(), Some("suppressed"));
        Ok(())
    });
    assert!(
        t.events()
            .iter()
            .all(|e| e.as_job::<EventWebhookJob>().is_none())
    );
}

#[test]
fn ws11_delivery_case_rate_limit_drops_the_21st_delivery_with_a_suppression_row_and_no_job() {
    let t = setup();
    t.write(|tx| {
        for _ in 0..20 {
            mention(tx)?;
        }
        assert_eq!(count(tx, id("bender_agent"), "mention")?, 20);
        Ok(())
    });
    t.sink.take();
    t.write(|tx| {
        mention(tx)?;
        assert_eq!(count(tx, id("bender_agent"), "mention")?, 20);
        assert_eq!(
            count(tx, id("bender_agent"), "delivery_suppressed_rate_limit")?,
            1
        );
        let outcome: String = tx.conn().query_row(
            "SELECT outcome FROM agent_events WHERE event_type='delivery_suppressed_rate_limit'",
            [],
            |r| r.get(0),
        )?;
        assert_eq!(outcome, "suppressed");
        Ok(())
    });
    assert!(
        t.events()
            .iter()
            .all(|e| e.as_job::<DeliveryJob>().is_none())
    );
}

#[test]
fn ws11_delivery_case_rate_limit_is_per_agent_per_room() {
    let t = setup();
    let a = t.write(|tx| {
        let (a, _) = other(tx, "Throttle Bot")?;
        for _ in 0..20 {
            mention(tx)?;
        }
        Ok(a)
    });
    t.sink.take();
    t.write(move |tx| {
        post(
            tx,
            id("david"),
            id("watercooler"),
            "Hey @[Throttle Bot]",
            None,
        )?;
        assert_eq!(count(tx, a, "mention")?, 1);
        Ok(())
    });
    assert_eq!(
        t.events()
            .iter()
            .filter_map(|e| e.as_job::<DeliveryJob>())
            .count(),
        1
    );
}

#[test]
fn ws11_delivery_case_reply_to_an_agent_s_message_creates_a_reply_event() {
    let t = setup();
    let eid = t.write(|tx| {
        let m = post(tx, id("bender"), id("watercooler"), "Agent here", None)?;
        assert_eq!(m.creator_id, id("bender"));
        Ok(m.id)
    });
    t.sink.take();
    t.write(move |tx| {
        post(tx, id("david"), id("watercooler"), "Answering", Some(eid))?;
        assert_eq!(last(tx, id("bender_agent"))?.event_type, "reply");
        Ok(())
    });
    assert_eq!(
        t.events()
            .iter()
            .filter_map(|e| e.as_job::<DeliveryJob>())
            .count(),
        1
    );
}

#[test]
fn ws11_delivery_case_a_message_that_both_mentions_and_replies_records_a_single_reply_event() {
    let t = setup();
    t.write(|tx| {
        let m = post(tx, id("bender"), id("watercooler"), "Agent here", None)?;
        post(tx, id("david"), m.room_id, "Hey @[Bender Bot]", Some(m.id))?;
        assert_eq!(count(tx, id("bender_agent"), "reply")?, 1);
        assert_eq!(count(tx, id("bender_agent"), "mention")?, 0);
        assert_eq!(last(tx, id("bender_agent"))?.event_type, "reply");
        Ok(())
    });
}

#[test]
fn ws11_delivery_case_direct_room_messages_create_direct_message_events() {
    let t = setup();
    t.write(|tx| {
        post(tx, id("kevin"), id("bender_and_kevin"), "Hello bot", None)?;
        let e = last(tx, id("bender_agent"))?;
        assert_eq!(e.event_type, "direct_message");
        assert_eq!(e.room_id, Some(id("bender_and_kevin")));
        Ok(())
    });
    assert_eq!(
        t.events()
            .iter()
            .filter_map(|e| e.as_job::<DeliveryJob>())
            .count(),
        1
    );
}

#[test]
fn ws11_delivery_case_agent_posting_writes_a_posted_row() {
    let t = setup();
    t.write(|tx| {
        post(tx, id("bender"), id("watercooler"), "Posting", None)?;
        assert_eq!(count(tx, id("bender_agent"), "posted")?, 1);
        let outcome: String = tx.conn().query_row(
            "SELECT outcome FROM agent_events WHERE agent_id=? AND event_type='posted'",
            [id("bender_agent")],
            |r| r.get(0),
        )?;
        assert_eq!(outcome, "delivered");
        Ok(())
    });
}

#[test]
fn ws11_delivery_case_agent_never_receives_its_own_messages() {
    let t = setup();
    t.write(|tx| {
        post(
            tx,
            id("bender"),
            id("watercooler"),
            "Talking @[Bender Bot]",
            None,
        )?;
        for kind in ["mention", "reply", "direct_message"] {
            assert_eq!(count(tx, id("bender_agent"), kind)?, 0);
        }
        Ok(())
    });
}

#[test]
fn ws11_delivery_case_bot_without_an_agent_row_receives_no_events() {
    let t = setup();
    t.write(|tx| {
        Agent::find(tx.conn(), id("bender_agent"))?
            .unwrap()
            .destroy(tx)?;
        mention(tx)?;
        assert_eq!(
            tx.conn()
                .query_row("SELECT COUNT(*) FROM agent_events", [], |r| r
                    .get::<_, i64>(0))?,
            0
        );
        Ok(())
    });
}

#[test]
fn ws11_delivery_case_hop_limit_suppresses_a_chain_that_reaches_hop_3() {
    let t = setup();
    t.write(|tx|{let (a,_,m3)=chain(tx)?;post(tx,id("bender"),id("watercooler"),"Hey @[Hop Bot B] again",Some(m3))?;assert_eq!(count(tx,a,"delivery_suppressed_hop_limit")?,1);let (hop,outcome):(i64,String)=tx.conn().query_row("SELECT hop,outcome FROM agent_events WHERE agent_id=? AND event_type='delivery_suppressed_hop_limit'",[a],|r|Ok((r.get(0)?,r.get(1)?)))?;assert_eq!((hop,outcome),(3,"suppressed".into()));Ok(())});
    assert_eq!(
        t.events()
            .iter()
            .filter_map(|e| e.as_job::<DeliveryJob>())
            .count(),
        3
    );
}

#[test]
fn ws11_delivery_case_replying_to_an_old_low_hop_message_does_not_reset_the_chain() {
    let t = setup();
    t.write(|tx|{let (a,m1,_)=chain(tx)?;post(tx,id("bender"),id("watercooler"),"Hey @[Hop Bot B] again",Some(m1))?;assert_eq!(count(tx,a,"delivery_suppressed_hop_limit")?,1);assert_eq!(tx.conn().query_row("SELECT hop FROM agent_events WHERE agent_id=? AND event_type='delivery_suppressed_hop_limit'",[a],|r|r.get::<_,i64>(0))?,3);Ok(())});
}

#[test]
fn ws11_delivery_case_suppression_rows_are_never_hop_triggers() {
    let t = setup();
    t.write(|tx| {
        let (a, _) = other(tx, "Suppression Bot B")?;
        AgentEvent::create(
            tx,
            NewEvent {
                agent_id: id("bender_agent"),
                room_id: Some(id("watercooler")),
                event_type: "delivery_suppressed_hop_limit".into(),
                outcome: Some("suppressed".into()),
                hop: 3,
                metadata: serde_json::json!({"hop":3}),
                ..Default::default()
            },
        )?;
        post(
            tx,
            id("bender"),
            id("watercooler"),
            "Hey @[Suppression Bot B] fresh",
            None,
        )?;
        assert_eq!(last(tx, a)?.hop, 0);
        Ok(())
    });
    assert_eq!(
        t.events()
            .iter()
            .filter_map(|e| e.as_job::<DeliveryJob>())
            .count(),
        1
    );
}

#[test]
fn ws11_delivery_case_pending_rows_are_hop_triggers() {
    let t = setup();
    t.write(|tx| {
        let (a, _) = other(tx, "Pending Bot B")?;
        mention(tx)?;
        assert_eq!(
            last(tx, id("bender_agent"))?.outcome.as_deref(),
            Some("pending")
        );
        post(
            tx,
            id("bender"),
            id("watercooler"),
            "Hey @[Pending Bot B] fresh",
            None,
        )?;
        assert_eq!(last(tx, a)?.hop, 1);
        Ok(())
    });
}

#[test]
fn ws11_delivery_case_deliveries_older_than_five_minutes_are_not_hop_triggers() {
    let t = setup();
    t.write(|tx| {
        let (a, _) = other(tx, "Window Bot B")?;
        mention(tx)?;
        let e = last(tx, id("bender_agent"))?;
        delivery::perform_delivery(tx, e.id)?;
        tx.conn().execute(
            "UPDATE agent_events SET created_at=? WHERE id=?",
            params![tx.now().ago(jiff::SignedDuration::from_mins(6)), e.id],
        )?;
        post(
            tx,
            id("bender"),
            id("watercooler"),
            "Hey @[Window Bot B] fresh",
            None,
        )?;
        assert_eq!(last(tx, a)?.hop, 0);
        Ok(())
    });
}

#[test]
fn ws11_delivery_case_bridging_rooms_carries_the_hop_chain_instead_of_resetting_it() {
    let t = setup();
    t.write(|tx|{let (a,b)=other(tx,"Bridge Bot B")?;let other_room=Room::find(tx.conn(),id("designers"))?;other_room.grant_to(tx,&[id("bender"),b])?;mention(tx)?;let first=last(tx,id("bender_agent"))?;assert_eq!(first.hop,0);post(tx,id("bender"),other_room.id,"Hey @[Bridge Bot B] over here",None)?;let second=last(tx,a)?;assert_eq!(second.hop,1);assert_eq!(second.chain_id,first.chain_id);post(tx,b,id("watercooler"),"Hey @[Bender Bot] back",None)?;let third=last(tx,id("bender_agent"))?;assert_eq!(third.hop,2);assert_eq!(third.chain_id,first.chain_id);post(tx,id("bender"),other_room.id,"Hey @[Bridge Bot B] again",None)?;assert_eq!(count(tx,a,"delivery_suppressed_hop_limit")?,1);assert_eq!(tx.conn().query_row("SELECT hop FROM agent_events WHERE agent_id=? AND event_type='delivery_suppressed_hop_limit'",[a],|r|r.get::<_,i64>(0))?,3);Ok(())});
    assert_eq!(
        t.events()
            .iter()
            .filter_map(|e| e.as_job::<DeliveryJob>())
            .count(),
        3
    );
}

#[test]
fn ws11_delivery_case_an_agent_s_own_posts_are_not_hop_triggers() {
    let t = setup();
    t.write(|tx| {
        let (a, _) = other(tx, "Chain Bot B")?;
        post(
            tx,
            id("bender"),
            id("watercooler"),
            "Hey @[Chain Bot B] one",
            None,
        )?;
        assert_eq!(last(tx, a)?.hop, 0);
        post(
            tx,
            id("bender"),
            id("watercooler"),
            "Hey @[Chain Bot B] two",
            None,
        )?;
        assert_eq!(last(tx, a)?.hop, 0);
        Ok(())
    });
}
