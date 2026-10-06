//! Every AgentStepTest case exercises the model, its validators and parent deletion.
use super::*;
use crate::{AgentStep, ChannelThread, Message, NewAgentStep, NewChannelThread, NewMessage};
use serde_json::{Value, json};
fn setup() -> (TestDb, i64) {
    let t = super::channel_thread_test::frozen();
    let m = t.write(|tx| {
        tx.conn().execute("DELETE FROM agent_steps", [])?;
        Ok(Message::create(
            tx,
            NewMessage {
                room_id: id("watercooler"),
                creator_id: id("bender"),
                markdown_source: Some("Working on it".into()),
                ..Default::default()
            },
        )?
        .id)
    });
    (t, m)
}
fn base(mid: i64) -> NewAgentStep {
    NewAgentStep {
        agent_id: id("bender_agent"),
        message_id: Some(mid),
        name: "Run tests".into(),
        ..Default::default()
    }
}
fn thread(tx: &mut Tx<'_>, owner: Option<i64>, work: bool) -> Result<i64> {
    let t = ChannelThread::create(
        tx,
        NewChannelThread {
            room_id: id("watercooler"),
            creator_id: id("david"),
            name: Some("Fix it".into()),
            work_status: work.then(|| "in_progress".into()),
            ..Default::default()
        },
    )?;
    tx.conn().execute(
        "UPDATE channel_threads SET work_owner_id=? WHERE id=?",
        rusqlite::params![owner, t.id],
    )?;
    Ok(t.id)
}
fn errors(e: crate::Errors) -> Value {
    let mut v = serde_json::Map::new();
    for (field, message) in e.0 {
        v.entry(field.to_owned())
            .or_insert_with(|| json!([]))
            .as_array_mut()
            .unwrap()
            .push(json!(message));
    }
    Value::Object(v)
}
fn oracle(key: &str) -> Value {
    let v: Value = serde_json::from_str(include_str!(
        "../../../../vectors/agents_step_contract.json"
    ))
    .unwrap();
    v["validation"][key].clone()
}
#[test]
fn ws11_step_case_attaches_to_own_message() {
    let (t, m) = setup();
    t.write(move |tx| {
        let step = AgentStep::create(
            tx,
            NewAgentStep {
                status: "running".into(),
                input_summary: Some("bundle exec rails test".into()),
                duration_ms: Some(1200),
                ..base(m)
            },
        )?;
        assert_eq!(step.message_id, Some(m));
        assert!(step.channel_thread_id.is_none());
        assert_eq!(step.position, 0);
        assert_eq!(step.status, "running");
        Ok(())
    });
}
#[test]
fn ws11_step_case_attaches_to_owned_work() {
    let (t, _) = setup();
    t.write(|tx| {
        let tid = thread(tx, Some(id("bender")), true)?;
        let step = AgentStep::create(
            tx,
            NewAgentStep {
                message_id: None,
                channel_thread_id: Some(tid),
                name: "Reproduce".into(),
                ..base(0)
            },
        )?;
        assert_eq!(step.channel_thread_id, Some(tid));
        assert!(step.message_id.is_none());
        Ok(())
    });
}
#[test]
fn ws11_step_case_exactly_one_parent() {
    let (t, m) = setup();
    t.write(move |tx| {
        let tid = thread(tx, Some(id("bender")), true)?;
        for (key, a) in [
            (
                "no_parent",
                NewAgentStep {
                    message_id: None,
                    ..base(m)
                },
            ),
            (
                "both",
                NewAgentStep {
                    channel_thread_id: Some(tid),
                    ..base(m)
                },
            ),
        ] {
            assert_eq!(
                errors(AgentStep::validate(tx.conn(), &a, None)?),
                oracle(key)
            );
            assert!(matches!(
                AgentStep::create(tx, a),
                Err(crate::Error::RecordInvalid(_))
            ));
        }
        Ok(())
    });
}
#[test]
fn ws11_step_case_rejects_foreign_message() {
    let (t, _) = setup();
    t.write(|tx| {
        let m = Message::create(
            tx,
            NewMessage {
                room_id: id("watercooler"),
                creator_id: id("david"),
                markdown_source: Some("Mine".into()),
                ..Default::default()
            },
        )?;
        let a = base(m.id);
        assert_eq!(
            errors(AgentStep::validate(tx.conn(), &a, None)?),
            oracle("foreign")
        );
        assert!(matches!(
            AgentStep::create(tx, a),
            Err(crate::Error::RecordInvalid(_))
        ));
        Ok(())
    });
}
#[test]
fn ws11_step_case_rejects_unowned_thread() {
    let (t, _) = setup();
    t.write(|tx| {
        let tid = thread(tx, Some(id("david")), true)?;
        let a = NewAgentStep {
            message_id: None,
            channel_thread_id: Some(tid),
            ..base(0)
        };
        assert_eq!(
            errors(AgentStep::validate(tx.conn(), &a, None)?),
            oracle("unowned")
        );
        assert!(matches!(
            AgentStep::create(tx, a),
            Err(crate::Error::RecordInvalid(_))
        ));
        Ok(())
    });
}
#[test]
fn ws11_step_case_rejects_nonwork_thread() {
    let (t, _) = setup();
    t.write(|tx| {
        let tid = thread(tx, None, false)?;
        let a = NewAgentStep {
            message_id: None,
            channel_thread_id: Some(tid),
            ..base(0)
        };
        assert_eq!(
            errors(AgentStep::validate(tx.conn(), &a, None)?),
            oracle("unowned")
        );
        assert!(matches!(
            AgentStep::create(tx, a),
            Err(crate::Error::RecordInvalid(_))
        ));
        Ok(())
    });
}
#[test]
fn ws11_step_case_count_capped_per_parent() {
    let (t, m) = setup();
    t.write(move |tx| {
        for n in 0..50 {
            AgentStep::create(
                tx,
                NewAgentStep {
                    name: format!("Step {n}"),
                    ..base(m)
                },
            )?;
        }
        let a = base(m);
        assert_eq!(
            errors(AgentStep::validate(tx.conn(), &a, None)?),
            oracle("limit")
        );
        assert!(matches!(
            AgentStep::create(tx, a),
            Err(crate::Error::RecordInvalid(_))
        ));
        Ok(())
    });
}
#[test]
fn ws11_step_case_field_sizes_capped() {
    let (t, m) = setup();
    t.write(move |tx| {
        let a = NewAgentStep {
            name: "x".repeat(121),
            input_summary: Some("x".repeat(1001)),
            output_summary: Some("x".repeat(1001)),
            status: "exploding".into(),
            duration_ms: Some(-1),
            ..base(m)
        };
        assert_eq!(
            errors(AgentStep::validate(tx.conn(), &a, None)?),
            oracle("oversized")
        );
        assert!(matches!(
            AgentStep::create(tx, a),
            Err(crate::Error::RecordInvalid(_))
        ));
        Ok(())
    });
}
#[test]
fn ws11_step_case_positions_follow_creation() {
    let (t, m) = setup();
    t.write(move |tx| {
        let a = AgentStep::create(
            tx,
            NewAgentStep {
                name: "First".into(),
                ..base(m)
            },
        )?;
        let b = AgentStep::create(
            tx,
            NewAgentStep {
                name: "Second".into(),
                ..base(m)
            },
        )?;
        let mut q = tx
            .conn()
            .prepare("SELECT id FROM agent_steps WHERE message_id=? ORDER BY position,id")?;
        let actual = q
            .query_map([m], |r| r.get::<_, i64>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        assert_eq!(actual, vec![a.id, b.id]);
        assert_eq!([a.position, b.position], [0, 1]);
        Ok(())
    });
}
#[test]
fn ws11_step_case_parent_deletion_cascades() {
    let (t, m) = setup();
    t.write(move |tx| {
        let tid = thread(tx, Some(id("bender")), true)?;
        let a = AgentStep::create(tx, base(m))?;
        let b = AgentStep::create(
            tx,
            NewAgentStep {
                message_id: None,
                channel_thread_id: Some(tid),
                ..base(0)
            },
        )?;
        Message::find(tx.conn(), m)?.destroy(tx)?;
        assert!(AgentStep::find(tx.conn(), a.id)?.is_none());
        assert!(AgentStep::find(tx.conn(), b.id)?.is_some());
        ChannelThread::find(tx.conn(), tid)?.destroy(tx)?;
        assert!(AgentStep::find(tx.conn(), b.id)?.is_none());
        Ok(())
    });
}
