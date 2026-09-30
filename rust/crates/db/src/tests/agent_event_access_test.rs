use super::*;
use crate::models::agent_delivery::{AgentEvent, NewEvent};
use crate::models::agent_event_access::{self as access, Acknowledgment};
use crate::{AgentGrant, Membership, NewGrant, Room, Tx};
use rusqlite::params;
use serde_json::{Value, json};

fn add(
    tx: &Tx<'_>,
    label: &str,
    kind: &str,
    room: Option<i64>,
    message: Option<i64>,
    outcome: Option<&str>,
) -> crate::Result<i64> {
    Ok(AgentEvent::create(
        tx,
        NewEvent {
            agent_id: id("bender_agent"),
            room_id: room,
            message_id: message,
            event_type: kind.into(),
            outcome: outcome.map(str::to_owned),
            metadata: json!({"label":label}),
            ..Default::default()
        },
    )?
    .id)
}
fn grant(tx: &Tx<'_>, room: Option<i64>) -> crate::Result<AgentGrant> {
    AgentGrant::create(
        tx,
        NewGrant {
            agent_id: id("bender_agent"),
            granted_by_id: id("david"),
            capability: "read_messages".into(),
            room_id: room,
            ..Default::default()
        },
    )
}
fn page(tx: &Tx<'_>, since: i64, limit: Option<i64>) -> crate::Result<Value> {
    Ok(Value::Array(
        access::readable_page(tx.conn(), id("bender_agent"), since, limit)?
            .into_iter()
            .map(|e| e.metadata["label"].clone())
            .collect(),
    ))
}
fn ack(tx: &Tx<'_>, event_id: i64) -> crate::Result<Value> {
    let (status, error) = match access::acknowledge(tx, id("bender_agent"), event_id)? {
        Acknowledgment::Acknowledged { id } => {
            assert_eq!(id, event_id);
            ("ok", None)
        }
        Acknowledgment::NotFound => ("not_found", Some("Event not found")),
        Acknowledgment::Forbidden => (
            "forbidden",
            Some("Forbidden: agent lacks read_messages capability"),
        ),
    };
    let event = AgentEvent::find(tx.conn(), event_id)?.unwrap();
    Ok(
        json!({"status":status,"error":error,"outcome":event.outcome,
        "webhook_status":event.webhook_status,"webhook_attempts":event.webhook_attempts}),
    )
}

#[test]
fn ws11_event_readability_and_ack_match_rails_access_matrix() {
    let t = super::channel_thread_test::frozen();
    t.write(|tx| {
        let gold: Value = serde_json::from_str(include_str!("../../../../vectors/agents_event_access_contract.json")).unwrap();
        tx.conn().execute("DELETE FROM agent_events", [])?;
        tx.conn().execute("DELETE FROM agent_grants", [])?;
        let water = id("watercooler");
        let designers = id("designers");
        for room in [water, designers] {
            Room::find(tx.conn(), room)?.grant_to(tx, &[id("bender")])?;
        }
        let mut rows = std::collections::HashMap::new();
        for (label, kind, room, message, outcome) in [
            ("blocked_message", "mention", Some(designers), Some(id("first")), Some("pending")),
            ("granted_message", "reply", Some(water), Some(id("fourth")), Some("pending")),
            ("missing_message", "mention", Some(water), Some(-1), Some("pending")),
            ("missing_event_room", "mention", None, Some(id("fourth")), Some("pending")),
            ("approval", "approval_decided", Some(designers), None, Some("pending")),
            ("github", "github_action_completed", None, None, Some("pending")),
            ("fizzy", "fizzy_action_completed", None, None, Some("pending")),
            ("granted_work", "work_assigned", Some(water), None, Some("pending")),
            ("blocked_work", "work_unassigned", Some(designers), None, Some("pending")),
            ("slash", "slash_command", Some(water), None, Some("pending")),
            ("posted", "posted", Some(water), None, Some("delivered")),
            ("suppressed", "mention", Some(water), Some(id("fourth")), Some("suppressed")),
            ("null_outcome", "mention", Some(water), Some(id("fourth")), None),
        ] {
            rows.insert(label, add(tx, label, kind, room, message, outcome)?);
        }
        tx.conn().execute("INSERT INTO agents(id,user_id,owner_id,kind,created_at,updated_at) VALUES (900010001,?,?,'personal',?,?)", params![id("david"), id("david"), tx.now(), tx.now()])?;
        rows.insert("other_agent", AgentEvent::create(tx, NewEvent {
            agent_id: 900010001, event_type: "github_action_completed".into(), outcome: Some("pending".into()),
            metadata: json!({"label":"other_agent"}), ..Default::default()
        })?.id);
        let mut room_grant = grant(tx, Some(water))?;
        assert_eq!(page(tx, 0, Some(100))?, gold["pages"]["scoped"]);
        assert_eq!(page(tx, 0, Some(1))?, gold["pages"]["first_page"]);
        assert_eq!(page(tx, rows["granted_message"], Some(1))?, gold["pages"]["second_page"]);
        tx.conn().execute("UPDATE agent_events SET webhook_status='pending',webhook_attempts=2 WHERE id=?", [rows["granted_message"]])?;
        for (i, label) in ["granted_message", "granted_message", "blocked_message", "missing_message", "missing_event_room", "blocked_work", "suppressed", "posted", "null_outcome", "other_agent"].into_iter().enumerate() {
            assert_eq!(ack(tx, rows[label])?, gold["acks"][format!("{label}_{i}")], "{label}");
        }
        room_grant.revoke(tx)?;
        assert_eq!(page(tx, 0, Some(100))?, gold["pages"]["revoked"]);
        assert_eq!(ack(tx, rows["approval"])?, gold["acks"]["revoked_approval"]);
        assert_eq!(ack(tx, rows["granted_work"])?, gold["acks"]["revoked_work"]);
        grant(tx, None)?;
        assert_eq!(page(tx, 0, Some(100))?, gold["pages"]["workspace"]);
        grant(tx, Some(water))?;
        assert_eq!(page(tx, 0, Some(100))?, gold["pages"]["overlapping"]);
        Membership::find_by_room_and_user(tx.conn(), water, id("bender"))?.unwrap().destroy(tx)?;
        assert_eq!(page(tx, 0, Some(100))?, gold["pages"]["removed_membership"]);
        assert_eq!(ack(tx, rows["granted_message"])?, gold["acks"]["removed_message"]);
        assert_eq!(ack(tx, rows["granted_work"])?, gold["acks"]["removed_work"]);
        // Agent's complete update lifecycle is a later slice; reproduce its input
        // effects here to test the independent reader/ack policies.
        AgentGrant::revoke_for_agent(tx, id("bender_agent"))?;
        tx.conn().execute("UPDATE agents SET suspended_at=? WHERE id=?", params![tx.now(), id("bender_agent")])?;
        assert_eq!(ack(tx, rows["approval"])?, gold["acks"]["suspended_approval"]);
        assert_eq!(page(tx, 0, Some(100))?, gold["pages"]["suspended"]);
        tx.conn().execute("UPDATE agents SET suspended_at=NULL WHERE id=?", [id("bender_agent")])?;
        tx.conn().execute("DELETE FROM agent_grants", [])?;
        assert_eq!(page(tx, 0, Some(100))?, gold["pages"]["legacy"]);
        Ok(())
    });
}

#[test]
fn ws11_event_page_filters_before_limit_and_clamps_page_size() {
    let t = super::channel_thread_test::frozen();
    t.write(|tx| {
        tx.conn().execute("DELETE FROM agent_events", [])?;
        tx.conn().execute("DELETE FROM agent_grants", [])?;
        Room::find(tx.conn(), id("watercooler"))?.grant_to(tx, &[id("bender")])?;
        grant(tx, Some(id("watercooler")))?;
        for _ in 0..110 {
            add(
                tx,
                "blocked",
                "mention",
                Some(id("designers")),
                Some(id("first")),
                Some("pending"),
            )?;
        }
        for _ in 0..110 {
            add(
                tx,
                "visible",
                "reply",
                Some(id("watercooler")),
                Some(id("fourth")),
                Some("pending"),
            )?;
        }
        for (limit, count) in [(None, 50), (Some(500), 100), (Some(0), 1), (Some(-4), 1)] {
            let rows = access::readable_page(tx.conn(), id("bender_agent"), 0, limit)?;
            assert_eq!(rows.len(), count);
            assert!(rows.iter().all(|e| e.metadata["label"] == "visible"));
        }
        Ok(())
    });
}

#[test]
fn ws11_event_ack_rolls_back_without_changing_webhook_claim() {
    let t = super::channel_thread_test::frozen();
    let event_id = t.write(|tx| {
        let event_id = add(tx, "approval", "approval_decided", None, None, Some("delivered"))?;
        tx.conn().execute("UPDATE agent_events SET webhook_status='pending',webhook_attempts=3,webhook_next_attempt_at=? WHERE id=?", params![tx.now(), event_id])?;
        Ok(event_id)
    });
    assert!(
        t.try_write(move |tx| {
            assert_eq!(
                access::acknowledge(tx, id("bender_agent"), event_id)?,
                Acknowledgment::Acknowledged { id: event_id }
            );
            Err::<(), _>(crate::Error::Other("WS11 rollback".into()))
        })
        .is_err()
    );
    let event = t.read(|conn| AgentEvent::find(conn, event_id)).unwrap();
    assert_eq!(event.outcome.as_deref(), Some("delivered"));
    assert_eq!(event.webhook_status, "pending");
    assert_eq!(event.webhook_attempts, 3);
    assert_eq!(event.webhook_next_attempt_at, Some(t.now()));
}

#[test]
fn ws11_anywhere_capability_reads_live_activity_without_a_grant_cache() {
    use crate::models::agent_access::{capability_for_agent, has_capability_anywhere};
    let t = super::channel_thread_test::frozen();
    t.write(|tx| {
        let gold: Value = serde_json::from_str(include_str!(
            "../../../../vectors/agents_event_access_contract.json"
        ))
        .unwrap();
        let agent = id("bender_agent");
        tx.conn().execute("DELETE FROM agent_grants", [])?;
        for (capability, key) in [
            ("read_messages", "legacy_read"),
            ("dm_anyone", "legacy_dm"),
            ("launch_missiles", "unknown"),
        ] {
            assert_eq!(
                json!(has_capability_anywhere(tx.conn(), agent, capability)?),
                gold["capabilities"][key]
            );
        }
        assert_eq!(
            json!(capability_for_agent(
                tx.conn(),
                agent,
                "read_messages",
                None
            )?),
            gold["capabilities"]["legacy_workspace"]
        );
        tx.conn()
            .execute("UPDATE users SET status=2 WHERE id=?", [id("bender")])?;
        assert_eq!(
            json!(has_capability_anywhere(tx.conn(), agent, "read_messages")?),
            gold["capabilities"]["inactive_user"]
        );
        tx.conn()
            .execute("UPDATE users SET status=0 WHERE id=?", [id("bender")])?;
        tx.conn().execute(
            "UPDATE agents SET suspended_at=? WHERE id=?",
            params![tx.now(), agent],
        )?;
        assert_eq!(
            json!(has_capability_anywhere(tx.conn(), agent, "read_messages")?),
            gold["capabilities"]["suspended"]
        );
        tx.conn()
            .execute("UPDATE agents SET suspended_at=NULL WHERE id=?", [agent])?;
        tx.conn().execute(
            "UPDATE rooms SET deleted_at=? WHERE id=?",
            params![tx.now(), id("watercooler")],
        )?;
        assert_eq!(
            json!(capability_for_agent(
                tx.conn(),
                agent,
                "read_messages",
                Some(id("watercooler"))
            )?),
            gold["capabilities"]["deleted_room"]
        );
        let mut recorded = grant(tx, None)?;
        assert!(has_capability_anywhere(tx.conn(), agent, "read_messages")?);
        recorded.revoke(tx)?;
        assert!(!has_capability_anywhere(tx.conn(), agent, "read_messages")?);
        Ok(())
    });
}
