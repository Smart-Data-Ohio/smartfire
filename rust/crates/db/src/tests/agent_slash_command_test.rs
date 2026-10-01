use super::*;
use crate::models::agent_slash_command as commands;
use crate::slash_commands::{self as slash, Context};
use crate::{
    Agent, AgentGrant, AgentKind, AgentSlashCommand, NewAgent, NewAgentSlashCommand, NewGrant,
    Room, Timestamp,
};
use rusqlite::params;
use serde_json::{Value, json};

fn frozen() -> TestDb {
    let t = super::channel_thread_test::frozen();
    t.clock
        .travel_to(Timestamp::parse_db("2026-03-02 16:00:00").unwrap());
    t.write(|tx| {
        tx.conn().execute("DELETE FROM agent_grants", [])?;
        tx.conn().execute("DELETE FROM agent_slash_commands", [])?;
        tx.conn().execute("DELETE FROM agent_events", [])?;
        Room::find(tx.conn(), id("watercooler"))?.grant_to(tx, &[id("bender")])?;
        Ok(())
    });
    t
}
fn base() -> NewAgentSlashCommand {
    NewAgentSlashCommand {
        agent_id: id("bender_agent"),
        room_id: id("watercooler"),
        name: "deploy".into(),
        ..Default::default()
    }
}
fn context() -> Context {
    Context {
        user_id: id("david"),
        room_id: id("watercooler"),
        thread_id: None,
        huddles_configured: false,
    }
}
fn gold() -> Value {
    serde_json::from_str(include_str!(
        "../../../../vectors/agents_slash_command_contract.json"
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
fn result_payload(result: slash::CommandResult) -> Value {
    json!({"kind":result.kind,"message":result.message,"url":result.url,"notice":result.notice,"payload":result.payload()})
}

#[test]
fn ws11_slash_command_model_normalization_validation_and_builtin_collision_match_rails() {
    let t = frozen();
    t.write(|tx| {
        let v = gold();
        for (key, a) in [
            (
                "blank",
                NewAgentSlashCommand {
                    name: "".into(),
                    ..base()
                },
            ),
            (
                "invalid",
                NewAgentSlashCommand {
                    name: "Deploy!".into(),
                    ..base()
                },
            ),
            (
                "slash",
                NewAgentSlashCommand {
                    name: "/deploy".into(),
                    ..base()
                },
            ),
            (
                "spaces",
                NewAgentSlashCommand {
                    name: "two words".into(),
                    ..base()
                },
            ),
            (
                "long",
                NewAgentSlashCommand {
                    name: "x".repeat(33),
                    ..base()
                },
            ),
            (
                "builtin",
                NewAgentSlashCommand {
                    name: "poll".into(),
                    ..base()
                },
            ),
            (
                "description",
                NewAgentSlashCommand {
                    description: Some("é".repeat(141)),
                    ..base()
                },
            ),
            (
                "boundary",
                NewAgentSlashCommand {
                    description: Some("é".repeat(140)),
                    ..base()
                },
            ),
            (
                "missing_agent",
                NewAgentSlashCommand {
                    agent_id: 0,
                    ..base()
                },
            ),
            (
                "missing_room",
                NewAgentSlashCommand {
                    room_id: 0,
                    ..base()
                },
            ),
        ] {
            assert_eq!(
                errors(AgentSlashCommand::validate(tx.conn(), &a, None)?),
                v["validation"][key],
                "{key}"
            );
        }
        let created = AgentSlashCommand::create(
            tx,
            NewAgentSlashCommand {
                name: " Deploy ".into(),
                description: Some(" Ship it ".into()),
                ..base()
            },
        )?;
        assert_eq!(created.name, "deploy");
        assert_eq!(created.description.as_deref(), Some("Ship it"));
        assert!(created.takes_arguments);
        assert_eq!(
            errors(AgentSlashCommand::validate(tx.conn(), &base(), None)?),
            v["validation"]["duplicate"]
        );
        Ok(())
    });
}

#[test]
fn ws11_slash_command_registration_replay_flag_and_foreign_denials_match_rails() {
    let t = frozen();
    t.write(|tx| {
        let v = gold();
        let agent = id("bender_agent");
        let room = id("watercooler");
        let check = |key: &str, result: crate::models::agent_service::ServiceResult| {
            assert_eq!(
                json!({"status":result.status,"payload":result.payload,"error":result.error}),
                v["registrations"][key],
                "{key}"
            );
        };
        check(
            "missing",
            commands::register(tx, agent, 0, "inspect", None, None)?,
        );
        check(
            "created",
            commands::register(
                tx,
                agent,
                room,
                " Inspect ",
                Some(" Inspect <>& "),
                Some(false),
            )?,
        );
        check(
            "updated",
            commands::register(tx, agent, room, "INSPECT", Some(" Again "), None)?,
        );
        check(
            "default",
            commands::register(tx, agent, room, "ping", None, None)?,
        );
        let other = Agent::create(
            tx,
            NewAgent {
                user_id: id("jason"),
                owner_id: Some(id("david")),
                kind: AgentKind::Workspace,
                ..Default::default()
            },
        )?;
        Room::find(tx.conn(), room)?.grant_to(tx, &[other.user_id])?;
        check(
            "foreign",
            commands::register(tx, other.id, room, "inspect", None, None)?,
        );
        check(
            "builtin",
            commands::register(tx, agent, room, "poll", None, None)?,
        );
        check(
            "unregister_foreign",
            commands::unregister(tx, other.id, room, "inspect")?,
        );
        check(
            "unregister",
            commands::unregister(tx, agent, room, " PING ")?,
        );
        check(
            "unregister_missing",
            commands::unregister(tx, agent, room, "ping")?,
        );
        let mut grant = AgentGrant::create(
            tx,
            NewGrant {
                agent_id: agent,
                granted_by_id: id("david"),
                capability: "post_messages".into(),
                room_id: Some(room),
                ..Default::default()
            },
        )?;
        grant.revoke(tx)?;
        check(
            "revoked_registration",
            commands::register(tx, agent, room, "inspect", None, None)?,
        );
        Ok(())
    });
}

#[test]
fn ws11_slash_dispatch_payload_callbacks_revocation_and_twenty_event_budget_match_rails() {
    let t = frozen();
    t.write(|tx| {
        let v=gold();let agent=id("bender_agent");let mut ctx=context();
        AgentSlashCommand::create(tx,NewAgentSlashCommand {name:"inspect".into(),..base()})?;
        assert_eq!(result_payload(slash::dispatch(tx,&ctx,"/INSPECT   hello <>&")?),v["invocations"]["root"]);
        tx.conn().execute("INSERT INTO channel_threads(id,room_id,creator_id,name,last_activity_at,created_at,updated_at) VALUES (900040001,?,?,'Slash',?,?,?)",params![ctx.room_id,ctx.user_id,tx.now(),tx.now(),tx.now()])?;
        ctx.thread_id=Some(900040001);
        assert_eq!(result_payload(slash::dispatch(tx,&ctx,"/inspect x\n y")?),v["invocations"]["thread"]);
        let events=crate::sql::query_all(tx.conn(),"SELECT * FROM agent_events WHERE event_type='slash_command' ORDER BY id",[],crate::models::agent_delivery::AgentEvent::from_row)?;
        assert_eq!(json!(events.iter().map(|event|json!({"event_type":event.event_type,"room_id":event.room_id,"actor_id":event.actor_id,"outcome":event.outcome,"webhook_status":event.webhook_status,"chain_uuid":event.chain_id.as_deref().is_some_and(|s|uuid::Uuid::parse_str(s).is_ok()),"metadata":event.metadata})).collect::<Vec<_>>()),v["events"]);
        tx.conn().execute("DELETE FROM agent_events",[])?;
        ctx.thread_id=None;
        for _ in 0..20 {assert_eq!(slash::dispatch(tx,&ctx,"/inspect flood")?.kind,"ephemeral");}
        assert_eq!(result_payload(slash::dispatch(tx,&ctx,"/inspect overflow")?),v["invocations"]["rate"]);
        let count:i64=tx.conn().query_row("SELECT COUNT(*) FROM agent_events WHERE event_type='slash_command'",[],|r|r.get(0))?;assert_eq!(json!(count),v["invocations"]["rate_count"]);
        let mut grant=AgentGrant::create(tx,NewGrant {agent_id:agent,granted_by_id:id("david"),capability:"post_messages".into(),room_id:Some(ctx.room_id),..Default::default()})?;
        grant.revoke(tx)?;
        assert_eq!(result_payload(slash::dispatch(tx,&ctx,"/inspect nope")?),v["invocations"]["revoked"]);
        Ok(())
    });
    let jobs = t
        .events()
        .iter()
        .filter(|e| {
            e.as_job::<crate::models::agent_delivery::EventWebhookJob>()
                .is_some()
        })
        .count();
    assert_eq!(jobs, 22);
}

#[test]
fn ws11_slash_invocation_limit_is_atomic_across_competing_humans() {
    let t = frozen();
    t.write(|tx| {
        AgentSlashCommand::create(
            tx,
            NewAgentSlashCommand {
                name: "inspect".into(),
                ..base()
            },
        )
    });
    let mut workers = vec![];
    for _ in 0..4 {
        let db = t.db.clone();
        workers.push(std::thread::spawn(move || {
            (0..7)
                .map(|_| {
                    db.write_blocking(|tx| slash::dispatch(tx, &context(), "/inspect race"))
                        .unwrap()
                        .kind
                })
                .collect::<Vec<_>>()
        }));
    }
    let kinds: Vec<_> = workers
        .into_iter()
        .flat_map(|worker| worker.join().unwrap())
        .collect();
    assert_eq!(
        kinds.iter().filter(|k| k.as_str() == "ephemeral").count(),
        20
    );
    assert_eq!(kinds.iter().filter(|k| k.as_str() == "error").count(), 8);
}

#[test]
fn ws11_slash_removed_membership_and_soft_deletion_fail_before_inserting_events() {
    let t = frozen();
    t.write(|tx| {
        let ctx = context();
        let agent = id("bender_agent");
        commands::register(tx, agent, ctx.room_id, "inspect", None, None)?;
        let member =
            crate::Membership::find_by_room_and_user(tx.conn(), ctx.room_id, id("bender"))?
                .unwrap();
        member.destroy(tx)?;
        assert_eq!(
            commands::register(tx, agent, ctx.room_id, "inspect", None, None)?.status,
            404
        );
        assert_eq!(slash::dispatch(tx, &ctx, "/inspect nope")?.kind, "error");
        Room::find(tx.conn(), ctx.room_id)?.grant_to(tx, &[id("bender")])?;
        tx.conn().execute(
            "UPDATE rooms SET deleted_at=? WHERE id=?",
            params![tx.now(), ctx.room_id],
        )?;
        assert_eq!(
            commands::unregister(tx, agent, ctx.room_id, "inspect")?.status,
            404
        );
        assert_eq!(slash::dispatch(tx, &ctx, "/inspect nope")?.kind, "error");
        assert!(!crate::sql::exists(
            tx.conn(),
            "SELECT 1 FROM agent_events WHERE event_type='slash_command'",
            []
        )?);
        Ok(())
    });
}

#[test]
fn ws11_slash_poll_only_invocation_and_rollback_emit_no_webhook() {
    let t = frozen();
    t.write(|tx| {
        tx.conn()
            .execute("DELETE FROM webhooks WHERE user_id=?", [id("bender")])?;
        commands::register(
            tx,
            id("bender_agent"),
            id("watercooler"),
            "inspect",
            None,
            None,
        )?;
        Ok(())
    });
    assert!(
        t.try_write(|tx| {
            slash::dispatch(tx, &context(), "/inspect rollback")?;
            Err::<(), _>(crate::Error::Other("WS11 rollback".into()))
        })
        .is_err()
    );
    t.write(|tx| {
        assert!(!crate::sql::exists(
            tx.conn(),
            "SELECT 1 FROM agent_events",
            []
        )?);
        assert_eq!(
            slash::dispatch(tx, &context(), "/inspect poll")?.kind,
            "ephemeral"
        );
        let status: String = tx.conn().query_row(
            "SELECT webhook_status FROM agent_events WHERE event_type='slash_command'",
            [],
            |r| r.get(0),
        )?;
        assert_eq!(status, "none");
        Ok(())
    });
    assert!(!t.events().iter().any(|e| {
        e.as_job::<crate::models::agent_delivery::EventWebhookJob>()
            .is_some()
    }));
}
