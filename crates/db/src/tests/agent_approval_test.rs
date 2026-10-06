use super::*;
use crate::models::agent_approval::ApprovalDecision;
use crate::{AgentApproval, NewApproval, Timestamp, User};
use rusqlite::params;
use serde_json::{Value, json};

fn base() -> NewApproval {
    NewApproval {
        agent_id: id("bender_agent"),
        action: "deploy".into(),
        summary: "Ship".into(),
        ..Default::default()
    }
}
fn setup() -> TestDb {
    let t = super::channel_thread_test::frozen();
    t.clock
        .travel_to(Timestamp::parse_db("2026-03-02 16:00:00").unwrap());
    t.write(|tx| {
        tx.conn().execute(
            "UPDATE agents SET owner_id=? WHERE id=?",
            params![id("kevin"), id("bender_agent")],
        )?;
        tx.conn().execute(
            "DELETE FROM activity_items WHERE source_type='AgentApproval'",
            [],
        )?;
        tx.conn().execute("DELETE FROM agent_approvals", [])?;
        tx.conn().execute("DELETE FROM agent_events", [])?;
        tx.conn().execute("DELETE FROM agent_grants", [])?;
        for (name, seq) in [("agent_approvals", 900020000), ("agent_events", 27)] {
            tx.conn()
                .execute("DELETE FROM sqlite_sequence WHERE name=?", [name])?;
            tx.conn().execute(
                "INSERT INTO sqlite_sequence(name,seq) VALUES (?,?)",
                params![name, seq],
            )?;
        }
        Ok(())
    });
    t
}
fn gold() -> Value {
    serde_json::from_str(include_str!(
        "../../../../vectors/agents_approval_contract.json"
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
fn workflow_errors(e: crate::Errors) -> Value {
    if e.is_empty() { json!([]) } else { errors(e) }
}
fn inbox(conn: &Connection, id: i64) -> crate::Result<Value> {
    Ok(json!(crate::sql::query_all(
        conn,
        "SELECT user_id,read_at,handled_at FROM activity_items WHERE source_type='AgentApproval' AND source_id=? ORDER BY user_id",
        [id],
        |r| {
            let read = r.get::<_, Option<Timestamp>>(1)?;
            let handled = r.get::<_, Option<Timestamp>>(2)?;
            Ok(
                json!({"user_id":r.get::<_,i64>(0)?,"unread":read.is_none()&&handled.is_none(),"handled":handled.is_some(),"read_at":read.map(crate::models::agent_payloads::json_time)}),
            )
        }
    )?))
}

#[test]
fn ws11_approval_validation_defaults_boundaries_and_uniqueness_match_rails() {
    let t = setup();
    t.write(|tx| {
        let now = tx.now();
        let vectors = gold();
        for (key, a) in [
            (
                "blank",
                NewApproval {
                    action: "".into(),
                    summary: "".into(),
                    ..base()
                },
            ),
            (
                "invalid_action",
                NewApproval {
                    action: "Deploy!".into(),
                    ..base()
                },
            ),
            (
                "action_long",
                NewApproval {
                    action: "a".repeat(61),
                    ..base()
                },
            ),
            (
                "summary_long",
                NewApproval {
                    summary: "é".repeat(501),
                    ..base()
                },
            ),
            (
                "valid_action",
                NewApproval {
                    action: "deploy.prod_1-ok".into(),
                    ..base()
                },
            ),
            (
                "invalid_status",
                NewApproval {
                    status: "bad".into(),
                    ..base()
                },
            ),
            (
                "blank_status",
                NewApproval {
                    status: "".into(),
                    ..base()
                },
            ),
            (
                "payload_large",
                NewApproval {
                    payload: Some("é".repeat(2049)),
                    ..base()
                },
            ),
            (
                "payload_boundary",
                NewApproval {
                    payload: Some("é".repeat(2048)),
                    ..base()
                },
            ),
            (
                "note_large",
                NewApproval {
                    decision_note: Some("é".repeat(201)),
                    ..base()
                },
            ),
            (
                "minimum",
                NewApproval {
                    expires_at: Some(now.since(jiff::SignedDuration::from_mins(4))),
                    ..base()
                },
            ),
            (
                "too_soon",
                NewApproval {
                    expires_at: Some(now.since(jiff::SignedDuration::from_secs(239))),
                    ..base()
                },
            ),
            (
                "maximum",
                NewApproval {
                    expires_at: Some(now.since(jiff::SignedDuration::from_secs(7 * 86400 + 60))),
                    ..base()
                },
            ),
            (
                "too_late",
                NewApproval {
                    expires_at: Some(now.since(jiff::SignedDuration::from_secs(7 * 86400 + 61))),
                    ..base()
                },
            ),
            (
                "missing_agent",
                NewApproval {
                    agent_id: 0,
                    ..base()
                },
            ),
            (
                "optional_missing",
                NewApproval {
                    room_id: Some(-1),
                    agent_credential_id: Some(-1),
                    decided_by_id: Some(-1),
                    ..base()
                },
            ),
        ] {
            assert_eq!(
                errors(AgentApproval::validate(tx.conn(), &a, now, None)?),
                vectors["validation"][key],
                "{key}"
            );
        }
        let created = AgentApproval::create(
            tx,
            NewApproval {
                external_id: Some("repeat".into()),
                ..base()
            },
        )?;
        assert_eq!(
            created.expires_at,
            now.since(jiff::SignedDuration::from_hours(24))
        );
        for (key, external) in [
            ("duplicate", Some("repeat")),
            ("blank_external", Some(" ")),
            ("nil_external", None),
        ] {
            assert_eq!(
                errors(AgentApproval::validate(
                    tx.conn(),
                    &NewApproval {
                        external_id: external.map(str::to_owned),
                        ..base()
                    },
                    now,
                    None
                )?),
                vectors["validation"][key]
            );
        }
        for _ in 0..2 {
            assert!(
                AgentApproval::create(
                    tx,
                    NewApproval {
                        external_id: Some(" ".into()),
                        ..base()
                    }
                )?
                .external_id
                .is_none()
            );
        }
        Ok(())
    });
}

#[test]
fn ws11_approval_decision_expiry_cancellation_and_inbox_match_rails() {
    let t = setup();
    t.write(|tx|AgentApproval::create(tx,NewApproval {external_id:Some("repeat".into()),..base()}));
    let mut approval=t.write(|tx|AgentApproval::create(tx,NewApproval {room_id:Some(id("watercooler")),payload:Some("{\"key\":\"<>&\"}".into()),external_id:Some("stable".into()),..base()}));
    t.write(move |tx| {
        let flow=gold()["flows"].clone();
        let owner=User::find(tx.conn(),id("kevin"))?;
        let admin=User::find(tx.conn(),id("david"))?;
        assert_eq!(approval.payload(tx.conn(),tx.now())?,flow["created"]);
        assert_eq!(inbox(tx.conn(),approval.id)?,flow["inbox"]);
        let mut stale=approval.clone();
        assert_eq!(workflow_errors(approval.decide(tx,"denied",&owner,Some("  "))?),flow["denied_errors"]);
        assert_eq!(approval.payload(tx.conn(),tx.now())?,flow["denied"]);
        assert_eq!(inbox(tx.conn(),approval.id)?,flow["settled_inbox"]);
        assert_eq!(errors(stale.decide(tx,"approved",&admin,None)?),flow["stale_errors"]);
        let events=crate::sql::query_all(tx.conn(),"SELECT agent_approval_id,outcome,webhook_status,metadata FROM agent_events WHERE event_type='approval_decided' ORDER BY id",[],|r|Ok(json!({"agent_approval_id":r.get::<_,Option<i64>>(0)?,"outcome":r.get::<_,Option<String>>(1)?,"webhook_status":r.get::<_,String>(2)?,"metadata":r.get::<_,Value>(3)?})))?;
        assert_eq!(json!(events),flow["events"]);
        assert_eq!(approval.decide(tx,"surprise",&admin,None).unwrap_err().to_string(),flow["unknown"]);
        Ok(())
    });
    let mut expired=t.write(|tx|AgentApproval::create(tx,base()));
    t.write(move |tx| {
        let flow=gold()["flows"].clone();
        tx.conn().execute("UPDATE agent_approvals SET expires_at=? WHERE id=?",params![tx.now(),expired.id])?;
        expired=AgentApproval::find(tx.conn(),expired.id)?.unwrap();
        assert_eq!(json!({"status":expired.status,"effective":expired.effective_status(tx.now())}),flow["effective"]);
        assert_eq!(errors(expired.cancel_by_agent(tx)?),flow["expired_errors"]);
        assert_eq!(json!({"status":expired.status,"inbox":inbox(tx.conn(),expired.id)?}),flow["expired"]);
        assert!(!expired.expire_if_due(tx)?);Ok(())
    });
    let mut cancelled=t.write(|tx|AgentApproval::create(tx,base()));
    t.write(move |tx| {
        let flow=gold()["flows"].clone();
        assert_eq!(workflow_errors(cancelled.cancel_by_agent(tx)?),flow["cancel_errors"]);
        assert_eq!(errors(cancelled.cancel_by_agent(tx)?),flow["cancel_again"]);
        let count:i64=tx.conn().query_row("SELECT COUNT(*) FROM agent_events WHERE agent_approval_id=?",[cancelled.id],|r|r.get(0))?;
        assert_eq!(json!(count),flow["cancel_events"]);Ok(())
    });
    let mut note=t.write(|tx|AgentApproval::create(tx,base()));
    t.write(move |tx| {
        let flow=gold()["flows"].clone();
        let owner=User::find(tx.conn(),id("kevin"))?;
        let admin=User::find(tx.conn(),id("david"))?;
        assert_eq!(errors(note.decide(tx,"denied",&admin,Some(&"é".repeat(201)))?),flow["long_note"]);
        assert_eq!(json!(AgentApproval::find(tx.conn(),note.id)?.unwrap().status),flow["long_note_status"]);
        tx.conn().execute("UPDATE users SET inbox_preferences=? WHERE id=?",params![json!({"agent_approvals":"0"}),owner.id])?;Ok(())
    });
    let opted=t.write(|tx|AgentApproval::create(tx,base()));
    t.read(|c| {assert_eq!(inbox(c,opted.id)?,gold()["flows"]["opt_out"]);Ok(())});
    let mut external=t.write(|tx|AgentApproval::create(tx,NewApproval {action:"github.comment".into(),..base()}));
    t.write(move |tx| {
        let flow=gold()["flows"].clone();
        let owner=User::find(tx.conn(),id("kevin"))?;
        let admin=User::find(tx.conn(),id("david"))?;
        assert!(external.decide(tx,"approved",&admin,Some("ok"))?.is_empty());
        tx.conn().execute("UPDATE agents SET suspended_at=? WHERE id=?",params![tx.now(),external.agent_id])?;
        assert_eq!(json!(external.decidable_by(tx.conn(),&owner)?),flow["suspended_decidable"]);
        tx.conn().execute("UPDATE users SET status=2 WHERE id=?",[id("bender")])?;
        assert_eq!(json!(external.decidable_by(tx.conn(),&owner)?),flow["inactive_decidable"]);Ok(())
    });
    let jobs:Vec<_>=t.events().into_iter().filter_map(|e|match e {
        Event::Job(job) if ["Agent::EventWebhookJob","Github::PerformAgentActionJob"].contains(&job.class)=>Some(json!({"class":job.class,"args":if job.class=="Agent::EventWebhookJob" {json!([job.arguments["event_id"],job.arguments["attempt"]])} else {json!([job.arguments["approval_id"]])}})),_=>None
    }).collect();
    let v = gold();
    let mut expected = v["flows"]["jobs"].as_array().unwrap().clone();
    expected.extend(v["flows"]["external_jobs"].as_array().unwrap().clone());
    assert_eq!(jobs, expected);
}

#[test]
fn ws11_approval_authorization_matches_rails_and_external_approval_is_admin_only() {
    let t = setup();
    t.write(|tx| {
        let v=gold();
        for action in ["deploy","github.comment","fizzy.close"] {
            let approval=AgentApproval::create(tx,NewApproval {action:action.into(),..base()})?;
            for user in ["kevin","david","jason","bender","jz"] {
                let user=User::find(tx.conn(),id(user))?;
                assert_eq!(json!({"decidable":approval.decidable_by(tx.conn(),&user)?,"approvable":approval.approvable_by(tx.conn(),&user)?}),v["permissions"][action][&user.name]);
            }
        }
        let mut approval=AgentApproval::create(tx,NewApproval {action:"github.comment".into(),..base()})?;
        let owner=User::find(tx.conn(),id("kevin"))?;
        assert_eq!(approval.decide_authorized(tx,"approved",&owner,None)?,ApprovalDecision::Forbidden);
        assert_eq!(AgentApproval::find(tx.conn(),approval.id)?.unwrap().status,"pending");
        let mut denied=approval.clone();
        assert_eq!(denied.decide_authorized(tx,"denied",&owner,None)?,ApprovalDecision::Applied);
        let admin=User::find(tx.conn(),id("david"))?;
        tx.conn().execute("UPDATE users SET status=1 WHERE id=?",[admin.id])?;
        assert!(!approval.decidable_by(tx.conn(),&admin)?);
        Ok(())
    });
}

#[test]
fn ws11_approval_inbox_replay_preserves_read_time_and_overdue_filter() {
    let t = setup();
    let (first,second)=t.write(|tx| Ok((AgentApproval::create(tx,base())?, AgentApproval::create(tx,base())?)));
    let (first,second)=t.write(move |tx| {
        let read=tx.now().ago(jiff::SignedDuration::from_mins(5));
        tx.conn().execute("UPDATE activity_items SET read_at=? WHERE source_type='AgentApproval' AND source_id=? AND user_id=?",params![read,first.id,id("kevin")])?;
        first.fan_out_inbox_items(tx)?;
        let count:i64=tx.conn().query_row("SELECT COUNT(*) FROM activity_items WHERE source_type='AgentApproval' AND source_id=?",[first.id],|r|r.get(0))?;assert_eq!(count,3);
        tx.conn().execute("DELETE FROM activity_items WHERE source_type='AgentApproval' AND source_id=? AND user_id!=?",params![second.id,id("david")])?;
        tx.conn().execute("UPDATE agent_approvals SET expires_at=? WHERE id IN (?,?)",params![tx.now(),first.id,second.id])?;
        AgentApproval::resolve_overdue(tx,Some(id("kevin")))?;
        assert_eq!(AgentApproval::find(tx.conn(),first.id)?.unwrap().status,"expired");
        assert_eq!(AgentApproval::find(tx.conn(),second.id)?.unwrap().status,"pending");
        let actual:Timestamp=tx.conn().query_row("SELECT read_at FROM activity_items WHERE source_type='AgentApproval' AND source_id=? AND user_id=?",params![first.id,id("kevin")],|r|r.get(0))?;assert_eq!(actual,read);
        Ok((first.id,second.id))
    });
    t.write(move |tx| {
        AgentApproval::resolve_overdue(tx, None)?;
        assert_eq!(
            AgentApproval::find(tx.conn(), second)?.unwrap().status,
            "expired"
        );
        AgentApproval::find(tx.conn(), first)?
            .unwrap()
            .destroy(tx)?;
        assert!(!crate::sql::exists(
            tx.conn(),
            "SELECT 1 FROM activity_items WHERE source_type='AgentApproval' AND source_id=?",
            [first]
        )?);
        Ok(())
    });
}

#[test]
fn ws11_approval_stale_concurrent_decisions_append_one_event() {
    let t = setup();
    let approval = t.write(|tx| AgentApproval::create(tx, base()));
    let mut workers = vec![];
    for (decision, actor) in [("approved", "david"), ("denied", "kevin")] {
        let db = t.db.clone();
        let mut stale = approval.clone();
        workers.push(std::thread::spawn(move || {
            db.write_blocking(move |tx| {
                let user = User::find(tx.conn(), id(actor))?;
                stale.decide(tx, decision, &user, None)
            })
            .unwrap()
        }));
    }
    let outcomes: Vec<_> = workers
        .into_iter()
        .map(|worker| worker.join().unwrap())
        .collect();
    assert_eq!(outcomes.iter().filter(|e| e.is_empty()).count(), 1);
    t.read(|conn| {
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM agent_events WHERE agent_approval_id=?",
            [approval.id],
            |r| r.get(0),
        )?;
        assert_eq!(count, 1);
        Ok(())
    });
}

#[test]
fn ws11_approval_rollback_keeps_pending_inbox_and_emits_no_decision_job() {
    let t = setup();
    let mut approval = t.write(|tx| AgentApproval::create(tx, base()));
    let id = approval.id;
    let before = t.events().len();
    assert!(
        t.try_write(move |tx| {
            let user = User::find(tx.conn(), super::id("david"))?;
            assert!(approval.decide(tx, "approved", &user, None)?.is_empty());
            Err::<(), _>(crate::Error::Other("WS11 rollback".into()))
        })
        .is_err()
    );
    assert_eq!(t.events().len(), before);
    t.read(|conn| {
        assert_eq!(AgentApproval::find(conn, id)?.unwrap().status, "pending");
        assert!(
            inbox(conn, id)?
                .as_array()
                .unwrap()
                .iter()
                .all(|item| item["unread"] == true)
        );
        assert!(!crate::sql::exists(
            conn,
            "SELECT 1 FROM agent_events WHERE agent_approval_id=?",
            [id]
        )?);
        Ok(())
    });
}

#[test]
fn ws11_approvals_service_replay_grants_budgets_and_payloads_match_rails() {
    use crate::models::agent_approvals::{self as service, ApprovalRequest};
    use crate::{AgentGrant, NewGrant};
    let t = setup();
    t.write(|tx| {
        let gold: Value = serde_json::from_str(include_str!(
            "../../../../vectors/agents_approvals_service_contract.json"
        ))
        .unwrap();
        let agent = id("bender_agent");
        tx.conn().execute(
            "UPDATE sqlite_sequence SET seq=900030000 WHERE name='agent_approvals'",
            [],
        )?;
        tx.conn().execute(
            "UPDATE agents SET daily_external_action_cap=NULL WHERE id=?",
            [agent],
        )?;
        let request = || ApprovalRequest {
            action: "deploy".into(),
            summary: "Ship".into(),
            payload: Some(crate::models::agent_payloads::rails_json(
                &json!({"key":"<>&"}),
            )),
            external_id: Some("repeat".into()),
            ..Default::default()
        };
        let check = |key: &str, result: crate::models::agent_service::ServiceResult| {
            assert_eq!(
                json!({"status":result.status,"error":result.error,"payload":result.payload}),
                gold["results"][key],
                "{key}"
            );
        };
        check("forbidden", service::create(tx, agent, request(), None)?);
        check(
            "missing_before_grant",
            service::create(
                tx,
                agent,
                ApprovalRequest {
                    room_id: Some(0),
                    ..request()
                },
                None,
            )?,
        );
        check("show_missing", service::show(tx, agent, 0)?);
        let mut grant = AgentGrant::create(
            tx,
            NewGrant {
                agent_id: agent,
                granted_by_id: id("david"),
                capability: "external_action".into(),
                ..Default::default()
            },
        )?;
        check("created", service::create(tx, agent, request(), None)?);
        let approval_id = 900030001;
        tx.conn().execute(
            "UPDATE agents SET daily_external_action_cap=1 WHERE id=?",
            [agent],
        )?;
        check(
            "replay",
            service::create(
                tx,
                agent,
                ApprovalRequest {
                    action: "github.comment".into(),
                    ..request()
                },
                None,
            )?,
        );
        for (key, action) in [
            ("github_blocked", "github.comment"),
            ("fizzy_blocked", "fizzy.close"),
        ] {
            check(
                key,
                service::create(
                    tx,
                    agent,
                    ApprovalRequest {
                        action: action.into(),
                        external_id: None,
                        ..request()
                    },
                    None,
                )?,
            );
        }
        check(
            "budget",
            service::create(
                tx,
                agent,
                ApprovalRequest {
                    external_id: None,
                    ..request()
                },
                None,
            )?,
        );
        check("show", service::show(tx, agent, approval_id)?);
        check("cancel", service::cancel(tx, agent, approval_id)?);
        check("cancel_again", service::cancel(tx, agent, approval_id)?);
        tx.conn().execute(
            "UPDATE agents SET daily_external_action_cap=NULL WHERE id=?",
            [agent],
        )?;
        check(
            "invalid",
            service::create(tx, agent, ApprovalRequest::default(), None)?,
        );
        let expired = AgentApproval::create(
            tx,
            NewApproval {
                summary: "Overdue".into(),
                ..base()
            },
        )?;
        tx.conn().execute(
            "UPDATE agent_approvals SET expires_at=? WHERE id=?",
            params![tx.now(), expired.id],
        )?;
        AgentApproval::create(
            tx,
            NewApproval {
                summary: "Fresh".into(),
                ..base()
            },
        )?;
        for status in ["pending", "expired", "unknown"] {
            check(
                if status == "unknown" { "all" } else { status },
                service::list(tx, agent, Some(status))?,
            );
        }
        grant.revoke(tx)?;
        check("revoked_show", service::show(tx, agent, approval_id)?);
        check("revoked_list", service::list(tx, agent, None)?);
        Ok(())
    });
}

#[test]
fn ws11_approvals_effective_filter_precedes_limit_and_scoped_grant_show() {
    use crate::models::agent_approvals as service;
    use crate::{AgentGrant, NewGrant, Room};
    let t = setup();
    t.write(|tx| {
        let agent = id("bender_agent");
        let room = id("watercooler");
        Room::find(tx.conn(), room)?.grant_to(tx, &[id("bender")])?;
        AgentGrant::create(
            tx,
            NewGrant {
                agent_id: agent,
                granted_by_id: id("david"),
                capability: "external_action".into(),
                room_id: Some(room),
                ..Default::default()
            },
        )?;
        let fresh = AgentApproval::create(
            tx,
            NewApproval {
                room_id: Some(room),
                ..base()
            },
        )?;
        for _ in 0..110 {
            let expired = AgentApproval::create(tx, base())?;
            tx.conn().execute(
                "UPDATE agent_approvals SET expires_at=? WHERE id=?",
                params![tx.now(), expired.id],
            )?;
        }
        let result = service::list(tx, agent, Some("pending"))?;
        assert_eq!(
            result.payload.as_ref().unwrap().as_array().unwrap().len(),
            1
        );
        assert_eq!(result.payload.unwrap()[0]["id"], fresh.id);
        assert_eq!(
            service::list(tx, agent, Some("expired"))?
                .payload
                .unwrap()
                .as_array()
                .unwrap()
                .len(),
            100
        );
        assert_eq!(service::show(tx, agent, fresh.id)?.status, 200);
        let workspace = AgentApproval::create(tx, base())?;
        assert_eq!(service::show(tx, agent, workspace.id)?.status, 403);
        tx.conn().execute(
            "UPDATE rooms SET deleted_at=? WHERE id=?",
            params![tx.now(), room],
        )?;
        assert_eq!(service::show(tx, agent, fresh.id)?.status, 403);
        Ok(())
    });
}
