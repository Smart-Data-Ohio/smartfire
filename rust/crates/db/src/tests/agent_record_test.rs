use super::*;
use crate::models::agent::{AgentStatusChange, AgentStatusTarget};
use crate::{Agent, AgentChanges, AgentGrant, AgentKind, NewAgent, NewGrant, Timestamp, User};
use rusqlite::params;
use serde_json::{Value, json};

fn gold() -> Value {
    serde_json::from_str(include_str!(
        "../../../../vectors/agents_agent_record_contract.json"
    ))
    .unwrap()
}
fn base() -> NewAgent {
    NewAgent {
        user_id: id("jason"),
        owner_id: Some(id("david")),
        kind: AgentKind::Workspace,
        ..Default::default()
    }
}
fn frozen() -> TestDb {
    let t = super::channel_thread_test::frozen();
    t.clock
        .travel_to(Timestamp::parse_db("2026-03-02 16:00:00").unwrap());
    t
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

#[test]
fn ws11_agent_record_validation_matches_rails_and_workspace_backfill_stays_valid() {
    let t = frozen();
    t.write(|tx| {
        let v = gold();
        assert_eq!(
            json!({"kind":NewAgent::default().kind.name(),"status":NewAgent::default().status}),
            v["reads"]["defaults"]
        );
        for (key, a) in [
            (
                "missing_user",
                NewAgent {
                    user_id: 0,
                    ..base()
                },
            ),
            (
                "personal_owner",
                NewAgent {
                    owner_id: None,
                    kind: AgentKind::Personal,
                    ..base()
                },
            ),
            (
                "workspace_owner",
                NewAgent {
                    owner_id: None,
                    ..base()
                },
            ),
            (
                "duplicate_user",
                NewAgent {
                    user_id: id("bender"),
                    ..base()
                },
            ),
            (
                "bad_status",
                NewAgent {
                    status: "napping".into(),
                    ..base()
                },
            ),
            (
                "long_note",
                NewAgent {
                    status_note: Some("é".repeat(201)),
                    ..base()
                },
            ),
            (
                "long_description",
                NewAgent {
                    description: Some("é".repeat(501)),
                    ..base()
                },
            ),
            (
                "long_presence",
                NewAgent {
                    working_presence: Some("é".repeat(141)),
                    ..base()
                },
            ),
            (
                "presence_boundary",
                NewAgent {
                    working_presence: Some("é".repeat(140)),
                    ..base()
                },
            ),
            (
                "zero_messages",
                NewAgent {
                    daily_message_cap: Some(0),
                    ..base()
                },
            ),
            (
                "negative_board",
                NewAgent {
                    daily_board_post_cap: Some(-1),
                    ..base()
                },
            ),
            (
                "zero_actions",
                NewAgent {
                    daily_external_action_cap: Some(0),
                    ..base()
                },
            ),
            (
                "valid",
                NewAgent {
                    daily_message_cap: Some(1),
                    ..base()
                },
            ),
        ] {
            assert_eq!(
                errors(Agent::validate(tx.conn(), &a, None)?),
                v["validation"][key],
                "{key}"
            );
        }
        let mut agent = Agent::find(tx.conn(), id("bender_agent"))?.unwrap();
        agent.update(
            tx,
            AgentChanges {
                owner_id: Some(None),
                description: Some(Some("Backfill".into())),
                ..Default::default()
            },
        )?;
        assert_eq!(
            json!(agent.owner_id.is_none()),
            v["reads"]["ownerless_valid"]
        );
        assert!(
            agent
                .update(
                    tx,
                    AgentChanges {
                        kind: Some(AgentKind::Personal),
                        ..Default::default()
                    }
                )
                .is_err()
        );
        assert_eq!(
            Agent::find(tx.conn(), agent.id)?.unwrap().kind,
            AgentKind::Workspace
        );
        let created = Agent::create(tx, base())?;
        assert!(created.status_changed_at.is_none());
        Ok(())
    });
}

#[test]
fn ws11_agent_descriptions_summaries_and_suspension_revoke_live_grants() {
    use crate::models::agent_delivery::{AgentEvent, NewEvent};
    let t = frozen();
    t.write(|tx| {
        let v = gold();
        let v = &v["reads"];
        tx.conn().execute("DELETE FROM agent_grants", [])?;
        let mut agent = Agent::find(tx.conn(), id("bender_agent"))?.unwrap();
        assert_eq!(json!(agent.grants_summary(tx.conn())?), v["legacy"]);
        for room in ["watercooler", "designers", "hq"] {
            AgentGrant::create(
                tx,
                NewGrant {
                    agent_id: agent.id,
                    granted_by_id: id("david"),
                    capability: "post_messages".into(),
                    room_id: Some(id(room)),
                    ..Default::default()
                },
            )?;
        }
        AgentGrant::create(
            tx,
            NewGrant {
                agent_id: agent.id,
                granted_by_id: id("david"),
                capability: "read_messages".into(),
                ..Default::default()
            },
        )?;
        assert_eq!(json!(agent.grants_summary(tx.conn())?), v["grants"]);
        AgentGrant::revoke_for_agent(tx, agent.id)?;
        assert_eq!(json!(agent.grants_summary(tx.conn())?), v["revoked"]);
        for (kind, key) in [
            (AgentKind::Personal, "personal"),
            (AgentKind::Workspace, "workspace"),
        ] {
            agent.update(
                tx,
                AgentChanges {
                    kind: Some(kind),
                    owner_id: Some(Some(id("kevin"))),
                    ..Default::default()
                },
            )?;
            assert_eq!(json!(agent.kind_description(tx.conn())?), v[key]);
        }
        agent.update(
            tx,
            AgentChanges {
                owner_id: Some(None),
                ..Default::default()
            },
        )?;
        assert_eq!(json!(agent.kind_description(tx.conn())?), v["ownerless"]);
        tx.conn().execute("DELETE FROM agent_events", [])?;
        for (kind, outcome, old) in [
            ("mention", "delivered", false),
            ("reply", "acknowledged", false),
            ("posted", "delivered", false),
            ("delivery_suppressed_rate_limit", "suppressed", false),
            ("mention", "delivered", true),
        ] {
            let event = AgentEvent::create(
                tx,
                NewEvent {
                    agent_id: agent.id,
                    event_type: kind.into(),
                    outcome: Some(outcome.into()),
                    ..Default::default()
                },
            )?;
            if old {
                tx.conn().execute(
                    "UPDATE agent_events SET created_at=? WHERE id=?",
                    params![tx.now().ago(jiff::SignedDuration::from_hours(25)), event.id],
                )?;
            }
        }
        assert_eq!(
            json!(agent.activity_summary(tx.conn(), tx.now())?),
            v["activity"]
        );
        AgentGrant::create(
            tx,
            NewGrant {
                agent_id: agent.id,
                granted_by_id: id("david"),
                capability: "read_messages".into(),
                ..Default::default()
            },
        )?;
        let stale = agent.clone();
        agent.update(
            tx,
            AgentChanges {
                suspended_at: Some(Some(tx.now())),
                ..Default::default()
            },
        )?;
        assert_eq!(json!(agent.active(tx.conn())?), v["active"]);
        assert_eq!(json!(agent.suspended()), v["suspended"]);
        assert!(!stale.active(tx.conn())?);
        assert!(!crate::sql::exists(
            tx.conn(),
            "SELECT 1 FROM agent_grants WHERE agent_id=? AND revoked_at IS NULL",
            [agent.id]
        )?);
        agent.update(
            tx,
            AgentChanges {
                suspended_at: Some(None),
                ..Default::default()
            },
        )?;
        assert!(!agent.can(tx.conn(), "read_messages", None)?);
        Ok(())
    });
}

#[test]
fn ws11_agent_status_stamp_and_broadcast_requests_match_rails_targets() {
    let t = frozen();
    let mut agent = t.read(|c| Agent::find(c, id("bender_agent"))).unwrap();
    for (key, changes) in [
        (
            "note",
            AgentChanges {
                status_note: Some(Some("Ready".into())),
                ..Default::default()
            },
        ),
        (
            "provider",
            AgentChanges {
                provider: Some(Some("Acme".into())),
                ..Default::default()
            },
        ),
        (
            "working",
            AgentChanges {
                status: Some("working".into()),
                status_note: Some(Some("Now".into())),
                ..Default::default()
            },
        ),
        (
            "no_change",
            AgentChanges {
                status: Some("working".into()),
                ..Default::default()
            },
        ),
        ("seen", AgentChanges::default()),
    ] {
        let from = t.events().len();
        agent = t.write(move |tx| {
            if key == "seen" {
                agent.touch_last_seen(tx)?;
            } else {
                agent.update(tx, changes)?;
            }
            Ok(agent)
        });
        let frames:Vec<_>=t.events()[from..].iter().filter_map(|event|match event {
            Event::Broadcast(request)=>request.decode::<AgentStatusChange>().map(|decoded| {
                let decoded=decoded.unwrap();
                json!({"stream":"agents:all","target":format!("{}_agent_{}",match decoded.target {AgentStatusTarget::Badge=>"status_badge",AgentStatusTarget::DirectoryRow=>"directory_row"},decoded.agent_id)})
            }),_=>None,
        }).collect();
        assert_eq!(
            json!({"changed_at":agent.status_changed_at.map(crate::models::agent_payloads::json_time),"broadcasts":frames}),
            gold()["status"][key]
        );
    }
}

#[test]
fn ws11_agent_working_presence_strip_ttl_and_clear_match_rails() {
    let t = frozen();
    let mut agent = t.read(|c| Agent::find(c, id("bender_agent"))).unwrap();
    for (key, text) in [
        ("set", " Thinking… "),
        ("clear", " \t\0"),
        ("unicode", "\u{a0}Thinking\u{a0}"),
    ] {
        agent = t.write(move |tx| {
            agent.set_working_presence(tx, Some(text))?;
            Ok(agent)
        });
        assert_eq!(
            json!({"stored":agent.working_presence,"expires_at":agent.working_presence_expires_at.map(crate::models::agent_payloads::json_time),"text":agent.working_presence_text(t.now())}),
            gold()["presence"][key]
        );
    }
    let deadline = agent.working_presence_expires_at.unwrap();
    assert_eq!(
        json!(agent.working_presence_text(deadline)),
        gold()["presence"]["boundary"]
    );
    assert_eq!(
        json!(
            agent.working_presence_text(Timestamp::from_microsecond(deadline.as_microsecond() - 1))
        ),
        gold()["presence"]["before"]
    );
    assert!(
        t.read(|c| Agent::find(c, agent.id))
            .unwrap()
            .working_presence
            .is_some()
    );
    let id = agent.id;
    assert!(
        t.try_write(move |tx| agent.set_working_presence(tx, Some(&"é".repeat(141))))
            .is_err()
    );
    let mut agent = t.read(|c| Agent::find(c, id)).unwrap();
    agent = t.write(move |tx| {
        agent.clear_working_presence(tx)?;
        Ok(agent)
    });
    assert!(agent.working_presence.is_none());
    assert!(agent.working_presence_expires_at.is_none());
}

#[test]
fn ws11_agent_directory_order_and_last_seen_use_live_rows() {
    let t = frozen();
    t.write(|tx| {
        for (name, status, suspended) in [
            ("Ágent", 0, false),
            ("Zed Suspended", 0, true),
            ("Aaron Gone", 1, false),
            ("Banned", 2, false),
        ] {
            let user = User::create_bot(tx, name, None)?;
            let mut agent = Agent::create(
                tx,
                NewAgent {
                    user_id: user.id,
                    owner_id: Some(id("david")),
                    kind: AgentKind::Workspace,
                    ..Default::default()
                },
            )?;
            if suspended {
                agent.update(
                    tx,
                    AgentChanges {
                        suspended_at: Some(Some(tx.now())),
                        ..Default::default()
                    },
                )?;
            }
            tx.conn().execute(
                "UPDATE users SET status=? WHERE id=?",
                params![status, user.id],
            )?;
        }
        let names = Agent::for_directory(tx.conn())?
            .iter()
            .map(|a| User::find(tx.conn(), a.user_id).map(|u| u.name))
            .collect::<crate::Result<Vec<_>>>()?;
        assert_eq!(json!(names), gold()["reads"]["directory"]);
        let mut agent = Agent::find(tx.conn(), id("bender_agent"))?.unwrap();
        let stale = agent.clone();
        agent.touch_last_seen(tx)?;
        let initial = agent.last_seen_at;
        let mut stale = stale;
        stale.touch_last_seen(tx)?;
        assert_eq!(
            Agent::find(tx.conn(), agent.id)?.unwrap().last_seen_at,
            initial
        );
        tx.conn().execute(
            "UPDATE agents SET last_seen_at=? WHERE id=?",
            params![tx.now().ago(jiff::SignedDuration::from_mins(1)), agent.id],
        )?;
        stale.touch_last_seen(tx)?;
        assert_eq!(stale.last_seen_at, Some(tx.now()));
        Ok(())
    });
}

#[test]
fn ws11_agent_suspension_and_status_broadcasts_roll_back_together() {
    let t = frozen();
    let mut agent = t.read(|c| Agent::find(c, id("bender_agent"))).unwrap();
    let id = agent.id;
    t.write(move |tx| {
        AgentGrant::create(
            tx,
            NewGrant {
                agent_id: id,
                granted_by_id: super::id("david"),
                capability: "read_messages".into(),
                ..Default::default()
            },
        )
    });
    let before = t.events().len();
    assert!(
        t.try_write(move |tx| {
            agent.update(
                tx,
                AgentChanges {
                    status: Some("working".into()),
                    suspended_at: Some(Some(tx.now())),
                    ..Default::default()
                },
            )?;
            Err::<(), _>(crate::Error::Other("WS11 rollback".into()))
        })
        .is_err()
    );
    assert_eq!(t.events().len(), before);
    t.read(|c| {
        let agent = Agent::find(c, id)?.unwrap();
        assert!(agent.active(c)?);
        assert_eq!(agent.status, "idle");
        assert!(agent.can(c, "read_messages", None)?);
        Ok(())
    });
}
