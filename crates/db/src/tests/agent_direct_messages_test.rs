use super::*;
use crate::models::{
    agent_delivery::{AgentEvent, NewEvent},
    agent_direct_messages::{self as dms, DirectMessageResult},
    agent_posting::{DriveInput, PostResult},
    audit_log::Context,
};
use crate::{AgentGrant, NewGrant, NewMessage, Room, Timestamp};
use rusqlite::params;
use serde_json::{Value, json};
fn setup() -> TestDb {
    let t = super::channel_thread_test::frozen();
    t.clock
        .travel_to(Timestamp::parse_db("2026-03-02 16:00:00").unwrap());
    t.write(|tx| {
        tx.conn().execute(
            "UPDATE agents SET owner_id=?,daily_message_cap=NULL WHERE id=?",
            params![id("david"), id("bender_agent")],
        )?;
        tx.conn().execute(
            "UPDATE users SET time_zone='UTC' WHERE id=?",
            [id("bender")],
        )?;
        tx.conn().execute("DELETE FROM agent_grants", [])?;
        tx.conn().execute("DELETE FROM agent_events", [])?;
        Ok(())
    });
    t
}
fn attrs(text: &str, client: Option<&str>) -> NewMessage {
    NewMessage {
        markdown_source: Some(text.into()),
        client_message_id: client.map(str::to_owned),
        ..Default::default()
    }
}
fn capture(tx: &Tx<'_>, result: DirectMessageResult) -> Result<Value> {
    Ok(match result {
        DirectMessageResult::Denied(r) => {
            json!({"status":r.status,"error":r.error,"payload":r.payload})
        }
        DirectMessageResult::Posted { room, message } => {
            let mut members = room.user_ids(tx.conn())?;
            members.sort();
            json!({"status":201,"error":null,"payload":{"message":{"source":message.markdown_source,"creator_id":message.creator_id,"client_message_id":message.client_message_id},"room":{"type":room.room_type.class_name(),"members":members}}})
        }
    })
}
fn check(tx: &mut Tx<'_>, key: &str, target: i64, a: NewMessage, drive: DriveInput) -> Result<()> {
    let result = dms::open_and_post(
        tx,
        id("bender_agent"),
        target,
        a,
        drive,
        &Context::default(),
    )?;
    let gold: Value = serde_json::from_str(include_str!(
        "../../../../vectors/agents_direct_messages_contract.json"
    ))
    .unwrap();
    assert_eq!(capture(tx, result)?, gold["results"][key], "{key}");
    Ok(())
}
#[test]
fn ws11_dm_target_permissions_budget_replay_drive_and_errors_match_rails() {
    let t = setup();
    t.write(|tx| {
        check(tx, "missing", 0, NewMessage::default(), DriveInput::Absent)?;
        check(
            tx,
            "bot",
            id("bender"),
            NewMessage::default(),
            DriveInput::Absent,
        )?;
        tx.conn()
            .execute("UPDATE users SET status=1 WHERE id=?", [id("jason")])?;
        check(
            tx,
            "inactive",
            id("jason"),
            NewMessage::default(),
            DriveInput::Absent,
        )?;
        tx.conn()
            .execute("UPDATE users SET status=0 WHERE id=?", [id("jason")])?;
        check(
            tx,
            "unrelated",
            id("jason"),
            NewMessage::default(),
            DriveInput::Absent,
        )?;
        check(
            tx,
            "owner",
            id("david"),
            attrs("Hello", Some("ws11-dm-owner")),
            DriveInput::Absent,
        )?;
        tx.conn().execute(
            "UPDATE agents SET daily_message_cap=0 WHERE id=?",
            [id("bender_agent")],
        )?;
        check(
            tx,
            "replay",
            id("david"),
            attrs("Hello", Some("ws11-dm-owner")),
            DriveInput::Invalid,
        )?;
        check(
            tx,
            "budget",
            id("david"),
            attrs("Other", None),
            DriveInput::Absent,
        )?;
        tx.conn().execute(
            "UPDATE agents SET daily_message_cap=NULL WHERE id=?",
            [id("bender_agent")],
        )?;
        AgentEvent::create(
            tx,
            NewEvent {
                agent_id: id("bender_agent"),
                actor_id: Some(id("jason")),
                event_type: "mention".into(),
                outcome: Some("suppressed".into()),
                ..Default::default()
            },
        )?;
        check(
            tx,
            "previous_human",
            id("jason"),
            attrs("Old inbound", Some("ws11-dm-prior")),
            DriveInput::Absent,
        )?;
        check(
            tx,
            "bad_reply",
            id("david"),
            NewMessage {
                reply_to_message_id: Some(1),
                ..attrs("Reply", None)
            },
            DriveInput::Absent,
        )?;
        check(
            tx,
            "invalid_drive",
            id("david"),
            attrs("Drive", None),
            DriveInput::Invalid,
        )?;
        check(
            tx,
            "blank",
            id("david"),
            attrs("", None),
            DriveInput::Absent,
        )?;
        let mut grant = AgentGrant::create(
            tx,
            NewGrant {
                agent_id: id("bender_agent"),
                capability: "post_messages".into(),
                room_id: Some(id("watercooler")),
                granted_by_id: id("david"),
                ..Default::default()
            },
        )?;
        check(
            tx,
            "existing_room_scope",
            id("david"),
            attrs("Hello", Some("ws11-dm-owner")),
            DriveInput::Absent,
        )?;
        check(
            tx,
            "scope_bot",
            id("bender"),
            NewMessage::default(),
            DriveInput::Absent,
        )?;
        grant.revoke(tx)?;
        check(
            tx,
            "revoked",
            id("david"),
            attrs("Hello", Some("ws11-dm-owner")),
            DriveInput::Absent,
        )?;
        Ok(())
    });
}
#[test]
fn ws11_dm_new_room_commits_denial_and_scope_is_rechecked_on_existing_room() {
    let t = setup();
    t.sink.take();
    t.write(|tx| {
        AgentGrant::create(tx,NewGrant{agent_id:id("bender_agent"),capability:"post_messages".into(),room_id:Some(id("watercooler")),granted_by_id:id("david"),..Default::default()})?;
        let result=dms::open_and_post(tx,id("bender_agent"),id("david"),attrs("",None),DriveInput::Absent,&Context::default())?;
        assert!(matches!(result,DirectMessageResult::Denied(r) if r.status==422));
        let room=Room::find_direct_for(tx.conn(),&[id("bender"),id("david")])?.expect("new DM survives validation denial");
        let audit:Value=tx.conn().query_row("SELECT json_object('actor_id',actor_id,'actor_label',actor_label,'changes',json(details)) FROM audit_logs WHERE action='room.create' AND target_id=? ORDER BY id DESC LIMIT 1",[room.id],|r|r.get(0))?;
        assert_eq!(audit,json!({"actor_id":id("bender"),"actor_label":"Agent Bender Bot","changes":{"name":null}}));
        let result=dms::open_and_post(tx,id("bender_agent"),id("david"),attrs("Now",None),DriveInput::Absent,&Context::default())?;
        assert!(matches!(result,DirectMessageResult::Denied(r) if r.status==403));Ok(())
    });
    let broadcasts=t.sink.take().into_iter().filter(|e|matches!(e,Event::Broadcast(r) if r.decode::<crate::broadcasts::Broadcast>().is_some_and(|r|matches!(r,Ok(crate::broadcasts::Broadcast::MembershipChanged { .. }))))).count();
    assert_eq!(broadcasts, 2);
}
#[test]
fn ws11_shared_posting_thread_validation_rolls_back_join_and_preflight_order() {
    let t = setup();
    t.write(|tx| {
        Room::find(tx.conn(), id("watercooler"))?.grant_to(tx, &[id("bender")])?;
        let thread = crate::ChannelThread::create(
            tx,
            crate::NewChannelThread {
                room_id: id("watercooler"),
                creator_id: id("david"),
                name: Some("Posting".into()),
                ..Default::default()
            },
        )?;
        let result = crate::models::agent_posting::post_service(
            tx,
            id("bender_agent"),
            NewMessage {
                room_id: thread.room_id,
                thread_id: Some(thread.id),
                ..attrs("", None)
            },
            DriveInput::Absent,
        )?;
        assert!(matches!(result,PostResult::Denied(r) if r.status==422));
        assert!(!crate::sql::exists(
            tx.conn(),
            "SELECT 1 FROM thread_memberships WHERE thread_id=? AND user_id=?",
            params![thread.id, id("bender")]
        )?);
        assert!(
            crate::models::agent_posting::post_service(
                tx,
                id("bender_agent"),
                NewMessage {
                    room_id: thread.room_id,
                    reply_to_message_id: Some(-1),
                    ..attrs("", None)
                },
                DriveInput::Invalid
            )
            .is_err()
        );
        Ok(())
    });
}
