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

fn typed_schema() -> Value {
    json!({"subcommands":[{"name":"run","description":"Run a deploy","options":[
        {"name":"target","description":"Deploy target","type":"string","required":true,"choices":[{"name":"Staging","value":"staging"}]},
        {"name":"count","description":"Retry count","type":"integer","required":true,"min":1,"max":3},
        {"name":"user","description":"Owner","type":"user","required":false},
        {"name":"room","description":"Destination","type":"room","required":false}
    ]}]})
}
#[test]
fn slash_malformed_schema_reads_and_listing_isolate_commands() {
    let t = frozen();
    t.write(|tx| {
        let agent = id("bender_agent");
        let room = id("watercooler");
        commands::register(tx, agent, room, "broken", None, None)?;
        commands::register_with_schema(tx, agent, room, "typed", None, None, Some(typed_schema()))?;
        tx.conn().execute(
            "UPDATE agent_slash_commands SET option_schema='{' WHERE name='broken'",
            [],
        )?;
        let command = AgentSlashCommand::find_by_name(tx.conn(), room, "broken")?.unwrap();
        assert_eq!(
            AgentSlashCommand::find(tx.conn(), command.id)?
                .unwrap()
                .name,
            "broken"
        );
        assert!(command.payload().get("option_schema").is_none());
        let suggestions = crate::command_suggestions::for_room(tx.conn(), room, false, None)?;
        let broken = suggestions.iter().find(|c| c.name == "broken").unwrap();
        assert_eq!(broken.description, "Custom command");
        assert!(
            serde_json::to_value(broken)
                .unwrap()
                .get("option_schema")
                .is_none()
        );
        let typed = suggestions.iter().find(|c| c.name == "typed").unwrap();
        assert_eq!(json!(typed.option_schema), typed_schema());
        Ok(())
    });
}

#[test]
fn slash_malformed_schema_invocation_returns_validation_without_delivery() {
    let t = frozen();
    t.write(|tx| {
        let ctx = context();
        commands::register(tx, id("bender_agent"), ctx.room_id, "broken", None, None)?;
        tx.conn().execute(
            "UPDATE agent_slash_commands SET option_schema='{' WHERE name='broken'",
            [],
        )?;
        match slash::dispatch(tx, &ctx, "/broken") {
            Err(crate::Error::FieldValidation(fields)) => {
                assert_eq!(
                    json!(fields),
                    json!({"options":["command option schema is unavailable"]})
                );
            }
            result => panic!("expected schema validation error, got {result:?}"),
        }
        let count: i64 = tx.conn().query_row(
            "SELECT COUNT(*) FROM agent_events WHERE event_type='slash_command'",
            [],
            |r| r.get(0),
        )?;
        assert_eq!(count, 0);
        commands::register(tx, id("bender_agent"), ctx.room_id, "valid", None, None)?;
        assert_eq!(slash::dispatch(tx, &ctx, "/valid")?.kind, "ephemeral");
        Ok(())
    });
}

#[test]
fn slash_malformed_schema_registration_can_keep_clear_and_replace() {
    let t = frozen();
    t.write(|tx| {
        let agent = id("bender_agent");
        let room = id("watercooler");
        commands::register(tx, agent, room, "broken", None, None)?;
        for schema in [Value::Null, typed_schema()] {
            tx.conn().execute(
                "UPDATE agent_slash_commands SET option_schema='{' WHERE name='broken'",
                [],
            )?;
            assert_eq!(
                commands::register(tx, agent, room, "broken", Some("Updated"), None)?.status,
                201
            );
            let stored: String = tx.conn().query_row(
                "SELECT option_schema FROM agent_slash_commands WHERE name='broken'",
                [],
                |r| r.get(0),
            )?;
            assert_eq!(stored, "{");
            let result = commands::register_with_schema(
                tx,
                agent,
                room,
                "broken",
                None,
                None,
                Some(schema.clone()),
            )?;
            assert_eq!(result.status, 201);
            let stored: Option<String> = tx.conn().query_row(
                "SELECT option_schema FROM agent_slash_commands WHERE name='broken'",
                [],
                |r| r.get(0),
            )?;
            assert_eq!(
                stored
                    .map(|s| serde_json::from_str::<Value>(&s).unwrap())
                    .unwrap_or(Value::Null),
                schema
            );
        }
        Ok(())
    });
}

#[test]
fn slash_typed_schema_management_keeps_canonical_json_and_authorization() {
    let t = frozen();
    t.write(|tx| {
        let agent = id("bender_agent");
        let room = id("watercooler");
        let schema = typed_schema();
        assert_eq!(
            commands::register_with_schema(
                tx,
                agent,
                room,
                "typed",
                None,
                None,
                Some(schema.clone())
            )?
            .status,
            201
        );
        let command = AgentSlashCommand::find_by_name(tx.conn(), room, "typed")?.unwrap();
        let stored: String = tx.conn().query_row(
            "SELECT option_schema FROM agent_slash_commands WHERE id=?",
            [command.id],
            |r| r.get(0),
        )?;
        assert_eq!(
            stored,
            serde_json::to_string(&command.option_schema()?).unwrap()
        );
        assert_eq!(
            commands::register(tx, agent, room, "typed", Some("Updated"), None)?
                .payload
                .unwrap()["option_schema"],
            schema
        );
        let suggestion = crate::command_suggestions::for_room(tx.conn(), room, false, None)?
            .into_iter()
            .find(|c| c.name == "typed")
            .unwrap();
        assert_eq!(json!(suggestion.option_schema), schema);
        let mut bad = schema.clone();
        bad["subcommands"][0]["options"][0]["name"] = json!("Bad");
        let invalid =
            commands::register_with_schema(tx, agent, room, "typed", None, None, Some(bad))?;
        assert_eq!(invalid.status, 422);
        assert_eq!(
            invalid.failure_body()["errors"]["option_schema.subcommands[0].options[0].name"][0],
            "must be 1–32 lowercase letters, digits, underscores or hyphens"
        );
        assert_eq!(
            json!(
                AgentSlashCommand::find(tx.conn(), command.id)?
                    .unwrap()
                    .option_schema()?
            ),
            schema
        );
        assert_eq!(
            commands::register_with_schema(
                tx,
                agent,
                0,
                "typed",
                None,
                None,
                Some(schema.clone())
            )?
            .status,
            404
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
        assert_eq!(
            commands::register_with_schema(
                tx,
                other.id,
                room,
                "typed",
                None,
                None,
                Some(Value::Null)
            )?
            .status,
            422
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
        assert_eq!(
            commands::register_with_schema(
                tx,
                agent,
                room,
                "typed",
                None,
                None,
                Some(Value::Null)
            )?
            .status,
            403
        );
        tx.conn()
            .execute("DELETE FROM agent_grants WHERE agent_id=?", [agent])?;
        assert_eq!(
            commands::register_with_schema(
                tx,
                agent,
                room,
                "typed",
                None,
                None,
                Some(Value::Null)
            )?
            .status,
            201
        );
        assert!(
            AgentSlashCommand::find(tx.conn(), command.id)?
                .unwrap()
                .option_schema()?
                .is_none()
        );
        Ok(())
    });
}
#[test]
fn slash_typed_invocation_validates_before_events_and_reaches_both_delivery_payloads() {
    use crate::models::{
        agent_delivery::AgentEvent,
        agent_event_polling,
        agent_payloads::{self, Payload, RepositoryAccess},
    };
    let t = frozen();
    t.write(|tx| {
        let agent=id("bender_agent");let ctx=context();
        commands::register_with_schema(tx,agent,ctx.room_id,"typed",None,None,Some(typed_schema()))?;
        let good=json!({"subcommand":"run","values":{"target":"staging","count":2,"user":ctx.user_id,"room":ctx.room_id}});
        for (name,value) in [("target",json!(true)),("target",json!("production")),("count",json!(4)),("count",json!(2.5)),("unknown",json!("x"))] {
            let mut bad=good.clone();bad["values"][name]=value;
            let values=serde_json::from_value(bad).unwrap();
            assert!(matches!(slash::dispatch_with_options(tx,&ctx,"/typed raw text",Some(&values)),Err(crate::Error::FieldValidation(fields)) if fields.contains_key(&format!("options.values.{name}"))));
        }
        assert!(matches!(slash::dispatch_with_options(tx,&ctx,"/typed",None),Err(crate::Error::FieldValidation(fields)) if fields.contains_key("options.subcommand")));
        let missing=serde_json::from_value(json!({"subcommand":"run","values":{}})).unwrap();
        assert!(matches!(slash::dispatch_with_options(tx,&ctx,"/typed",Some(&missing)),Err(crate::Error::FieldValidation(fields)) if fields.contains_key("options.values.target")));
        tx.conn().execute("UPDATE users SET status=1 WHERE id=?",[id("kevin")])?;
        tx.conn().execute("DELETE FROM memberships WHERE room_id=? AND user_id=?",params![id("pets"),ctx.user_id])?;
        for (name,value) in [("user",0x7fffffff_i64),("user",id("kevin")),("room",id("pets")),("room",0x7fffffff_i64)] {
            let mut bad=good.clone();bad["values"][name]=json!(value);
            let values=serde_json::from_value(bad).unwrap();
            assert!(matches!(slash::dispatch_with_options(tx,&ctx,"/typed",Some(&values)),Err(crate::Error::RecordNotFound(_))),"{name} {value}");
        }
        assert!(!crate::sql::exists(tx.conn(),"SELECT 1 FROM agent_events WHERE event_type='slash_command'",[])?);
        let values=serde_json::from_value(good.clone()).unwrap();
        assert_eq!(slash::dispatch_with_options(tx,&ctx,"/typed raw text",Some(&values))?.kind,"ephemeral");
        let event=AgentEvent::for_agent(tx.conn(),agent)?.into_iter().find(|e|e.event_type=="slash_command").unwrap();
        assert_eq!(event.metadata["arguments"],"raw text");assert_eq!(event.metadata["options"],good);
        let access=RepositoryAccess::default();
        let Payload::Ready{body,..}=agent_payloads::build(tx.conn(),tx.rich_text(),&event,&access)? else {panic!("deliverable slash");};
        let payload:Value=serde_json::from_str(&body).unwrap();
        assert_eq!(payload["command"]["arguments"],"raw text");assert_eq!(payload["command"]["options"],good);
        let poll=agent_event_polling::poll(tx.conn(),agent,None,None,tx.now(),&access,|_|panic!("slash has no message"))?;
        assert_eq!(poll["events"][0]["command"]["options"],good);
        assert_eq!(poll["events"][0]["command"]["arguments"],"raw text");
        // NULL-schema commands retain their exact legacy event, even with extra typed input.
        commands::register(tx,agent,ctx.room_id,"legacy",None,None)?;
        assert_eq!(slash::dispatch_with_options(tx,&ctx,"/legacy untouched",Some(&values))?.kind,"ephemeral");
        let events=AgentEvent::for_agent(tx.conn(),agent)?;
        assert_eq!(events.last().unwrap().metadata,json!({"command":"legacy","arguments":"untouched"}));
        Ok(())
    });
}
