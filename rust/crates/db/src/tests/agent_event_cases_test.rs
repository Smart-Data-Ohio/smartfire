//! AgentEventTest: model validation and scopes are independent of polling access.
use super::*;
use crate::Agent;
use crate::models::agent_delivery::DELIVERABLE_TYPES;
use crate::models::agent_delivery::{AgentEvent, NewEvent};
use serde_json::json;
fn oracle(key: &str) -> serde_json::Value {
    let v: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../vectors/agents_event_model_contract.json"
    ))
    .unwrap();
    v[key].clone()
}
fn setup() -> TestDb {
    let t = TestDb::new();
    t.write(|tx| {
        tx.conn().execute("DELETE FROM agent_events", [])?;
        Ok(())
    });
    t
}
fn base(kind: &str) -> NewEvent {
    NewEvent {
        agent_id: id("bender_agent"),
        event_type: kind.into(),
        outcome: Some("pending".into()),
        ..Default::default()
    }
}
#[test]
fn ws11_event_case_requires_known_type() {
    let t = setup();
    let Err(crate::Error::RecordInvalid(e)) =
        t.try_write(|tx| AgentEvent::create(tx, base("launch_missiles")))
    else {
        panic!("invalid kind accepted")
    };
    assert!(e.0.contains(&("event_type", "is not included in the list".into())));
}
#[test]
fn ws11_event_case_accepts_documented_types() {
    let t = setup();
    t.write(|tx| {
        let vocabulary: Vec<_> = DELIVERABLE_TYPES
            .into_iter()
            .chain([
                "delivery_suppressed_rate_limit",
                "delivery_suppressed_hop_limit",
                "delivery_suppressed_revoked",
                "posted",
            ])
            .collect();
        assert_eq!(json!(vocabulary), oracle("types"));
        for kind in oracle("types").as_array().unwrap() {
            let kind = kind.as_str().unwrap();
            assert_eq!(AgentEvent::create(tx, base(kind))?.event_type, kind);
        }
        Ok(())
    });
}
#[test]
fn ws11_event_case_requires_known_outcome() {
    let t = setup();
    let Err(crate::Error::RecordInvalid(e)) = t.try_write(|tx| {
        AgentEvent::create(
            tx,
            NewEvent {
                outcome: Some("bogus".into()),
                ..base("mention")
            },
        )
    }) else {
        panic!("invalid outcome accepted")
    };
    assert!(e.0.contains(&("outcome", "is not included in the list".into())));
}
#[test]
fn ws11_event_case_optional_associations() {
    let t = setup();
    let e = t.write(|tx| AgentEvent::create(tx, base("mention")));
    assert!(
        e.room_id.is_none()
            && e.message_id.is_none()
            && e.actor_id.is_none()
            && e.agent_credential_id.is_none()
    );
}
fn kinds(rows: Vec<AgentEvent>) -> Vec<String> {
    let mut v: Vec<_> = rows.into_iter().map(|e| e.event_type).collect();
    v.sort();
    v
}
fn scope_test(message: bool) {
    let t = setup();
    t.write(|tx| {
        for kind in [
            "mention",
            "direct_message",
            "reply",
            "approval_decided",
            "work_assigned",
            "work_unassigned",
            "github_action_completed",
        ] {
            AgentEvent::create(tx, base(kind))?;
        }
        for kind in [
            "delivery_suppressed_rate_limit",
            "delivery_suppressed_hop_limit",
            "delivery_suppressed_revoked",
            "posted",
        ] {
            AgentEvent::create(
                tx,
                NewEvent {
                    outcome: Some("suppressed".into()),
                    ..base(kind)
                },
            )?;
        }
        Ok(())
    });
    let actual = t.read(move |c| {
        if message {
            AgentEvent::message_deliverable_for_agent(c, id("bender_agent"))
        } else {
            AgentEvent::deliverable_for_agent(c, id("bender_agent"))
        }
    });
    assert_eq!(
        json!(kinds(actual)),
        oracle(if message {
            "message_deliverable"
        } else {
            "deliverable"
        })
    );
}
#[test]
fn ws11_event_case_deliverable_scope() {
    scope_test(false);
}
#[test]
fn ws11_event_case_message_deliverable_scope() {
    scope_test(true);
}
#[test]
fn ws11_event_case_hop_defaults_and_metadata() {
    let t = setup();
    t.write(|tx| {
        assert_eq!(AgentEvent::create(tx, base("mention"))?.hop(), 0);
        let e = AgentEvent::create(
            tx,
            NewEvent {
                metadata: json!({"hop":2}),
                ..base("mention")
            },
        )?;
        assert_eq!(e.hop(), 2);
        assert_eq!(AgentEvent::find(tx.conn(), e.id)?.unwrap().hop, 2);
        Ok(())
    });
}
#[test]
fn ws11_event_case_acknowledgment_idempotent() {
    let t = setup();
    let eid = t.write(|tx| {
        Ok(AgentEvent::create(
            tx,
            NewEvent {
                outcome: Some("delivered".into()),
                ..base("mention")
            },
        )?
        .id)
    });
    t.write(move|tx|{let mut e=AgentEvent::find(tx.conn(),eid)?.unwrap();e.acknowledge(tx)?;assert_eq!(AgentEvent::find(tx.conn(),eid)?.unwrap().outcome.as_deref(),Some("acknowledged"));tx.conn().execute_batch("CREATE TEMP TRIGGER ws11_reject_repeat_ack BEFORE UPDATE ON agent_events BEGIN SELECT RAISE(ABORT,'repeat ack write'); END")?;e.acknowledge(tx)?;assert_eq!(AgentEvent::find(tx.conn(),eid)?.unwrap().outcome.as_deref(),Some("acknowledged"));Ok(())});
}
#[test]
fn ws11_event_case_destroying_agent_removes_events() {
    let t = setup();
    t.write(|tx| {
        AgentEvent::create(tx, base("mention"))?;
        Agent::find(tx.conn(), id("bender_agent"))?
            .unwrap()
            .destroy(tx)?;
        assert!(AgentEvent::for_agent(tx.conn(), id("bender_agent"))?.is_empty());
        Ok(())
    });
}
#[test]
fn ws11_event_case_agent_exposes_own_ledger() {
    let t = setup();
    t.write(|tx| {
        let e = AgentEvent::create(tx, base("mention"))?;
        let own = AgentEvent::for_agent(tx.conn(), id("bender_agent"))?;
        assert_eq!(own.len(), 1);
        assert_eq!(own[0].id, e.id);
        assert!(AgentEvent::for_agent(tx.conn(), -1)?.is_empty());
        Ok(())
    });
}
