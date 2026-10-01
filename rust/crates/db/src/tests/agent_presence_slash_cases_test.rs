//! Individual source cases pinned to presence_slash_named_cases.rb.
use super::*;
use crate::models::agent_working_presence;
use crate::{Agent, AgentKind, AgentSlashCommand, NewAgent, NewAgentSlashCommand, Timestamp, User};
use serde_json::{Value, json};
fn gold(section: &str, key: &str) -> Value {
    let all: Value = serde_json::from_str(include_str!(
        "../../../../vectors/agents_presence_slash_named_cases.json"
    ))
    .unwrap();
    all[section][key].clone()
}
fn setup() -> TestDb {
    let t = super::channel_thread_test::frozen();
    t.clock
        .travel_to(Timestamp::parse_db("2026-03-02 16:00:00").unwrap());
    t.write(|tx| {
        tx.conn().execute("DELETE FROM agent_slash_commands", [])?;
        Ok(())
    });
    t
}
fn presence(conn: &Connection, now: Timestamp) -> Result<Value> {
    let a = Agent::find(conn, id("bender_agent"))?.unwrap();
    Ok(
        json!({"text":a.working_presence_text(now),"stored":a.working_presence,"expires_at":a.working_presence_expires_at.map(crate::models::agent_payloads::json_time)}),
    )
}
fn errors(e: crate::Errors) -> Value {
    let mut map = serde_json::Map::new();
    for (field, message) in e.0 {
        map.entry(field.to_owned())
            .or_insert_with(|| json!([]))
            .as_array_mut()
            .unwrap()
            .push(json!(message));
    }
    Value::Object(map)
}
fn service(r: crate::models::agent_service::ServiceResult) -> Value {
    json!({"status":r.status,"payload":r.payload,"error":r.error})
}
#[test]
fn ws11_presence_case_ttl() {
    let t = setup();
    t.write(|tx| {
        Agent::find(tx.conn(), id("bender_agent"))?
            .unwrap()
            .set_working_presence(tx, Some("Running tests…"))?;
        assert_eq!(presence(tx.conn(), tx.now())?, gold("presence", "ttl"));
        Ok(())
    });
}
#[test]
fn ws11_presence_case_blank_clears() {
    let t = setup();
    t.write(|tx| {
        let mut a = Agent::find(tx.conn(), id("bender_agent"))?.unwrap();
        a.set_working_presence(tx, Some("Thinking…"))?;
        a.set_working_presence(tx, Some(""))?;
        assert_eq!(presence(tx.conn(), tx.now())?, gold("presence", "blank"));
        Ok(())
    });
}
#[test]
fn ws11_presence_case_expired_reads_cleared() {
    let t = setup();
    t.write(|tx| {
        Agent::find(tx.conn(), id("bender_agent"))?
            .unwrap()
            .set_working_presence(tx, Some("Thinking…"))
    });
    t.travel(360);
    t.read(|conn| {
        assert_eq!(presence(conn, t.now())?, gold("presence", "expired"));
        Ok(())
    });
}
#[test]
fn ws11_presence_case_model_limit() {
    let t = setup();
    t.read(|conn| {
        let a = crate::NewAgent {
            user_id: id("bender"),
            owner_id: Some(id("david")),
            working_presence: Some("x".repeat(141)),
            ..Default::default()
        };
        assert_eq!(
            errors(Agent::validate(conn, &a, Some(id("bender_agent")))?),
            gold("presence", "limit")
        );
        Ok(())
    });
}
#[test]
fn ws11_presence_case_shared_set_clear() {
    let t = setup();
    t.write(|tx| {
        let set = service(agent_working_presence::set(
            tx,
            id("bender_agent"),
            Some("Thinking…"),
        )?);
        let clear = service(agent_working_presence::set(
            tx,
            id("bender_agent"),
            Some(""),
        )?);
        assert_eq!(json!([set, clear]), gold("presence", "service"));
        Ok(())
    });
}
#[test]
fn ws11_presence_case_shared_limit() {
    let t = setup();
    t.write(|tx| {
        assert_eq!(
            service(agent_working_presence::set(
                tx,
                id("bender_agent"),
                Some(&"x".repeat(141))
            )?),
            gold("presence", "service_limit")
        );
        Ok(())
    });
}
fn command() -> NewAgentSlashCommand {
    NewAgentSlashCommand {
        agent_id: id("bender_agent"),
        room_id: id("watercooler"),
        name: "Deploy".into(),
        description: Some(" Ship it ".into()),
        ..Default::default()
    }
}
fn command_snapshot(c: AgentSlashCommand) -> Value {
    json!({"name":c.name,"description":c.description,"takes_arguments":c.takes_arguments})
}
#[test]
fn ws11_slash_model_case_registers() {
    let t = setup();
    t.write(|tx| {
        assert_eq!(
            command_snapshot(AgentSlashCommand::create(tx, command())?),
            gold("commands", "register")
        );
        Ok(())
    });
}
#[test]
fn ws11_slash_model_case_default_arguments() {
    let t = setup();
    t.write(|tx| {
        let c = AgentSlashCommand::create(
            tx,
            NewAgentSlashCommand {
                description: None,
                ..command()
            },
        )?;
        assert_eq!(
            json!(c.takes_arguments),
            gold("commands", "register")["takes_arguments"]
        );
        Ok(())
    });
}
#[test]
fn ws11_slash_model_case_room_unique_across_agents() {
    let t = setup();
    t.write(|tx| {
        AgentSlashCommand::create(tx, command())?;
        let user = User::create_bot(tx, "Other Bot", None)?;
        let other = Agent::create(
            tx,
            NewAgent {
                user_id: user.id,
                owner_id: Some(id("david")),
                kind: AgentKind::Workspace,
                ..Default::default()
            },
        )?;
        assert_eq!(
            errors(AgentSlashCommand::validate(
                tx.conn(),
                &NewAgentSlashCommand {
                    agent_id: other.id,
                    name: "deploy".into(),
                    ..command()
                },
                None
            )?),
            gold("commands", "duplicate")
        );
        Ok(())
    });
}
#[test]
fn ws11_slash_model_case_command_words() {
    let t = setup();
    t.read(|conn| {
        let mut all = vec![];
        for name in [
            "Deploy!".into(),
            "/deploy".into(),
            "two words".into(),
            "x".repeat(33),
        ] {
            all.push(errors(AgentSlashCommand::validate(
                conn,
                &NewAgentSlashCommand { name, ..command() },
                None,
            )?));
        }
        assert_eq!(json!(all), gold("commands", "invalid"));
        Ok(())
    });
}
#[test]
fn ws11_slash_model_case_no_builtin_shadow() {
    let t = setup();
    t.read(|conn| {
        assert_eq!(
            errors(AgentSlashCommand::validate(
                conn,
                &NewAgentSlashCommand {
                    name: "poll".into(),
                    description: None,
                    ..command()
                },
                None
            )?),
            gold("commands", "builtin")
        );
        Ok(())
    });
}
#[test]
fn ws11_slash_model_case_description_limit() {
    let t = setup();
    t.read(|conn| {
        assert_eq!(
            errors(AgentSlashCommand::validate(
                conn,
                &NewAgentSlashCommand {
                    name: "inspect".into(),
                    description: Some("x".repeat(141)),
                    ..command()
                },
                None
            )?),
            gold("commands", "description")
        );
        Ok(())
    });
}
