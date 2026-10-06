//! Five agent invocation cases from the 40-case Rails dispatcher file.
//! Built-in commands and their UI remain WS8 comparisons.
use super::*;
use crate::models::agent_delivery::AgentEvent;
use crate::slash_commands::{Context, dispatch};
use crate::{
    AgentGrant, AgentSlashCommand, ChannelThread, NewAgentSlashCommand,
    NewChannelThread, NewGrant,
};
fn setup() -> TestDb {
    let t = super::channel_thread_test::frozen();
    t.write(|tx| {
        tx.conn().execute("DELETE FROM agent_slash_commands", [])?;
        tx.conn().execute("DELETE FROM agent_grants", [])?;
        tx.conn().execute("DELETE FROM agent_events", [])?;
        AgentSlashCommand::create(
            tx,
            NewAgentSlashCommand {
                agent_id: id("bender_agent"),
                room_id: id("watercooler"),
                name: "deploy".into(),
                description: Some("Ship it".into()),
                ..Default::default()
            },
        )?;
        Ok(())
    });
    t
}
fn context() -> Context {
    Context {
        user_id: id("david"),
        room_id: id("watercooler"),
        thread_id: None,
        huddles_configured: false,
    }
}
fn latest(tx: &Tx<'_>) -> Result<AgentEvent> {
    let eid = tx.conn().query_row(
        "SELECT MAX(id) FROM agent_events WHERE agent_id=?",
        [id("bender_agent")],
        |r| r.get::<_, i64>(0),
    )?;
    Ok(AgentEvent::find(tx.conn(), eid)?.unwrap())
}
#[test]
fn ws11_dispatcher_case_invoking_an_agent_command_delivers_a_slash_command_event() {
    let t = setup();
    t.write(|tx| {
        let result = dispatch(tx, &context(), "/deploy staging")?;
        assert_eq!(result.kind, "ephemeral");
        assert_eq!(result.message.as_deref(), Some("Sent to Bender Bot"));
        let e = latest(tx)?;
        assert_eq!(e.event_type, "slash_command");
        assert_eq!(e.room_id, Some(id("watercooler")));
        assert_eq!(e.actor_id, Some(id("david")));
        assert_eq!(e.metadata["command"], "deploy");
        assert_eq!(e.metadata["arguments"], "staging");
        assert_eq!(
            tx.conn().query_row(
                "SELECT COUNT(*) FROM agent_events WHERE agent_id=?",
                [e.agent_id],
                |r| r.get::<_, i64>(0)
            )?,
            1
        );
        Ok(())
    });
}
#[test]
fn ws11_dispatcher_case_invoking_an_agent_command_in_a_thread_records_the_thread() {
    let t = setup();
    t.write(|tx| {
        let thread = ChannelThread::create(
            tx,
            NewChannelThread {
                room_id: id("watercooler"),
                creator_id: id("david"),
                name: Some("Side chat".into()),
                ..Default::default()
            },
        )?;
        let result = dispatch(
            tx,
            &Context {
                thread_id: Some(thread.id),
                ..context()
            },
            "/deploy staging",
        )?;
        assert_eq!(result.kind, "ephemeral");
        assert_eq!(latest(tx)?.metadata["thread_id"], thread.id);
        Ok(())
    });
}
#[test]
fn ws11_dispatcher_case_invoking_an_agent_command_in_the_channel_records_no_thread() {
    let t = setup();
    t.write(|tx| {
        dispatch(tx, &context(), "/deploy staging")?;
        assert!(latest(tx)?.metadata["thread_id"].is_null());
        Ok(())
    });
}
#[test]
fn ws11_dispatcher_case_invoking_an_agent_command_requires_post_messages() {
    let t = setup();
    t.write(|tx| {
        AgentGrant::create(
            tx,
            NewGrant {
                agent_id: id("bender_agent"),
                room_id: Some(id("watercooler")),
                granted_by_id: id("david"),
                capability: "read_messages".into(),
                ..Default::default()
            },
        )?;
        let result = dispatch(tx, &context(), "/deploy staging")?;
        assert_eq!(result.kind, "error");
        assert!(result.message.unwrap().contains("no longer available"));
        assert_eq!(
            tx.conn().query_row(
                "SELECT COUNT(*) FROM agent_events WHERE agent_id=?",
                [id("bender_agent")],
                |r| r.get::<_, i64>(0)
            )?,
            0
        );
        Ok(())
    });
}
#[test]
fn ws11_dispatcher_case_invoking_an_agent_command_is_rate_limited() {
    let t = setup();
    t.write(|tx| {
        for _ in 0..20 {
            assert_eq!(
                dispatch(tx, &context(), "/deploy staging")?.kind,
                "ephemeral"
            );
        }
        let result = dispatch(tx, &context(), "/deploy staging")?;
        assert_eq!(result.kind, "error");
        assert!(result.message.unwrap().contains("too many"));
        assert_eq!(
            tx.conn().query_row(
                "SELECT COUNT(*) FROM agent_events WHERE agent_id=?",
                [id("bender_agent")],
                |r| r.get::<_, i64>(0)
            )?,
            20
        );
        Ok(())
    });
}
