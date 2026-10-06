//! Named comparisons against security_lifecycle_cases.rb on Rails d7c7de92.
use super::*;
use crate::models::{agent_lifecycle, audit_log::Context};
use crate::{
    Agent, AgentApproval, AgentGrant, Membership, Message, NewApproval, NewGrant, NewMessage, Room,
    Timestamp, User,
};
use rusqlite::params;
use serde_json::{Value, json};

fn gold(key: &str) -> Value {
    let all: Value = serde_json::from_str(include_str!(
        "../../../../vectors/agents_security_lifecycle_cases.json"
    ))
    .unwrap();
    all["cases"][key].clone()
}
fn setup() -> TestDb {
    let t = super::channel_thread_test::frozen();
    t.clock
        .travel_to(Timestamp::parse_db("2026-03-02 16:00:00").unwrap());
    t.write(|tx| {
        tx.conn().execute(
            "UPDATE agents SET owner_id=?,suspended_at=NULL WHERE id=?",
            params![id("david"), id("bender_agent")],
        )?;
        tx.conn().execute("DELETE FROM agent_grants", [])?;
        tx.conn().execute(
            "DELETE FROM activity_items WHERE source_type='AgentApproval'",
            [],
        )?;
        tx.conn().execute("DELETE FROM agent_approvals", [])?;
        tx.conn().execute("DELETE FROM agent_events", [])?;
        tx.conn().execute("DELETE FROM audit_logs", [])?;
        Room::find(tx.conn(), id("watercooler"))?.grant_to(tx, &[id("bender")])?;
        Ok(())
    });
    t.sink.take();
    t
}
fn grant(tx: &mut Tx<'_>, room: Option<i64>, capability: &str) -> Result<AgentGrant> {
    AgentGrant::create(
        tx,
        NewGrant {
            agent_id: id("bender_agent"),
            granted_by_id: id("david"),
            room_id: room,
            capability: capability.into(),
            ..Default::default()
        },
    )
}
fn revoke_snapshot(conn: &Connection, scoped: i64, extra: i64) -> Result<Value> {
    let agent = Agent::find(conn, id("bender_agent"))?;
    Ok(
        json!({"scoped_revoked": AgentGrant::find(conn, scoped)?.map(|g|g.revoked()),
        "extra_revoked": AgentGrant::find(conn, extra)?.unwrap().revoked(),
        "member": Membership::find_by_room_and_user(conn, id("watercooler"), id("bender"))?.is_some(),
        "can_post": agent.as_ref().map(|a|a.can(conn,"post_messages",Some(id("watercooler")))).transpose()?,
        "can_react": agent.as_ref().map(|a|a.can(conn,"react",Some(id("watercooler")))).transpose()?}),
    )
}
fn revocation(kind: &'static str) {
    let t = setup();
    let (scoped, extra) = t.write(move |tx| {
        if kind == "workspace" {
            return Ok((0, grant(tx, None, "post_messages")?.id));
        }
        let scoped = grant(tx, Some(id("watercooler")), "post_messages")?.id;
        let extra = if kind == "membership" {
            grant(tx, Some(id("designers")), "post_messages")?.id
        } else {
            grant(tx, None, "react")?.id
        };
        Ok((scoped, extra))
    });
    let result = t.try_write(move |tx| {
        match kind {
            "membership" | "workspace" | "rollback" => {
                Membership::find_by_room_and_user(tx.conn(), id("watercooler"), id("bender"))?
                    .unwrap()
                    .destroy(tx)?
            }
            "room" => Room::find(tx.conn(), id("watercooler"))?.destroy(tx)?,
            "suspend" => agent_lifecycle::suspend(tx, id("bender_agent"), &Context::default())?,
            "deactivate" => User::find(tx.conn(), id("bender"))?.deactivate(tx)?,
            "ban" => User::find(tx.conn(), id("bender"))?.ban(tx)?,
            "destroy" => User::find(tx.conn(), id("bender"))?.destroy(tx)?,
            _ => unreachable!(),
        }
        assert_eq!(
            revoke_snapshot(tx.conn(), scoped, extra)?,
            gold(&format!("revoke_{kind}"))
        );
        if kind == "rollback" {
            return Err(crate::Error::Other("rollback".into()));
        }
        Ok(())
    });
    if kind == "rollback" {
        assert!(result.is_err());
        t.read(move |conn| {
            assert_eq!(
                revoke_snapshot(conn, scoped, extra)?,
                gold("revoke_rollback_after")
            );
            Ok(())
        });
    } else {
        result.unwrap();
    }
}
#[test]
fn ws11_revocation_case_membership_scoped_only() {
    revocation("membership");
}
#[test]
fn ws11_revocation_case_workspace_survives() {
    revocation("workspace");
}
#[test]
fn ws11_revocation_case_removal_and_grant_rollback() {
    revocation("rollback");
}
#[test]
fn ws11_revocation_case_room_destroy() {
    revocation("room");
}
#[test]
fn ws11_revocation_case_suspend_all() {
    revocation("suspend");
}
#[test]
fn ws11_revocation_case_deactivate_all() {
    revocation("deactivate");
}
#[test]
fn ws11_revocation_case_ban_all() {
    revocation("ban");
}
#[test]
fn ws11_revocation_case_destroy_all() {
    revocation("destroy");
}

fn kill_setup() -> (TestDb, i64) {
    let t = setup();
    let pending = t.write(|tx| {
        let pending = AgentApproval::create(
            tx,
            NewApproval {
                agent_id: id("bender_agent"),
                action: "deploy".into(),
                summary: "Ship it".into(),
                ..Default::default()
            },
        )?;
        let expired = AgentApproval::create(
            tx,
            NewApproval {
                agent_id: id("bender_agent"),
                action: "old".into(),
                summary: "Stale".into(),
                ..Default::default()
            },
        )?;
        tx.conn().execute(
            "UPDATE agent_approvals SET expires_at=? WHERE id=?",
            params![
                tx.now().ago(jiff::SignedDuration::from_hours(1)),
                expired.id
            ],
        )?;
        let approved = AgentApproval::create(
            tx,
            NewApproval {
                agent_id: id("bender_agent"),
                action: "done".into(),
                summary: "Over".into(),
                ..Default::default()
            },
        )?;
        tx.conn().execute(
            "UPDATE agent_approvals SET status='approved',decided_by_id=?,decided_at=? WHERE id=?",
            params![id("david"), tx.now(), approved.id],
        )?;
        grant(tx, None, "post_messages")?;
        Agent::find(tx.conn(), id("bender_agent"))?
            .unwrap()
            .set_working_presence(tx, Some("Thinking…"))?;
        Ok(pending.id)
    });
    (t, pending)
}
#[test]
fn ws11_kill_case_suspend_cancel_expire_audit() {
    let (t, _) = kill_setup();
    let count =
        t.write(|tx| agent_lifecycle::kill_switch(tx, id("bender_agent"), &Context::default()));
    t.read(move|conn| {
        let agent=Agent::find(conn,id("bender_agent"))?.unwrap();
        let statuses=crate::sql::query_all(conn,"SELECT status FROM agent_approvals ORDER BY id",[],|r|r.get::<_,String>(0))?;
        let audits=crate::sql::query_all(conn,"SELECT action,details FROM audit_logs WHERE action IN ('agent.suspend','agent.kill_switch') ORDER BY id",[],|r|Ok(json!({"action":r.get::<_,String>(0)?,"details":r.get::<_,Value>(1)?})))?;
        let active=conn.query_row("SELECT COUNT(*) FROM agent_grants WHERE revoked_at IS NULL",[],|r|r.get::<_,i64>(0))?;
        assert_eq!(json!({"cancelled":count,"suspended":agent.suspended(),"statuses":statuses,"presence":agent.working_presence,"active_grants":active,"audits":audits}),gold("kill"));Ok(())
    });
}
#[test]
fn ws11_kill_case_inbox_handled() {
    let (t, pending) = kill_setup();
    t.write(|tx| agent_lifecycle::kill_switch(tx, id("bender_agent"), &Context::default()));
    t.read(move|conn| {
        let items=crate::sql::query_all(conn,"SELECT handled_at IS NOT NULL FROM activity_items WHERE source_type='AgentApproval' AND source_id=? AND user_id=?",params![pending,id("david")],|r|r.get::<_,bool>(0))?;
        assert_eq!(items.len(),1);assert_eq!(json!({"handled":items[0]}),gold("kill_inbox"));Ok(())
    });
}
#[test]
fn ws11_kill_case_repeat_safe() {
    let (t, _) = kill_setup();
    t.write(|tx| agent_lifecycle::kill_switch(tx, id("bender_agent"), &Context::default()));
    let count =
        t.write(|tx| agent_lifecycle::kill_switch(tx, id("bender_agent"), &Context::default()));
    t.read(move|conn| {
        let count_audit=|action:&str|->Result<i64>{Ok(conn.query_row("SELECT COUNT(*) FROM audit_logs WHERE action=?",[action],|r|r.get(0))?)};
        assert_eq!(json!({"cancelled":count,"suspended":Agent::find(conn,id("bender_agent"))?.unwrap().suspended(),"suspend_audits":count_audit("agent.suspend")?,"kill_audits":count_audit("agent.kill_switch")?}),gold("kill_repeat"));Ok(())
    });
}
fn quiet(kind: &'static str) {
    let t = setup();
    t.write(|tx| {
        tx.conn().execute(
            "UPDATE memberships SET unread_at=NULL WHERE room_id=?",
            [id("watercooler")],
        )?;
        Ok(())
    });
    let mid = t.write(move |tx| {
        Ok(Message::create(
            tx,
            NewMessage {
                creator_id: id("bender"),
                room_id: id("watercooler"),
                streaming: true,
                markdown_source: Some(
                    if kind == "rollback" {
                        "Still thinking"
                    } else {
                        "Hey @[David] hovercraft"
                    }
                    .into(),
                ),
                client_message_id: Some(format!("ws11-security-{kind}")),
                ..Default::default()
            },
        )?
        .id)
    });
    t.sink.take();
    let result = t.try_write(move |tx| {
        if kind == "kill" {
            agent_lifecycle::kill_switch(tx, id("bender_agent"), &Context::default())?;
        } else {
            agent_lifecycle::suspend(tx, id("bender_agent"), &Context::default())?;
        }
        if kind == "rollback" {
            return Err(crate::Error::Other("rollback".into()));
        }
        Ok(())
    });
    if kind == "rollback" {
        assert!(result.is_err())
    } else {
        result.unwrap()
    }
    let emitted = t.events();
    let jobs = emitted
        .iter()
        .filter(|e| matches!(e, Event::Job(_) | Event::DeliverWebhook { .. }))
        .count();
    let broadcasts = emitted
        .iter()
        .filter_map(|e| e.as_broadcast())
        .collect::<Vec<_>>();
    let message_broadcasts=broadcasts.iter().filter(|b|matches!(b,crate::broadcasts::Broadcast::Turbo(s) if matches!(s.partial,Some(crate::broadcasts::Partial::MessageReplace {message_id}) if message_id==mid))).count();
    let unread_broadcasts=broadcasts.iter().filter(|b|matches!(b,crate::broadcasts::Broadcast::Cable {stream,..} if stream.ends_with("_unreads"))).count();
    t.read(move|conn| {
        let count=|table:&str,where_clause:&str|->Result<i64>{Ok(conn.query_row(&format!("SELECT COUNT(*) FROM {table} WHERE {where_clause}"),[mid],|r|r.get(0))?)};
        let search=Message::search_in_room(conn,id("watercooler"),"hovercraft")?.into_iter().map(|m|m.id).collect::<Vec<_>>();
        let unread=Membership::find_by_room_and_user(conn,id("watercooler"),id("david"))?.unwrap().unread_at.map(crate::models::agent_payloads::json_time);
        assert_eq!(json!({"streaming":Message::find(conn,mid)?.streaming,"suspended":Agent::find(conn,id("bender_agent"))?.unwrap().suspended(),"activity":count("activity_items","source_type='Message' AND source_id=?")?,"events":count("agent_events","message_id=?")?,"search":search,"unread":unread,"jobs":jobs,"message_broadcasts":message_broadcasts,"unread_broadcasts":unread_broadcasts}),gold(&format!("quiet_{kind}")));Ok(())
    });
}
#[test]
fn ws11_kill_case_quiet_stream() {
    quiet("kill");
}
#[test]
fn ws11_kill_case_suspend_quiet_stream() {
    quiet("suspend");
}
#[test]
fn ws11_kill_case_suspension_rollback() {
    quiet("rollback");
}
