use super::*;
use crate::models::agent_service::ServiceResult;
use crate::models::agent_step::{self as steps, StepParentChange};
use crate::{
    AgentGrant, AgentStep, AgentStepChanges, ChannelThread, Message, NewAgentStep, NewGrant, Room,
    Timestamp,
};
use rusqlite::params;
use serde_json::{Value, json};

const MESSAGE: i64 = 900060001;
const FOREIGN: i64 = 900060002;
const WORK: i64 = 900060003;
const CHAT: i64 = 900060004;

fn setup() -> TestDb {
    let t = super::channel_thread_test::frozen();
    t.clock
        .travel_to(Timestamp::parse_db("2026-03-02 16:00:00").unwrap());
    t.write(|tx| {
        tx.conn().execute("DELETE FROM agent_grants", [])?;
        tx.conn().execute("DELETE FROM agent_steps", [])?;
        tx.conn().execute("DELETE FROM sqlite_sequence WHERE name='agent_steps'", [])?;
        tx.conn().execute("INSERT INTO sqlite_sequence(name,seq) VALUES ('agent_steps',900050000)", [])?;
        Room::find(tx.conn(), id("watercooler"))?.grant_to(tx, &[id("bender")])?;
        for (message, creator, client) in [(MESSAGE,id("bender"),"ws11-steps-parent"),(FOREIGN,id("david"),"ws11-steps-foreign")] {
            tx.conn().execute("INSERT INTO messages(id,room_id,creator_id,client_message_id,created_at,updated_at) VALUES (?,?,?,?,?,?)", params![message,id("watercooler"),creator,client,tx.now(),tx.now()])?;
        }
        for (thread, name, status, owner) in [(WORK,"Work",Some("in_progress"),Some(id("bender"))),(CHAT,"Chat",None,None)] {
            tx.conn().execute("INSERT INTO channel_threads(id,room_id,creator_id,name,work_status,work_owner_id,last_activity_at,created_at,updated_at) VALUES (?,?,?,?,?,?,?,?,?)",params![thread,id("watercooler"),id("david"),name,status,owner,tx.now(),tx.now(),tx.now()])?;
        }
        Ok(())
    });
    t
}
fn base() -> NewAgentStep {
    NewAgentStep {
        agent_id: id("bender_agent"),
        message_id: Some(MESSAGE),
        name: "Run tests".into(),
        ..Default::default()
    }
}
fn gold() -> Value {
    serde_json::from_str(include_str!(
        "../../../../vectors/agents_step_contract.json"
    ))
    .unwrap()
}
fn errors(e: crate::Errors) -> Value {
    let mut result = serde_json::Map::new();
    for (field, message) in e.0 {
        result
            .entry(field.to_owned())
            .or_insert_with(|| json!([]))
            .as_array_mut()
            .unwrap()
            .push(json!(message));
    }
    Value::Object(result)
}
fn parent_change(event: &Event) -> Option<StepParentChange> {
    match event {
        Event::Broadcast(request) => request.decode::<StepParentChange>().and_then(|r| r.ok()),
        _ => None,
    }
}
fn check(key: &str, result: ServiceResult) {
    assert_eq!(
        json!({"status":result.status,"payload":result.payload,"error":result.error}),
        gold()["results"][key],
        "{key}"
    );
}

#[test]
fn ws11_step_model_validation_ownership_sizes_and_optional_orphan_match_rails() {
    let t = setup();
    t.write(|tx| {
        for (key, a) in [
            (
                "blank",
                NewAgentStep {
                    name: "".into(),
                    ..base()
                },
            ),
            (
                "no_parent",
                NewAgentStep {
                    message_id: None,
                    ..base()
                },
            ),
            (
                "both",
                NewAgentStep {
                    channel_thread_id: Some(WORK),
                    ..base()
                },
            ),
            (
                "foreign",
                NewAgentStep {
                    message_id: Some(FOREIGN),
                    ..base()
                },
            ),
            (
                "unowned",
                NewAgentStep {
                    message_id: None,
                    channel_thread_id: Some(CHAT),
                    ..base()
                },
            ),
            (
                "oversized",
                NewAgentStep {
                    name: "é".repeat(121),
                    input_summary: Some("é".repeat(1001)),
                    output_summary: Some("é".repeat(1001)),
                    status: "exploding".into(),
                    duration_ms: Some(-1),
                    ..base()
                },
            ),
            (
                "boundary",
                NewAgentStep {
                    name: "é".repeat(120),
                    input_summary: Some("é".repeat(1000)),
                    duration_ms: Some(0),
                    ..base()
                },
            ),
            (
                "blank_summary",
                NewAgentStep {
                    input_summary: Some(" ".repeat(2000)),
                    ..base()
                },
            ),
            (
                "orphan",
                NewAgentStep {
                    message_id: Some(-1),
                    ..base()
                },
            ),
        ] {
            assert_eq!(
                errors(AgentStep::validate(tx.conn(), &a, None)?),
                gold()["validation"][key],
                "{key}"
            );
        }
        tx.conn().execute(
            "UPDATE channel_threads SET work_owner_id=? WHERE id=?",
            params![id("david"), WORK],
        )?;
        assert!(
            !AgentStep::validate(
                tx.conn(),
                &NewAgentStep {
                    message_id: None,
                    channel_thread_id: Some(WORK),
                    ..base()
                },
                None
            )?
            .is_empty()
        );
        tx.conn().execute(
            "UPDATE channel_threads SET work_status=' ', work_owner_id=? WHERE id=?",
            params![id("bender"), WORK],
        )?;
        assert!(
            !AgentStep::validate(
                tx.conn(),
                &NewAgentStep {
                    message_id: None,
                    channel_thread_id: Some(WORK),
                    ..base()
                },
                None
            )?
            .is_empty()
        );
        Ok(())
    });
}

#[test]
fn ws11_step_services_payload_grants_denial_order_and_parent_broadcasts_match_rails() {
    let t = setup();
    t.write(|tx| {
        let agent = id("bender_agent");
        check(
            "created",
            steps::create(
                tx,
                agent,
                NewAgentStep {
                    input_summary: Some(" ".into()),
                    duration_ms: Some(1200),
                    ..base()
                },
            )?,
        );
        check(
            "updated",
            steps::update(
                tx,
                agent,
                900050001,
                AgentStepChanges {
                    status: Some("done".into()),
                    output_summary: Some(Some("".into())),
                    duration_ms: Some(Some(1500)),
                    ..Default::default()
                },
            )?,
        );
        check(
            "no_parent",
            steps::create(
                tx,
                agent,
                NewAgentStep {
                    message_id: None,
                    ..base()
                },
            )?,
        );
        check(
            "foreign",
            steps::create(
                tx,
                agent,
                NewAgentStep {
                    message_id: Some(FOREIGN),
                    ..base()
                },
            )?,
        );
        check(
            "missing_step",
            steps::update(tx, agent, 0, Default::default())?,
        );
        let mut grant = AgentGrant::create(
            tx,
            NewGrant {
                agent_id: agent,
                granted_by_id: id("david"),
                room_id: Some(id("watercooler")),
                capability: "manage_threads".into(),
                ..Default::default()
            },
        )?;
        check(
            "thread",
            steps::create(
                tx,
                agent,
                NewAgentStep {
                    message_id: None,
                    channel_thread_id: Some(WORK),
                    name: "Reproduce".into(),
                    output_summary: Some("Found".into()),
                    ..base()
                },
            )?,
        );
        check("missing_post_grant", steps::create(tx, agent, base())?);
        check(
            "revoked_update",
            steps::update(
                tx,
                agent,
                900050001,
                AgentStepChanges {
                    name: Some("Nope".into()),
                    ..Default::default()
                },
            )?,
        );
        grant.revoke(tx)?;
        check(
            "revoked_thread",
            steps::create(
                tx,
                agent,
                NewAgentStep {
                    message_id: None,
                    channel_thread_id: Some(WORK),
                    ..base()
                },
            )?,
        );
        Ok(())
    });
    let parents: Vec<_> = t
        .events()
        .iter()
        .filter_map(parent_change)
        .map(|p| (p.message_id, p.thread_id))
        .collect();
    assert_eq!(
        parents,
        vec![
            (Some(MESSAGE), None),
            (Some(MESSAGE), None),
            (None, Some(WORK))
        ]
    );
}

#[test]
fn ws11_step_parent_limit_and_monotonic_positions_survive_deletion_and_competing_writers() {
    let t = setup();
    t.write(|tx| {
        for _ in 0..49 {
            AgentStep::create(tx, base())?;
        }
        Ok(())
    });
    let handles: Vec<_> = (0..4)
        .map(|_| {
            let db = t.db.clone();
            std::thread::spawn(move || {
                db.write_blocking(|tx| steps::create(tx, id("bender_agent"), base()))
                    .unwrap()
                    .status
            })
        })
        .collect();
    let statuses: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    assert_eq!(statuses.iter().filter(|&&s| s == 201).count(), 1);
    assert_eq!(statuses.iter().filter(|&&s| s == 422).count(), 3);
    t.write(|tx| {
        assert_eq!(
            errors(AgentStep::validate(tx.conn(), &base(), None)?),
            gold()["validation"]["limit"]
        );
        tx.conn().execute(
            "DELETE FROM agent_steps WHERE id=(SELECT MIN(id) FROM agent_steps WHERE message_id=?)",
            [MESSAGE],
        )?;
        let replacement = AgentStep::create(tx, base())?;
        let count: i64 = tx.conn().query_row(
            "SELECT COUNT(*) FROM agent_steps WHERE message_id=?",
            [MESSAGE],
            |r| r.get(0),
        )?;
        assert_eq!(
            json!({"replacement":replacement.position,"count":count}),
            gold()["positions"]
        );
        Ok(())
    });
}

#[test]
fn ws11_step_update_immutable_parent_noop_and_live_ownership_rechecks() {
    let t = setup();
    let step = t.write(|tx| {
        let step = AgentStep::create(tx, base())?;
        assert_eq!(
            steps::update(tx, id("bender_agent"), step.id, Default::default())?.status,
            200
        );
        assert_eq!(
            AgentStep::find(tx.conn(), step.id)?.unwrap().updated_at,
            step.updated_at
        );
        tx.conn().execute(
            "UPDATE messages SET creator_id=? WHERE id=?",
            params![id("david"), MESSAGE],
        )?;
        assert_eq!(
            steps::update(tx, id("bender_agent"), step.id, Default::default())?.status,
            403
        );
        tx.conn().execute(
            "UPDATE messages SET creator_id=? WHERE id=?",
            params![id("bender"), MESSAGE],
        )?;
        tx.conn().execute(
            "UPDATE rooms SET deleted_at=? WHERE id=?",
            params![tx.now(), id("watercooler")],
        )?;
        assert_eq!(steps::create(tx, id("bender_agent"), base())?.status, 404);
        assert_eq!(
            steps::update(tx, id("bender_agent"), step.id, Default::default())?.status,
            403
        );
        Ok(step)
    });
    assert_eq!(
        t.events()
            .iter()
            .filter(|e| parent_change(e).is_some())
            .count(),
        1
    );
    t.read(|conn| {
        assert_eq!(
            AgentStep::find(conn, step.id)?.unwrap().message_id,
            Some(MESSAGE)
        );
        Ok(())
    });
}

#[test]
fn ws11_step_parent_destruction_cascades_and_transaction_rollback_drops_broadcasts() {
    let t = setup();
    let count = t.events().len();
    assert!(
        t.try_write(|tx| {
            steps::create(tx, id("bender_agent"), base())?;
            Err::<(), _>(crate::Error::Other("WS11 rollback".into()))
        })
        .is_err()
    );
    assert_eq!(t.events().len(), count);
    t.write(|tx| {
        assert!(!crate::sql::exists(
            tx.conn(),
            "SELECT 1 FROM agent_steps",
            []
        )?);
        AgentStep::create(tx, base())?;
        AgentStep::create(
            tx,
            NewAgentStep {
                message_id: None,
                channel_thread_id: Some(WORK),
                ..base()
            },
        )?;
        Message::find_by_id(tx.conn(), MESSAGE)?
            .unwrap()
            .destroy(tx)?;
        assert!(!crate::sql::exists(
            tx.conn(),
            "SELECT 1 FROM agent_steps WHERE message_id=?",
            [MESSAGE]
        )?);
        assert!(crate::sql::exists(
            tx.conn(),
            "SELECT 1 FROM agent_steps WHERE channel_thread_id=?",
            [WORK]
        )?);
        ChannelThread::find_by_id(tx.conn(), WORK)?
            .unwrap()
            .destroy(tx)?;
        assert!(!crate::sql::exists(
            tx.conn(),
            "SELECT 1 FROM agent_steps",
            []
        )?);
        Ok(())
    });
}

#[test]
fn ws11_step_service_validation_failure_preserves_parent_and_structured_errors() {
    let t = setup();
    t.write(|tx| {
        let result = steps::create(
            tx,
            id("bender_agent"),
            NewAgentStep {
                name: "".into(),
                ..base()
            },
        )?;
        assert_eq!(result.status, 422);
        assert_eq!(result.error.as_deref(), Some("Name can't be blank"));
        assert_eq!(
            result.payload,
            Some(json!({"errors":{"name":["can't be blank"]}}))
        );
        let step = AgentStep::create(tx, base())?;
        let result = steps::update(
            tx,
            id("bender_agent"),
            step.id,
            AgentStepChanges {
                status: Some("explode".into()),
                ..Default::default()
            },
        )?;
        assert_eq!(result.status, 422);
        assert_eq!(
            result.payload,
            Some(json!({"errors":{"status":["is not included in the list"]}}))
        );
        assert_eq!(
            AgentStep::find(tx.conn(), step.id)?.unwrap().status,
            "running"
        );
        Ok(())
    });
    assert!(!t.events().iter().any(|e| parent_change(e).is_some()));
}
