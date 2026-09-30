//! Named comparisons from pinned AgentApprovalTest. The preference-neighbour
//! mention assertion awaits WS12's installed activity producer.
use super::*;
use crate::models::agent_delivery::EventWebhookJob;
use crate::{AgentApproval, AgentKind, NewAgent, NewApproval, User};
use rusqlite::params;
fn setup() -> TestDb {
    let t = super::channel_thread_test::frozen();
    t.write(|tx| {
        tx.conn().execute(
            "DELETE FROM activity_items WHERE source_type='AgentApproval'",
            [],
        )?;
        tx.conn().execute("DELETE FROM agent_approvals", [])?;
        tx.conn().execute("DELETE FROM agent_events", [])?;
        Ok(())
    });
    t.sink.take();
    t
}
fn base() -> NewApproval {
    NewApproval {
        agent_id: id("bender_agent"),
        action: "deploy".into(),
        summary: "Ship it".into(),
        ..Default::default()
    }
}
fn has(errors: &crate::Errors, field: &str, text: &str) -> bool {
    errors
        .0
        .iter()
        .any(|(f, m)| *f == field && m.contains(text))
}
fn create(tx: &mut Tx<'_>) -> Result<AgentApproval> {
    AgentApproval::create(tx, base())
}
fn due(tx: &Tx<'_>, a: &AgentApproval) -> Result<()> {
    tx.conn().execute(
        "UPDATE agent_approvals SET expires_at=? WHERE id=?",
        params![tx.now().ago(jiff::SignedDuration::from_mins(1)), a.id],
    )?;
    Ok(())
}
#[test]
fn ws11_approval_case_requires_action_summary_defaults_expiry() {
    let t = setup();
    t.write(|tx| {
        let e = AgentApproval::validate(
            tx.conn(),
            &NewApproval {
                action: "".into(),
                summary: "".into(),
                ..base()
            },
            tx.now(),
            None,
        )?;
        assert!(has(&e, "action", "can't be blank"));
        assert!(has(&e, "summary", "can't be blank"));
        assert!(!e.0.iter().any(|(k, _)| *k == "expires_at"));
        Ok(())
    });
}
#[test]
fn ws11_approval_case_action_charset_and_length() {
    let t = setup();
    t.write(|tx| {
        for (action, msg) in [
            ("Deploy!".into(), "is invalid"),
            ("a".repeat(61), "too long"),
        ] {
            let e = AgentApproval::validate(
                tx.conn(),
                &NewApproval { action, ..base() },
                tx.now(),
                None,
            )?;
            assert!(has(&e, "action", msg));
        }
        assert!(
            AgentApproval::validate(
                tx.conn(),
                &NewApproval {
                    action: "deploy.prod_1-ok".into(),
                    ..base()
                },
                tx.now(),
                None
            )?
            .is_empty()
        );
        Ok(())
    });
}
#[test]
fn ws11_approval_case_summary_length() {
    let t = setup();
    t.write(|tx| {
        assert!(has(
            &AgentApproval::validate(
                tx.conn(),
                &NewApproval {
                    summary: "x".repeat(501),
                    ..base()
                },
                tx.now(),
                None
            )?,
            "summary",
            "too long"
        ));
        Ok(())
    });
}
#[test]
fn ws11_approval_case_payload_four_kilobytes() {
    let t = setup();
    t.write(|tx| {
        assert!(has(
            &AgentApproval::validate(
                tx.conn(),
                &NewApproval {
                    payload: Some("x".repeat(4097)),
                    ..base()
                },
                tx.now(),
                None
            )?,
            "payload",
            "is too large (maximum is 4 KB)"
        ));
        assert!(
            AgentApproval::validate(
                tx.conn(),
                &NewApproval {
                    payload: Some("x".repeat(4096)),
                    ..base()
                },
                tx.now(),
                None
            )?
            .is_empty()
        );
        Ok(())
    });
}
#[test]
fn ws11_approval_case_default_twenty_four_hours() {
    let t = setup();
    t.write(|tx| {
        assert_eq!(
            create(tx)?.expires_at,
            tx.now().since(jiff::SignedDuration::from_hours(24))
        );
        Ok(())
    });
}
#[test]
fn ws11_approval_case_expiry_five_minutes_to_seven_days() {
    let t = setup();
    t.write(|tx| {
        for (secs, error) in [
            (120, Some("must be at least 5 minutes from now")),
            (8 * 86400, Some("must be within 7 days from now")),
            (300, None),
            (7 * 86400, None),
        ] {
            let e = AgentApproval::validate(
                tx.conn(),
                &NewApproval {
                    expires_at: Some(tx.now().since(jiff::SignedDuration::from_secs(secs))),
                    ..base()
                },
                tx.now(),
                None,
            )?;
            match error {
                Some(msg) => assert!(has(&e, "expires_at", msg)),
                None => assert!(e.is_empty()),
            }
        }
        Ok(())
    });
}
#[test]
fn ws11_approval_case_external_id_unique_per_agent() {
    let t = setup();
    t.write(|tx| {
        AgentApproval::create(
            tx,
            NewApproval {
                external_id: Some("key-1".into()),
                ..base()
            },
        )?;
        assert!(has(
            &AgentApproval::validate(
                tx.conn(),
                &NewApproval {
                    external_id: Some("key-1".into()),
                    ..base()
                },
                tx.now(),
                None
            )?,
            "external_id",
            "has already been taken"
        ));
        let bot = User::create_bot(tx, "Approval Test Bot", None)?;
        let a = crate::Agent::create(
            tx,
            NewAgent {
                user_id: bot.id,
                owner_id: Some(id("kevin")),
                kind: AgentKind::Workspace,
                ..Default::default()
            },
        )?;
        assert!(
            AgentApproval::validate(
                tx.conn(),
                &NewApproval {
                    agent_id: a.id,
                    external_id: Some("key-1".into()),
                    ..base()
                },
                tx.now(),
                None
            )?
            .is_empty()
        );
        Ok(())
    });
}
#[test]
fn ws11_approval_case_multiple_without_external_id() {
    let t = setup();
    t.write(|tx| {
        let a = create(tx)?;
        let b = create(tx)?;
        assert_ne!(a.id, b.id);
        assert!(a.external_id.is_none() && b.external_id.is_none());
        Ok(())
    });
}
#[test]
fn ws11_approval_case_effective_expired_without_persisting() {
    let t = setup();
    t.write(|tx| {
        let a = AgentApproval::create(
            tx,
            NewApproval {
                expires_at: Some(tx.now().since(jiff::SignedDuration::from_mins(6))),
                ..base()
            },
        )?;
        assert_eq!(a.effective_status(tx.now()), "pending");
        due(tx, &a)?;
        let a = AgentApproval::find(tx.conn(), a.id)?.unwrap();
        assert_eq!(a.effective_status(tx.now()), "expired");
        assert_eq!(a.status, "pending");
        Ok(())
    });
}
#[test]
fn ws11_approval_case_expire_only_overdue_pending() {
    let t = setup();
    t.write(|tx| {
        let mut a = create(tx)?;
        assert!(!a.expire_if_due(tx)?);
        assert_eq!(a.status, "pending");
        let mut b = create(tx)?;
        due(tx, &b)?;
        assert!(b.expire_if_due(tx)?);
        assert_eq!(b.status, "expired");
        let mut c = create(tx)?;
        assert!(
            c.decide(tx, "approved", &User::find(tx.conn(), id("david"))?, None)?
                .is_empty()
        );
        due(tx, &c)?;
        assert!(!c.expire_if_due(tx)?);
        assert_eq!(c.status, "approved");
        Ok(())
    });
}
#[test]
fn ws11_approval_case_decide_rejects_expired_and_decided() {
    let t = setup();
    t.write(|tx| {
        let by = User::find(tx.conn(), id("david"))?;
        let mut a = create(tx)?;
        due(tx, &a)?;
        assert!(has(
            &a.decide(tx, "approved", &by, None)?,
            "base",
            "expired"
        ));
        assert_eq!(a.status, "expired");
        let mut b = create(tx)?;
        assert!(b.decide(tx, "denied", &by, Some("nope"))?.is_empty());
        assert_eq!(b.status, "denied");
        assert_eq!(b.decision_note.as_deref(), Some("nope"));
        assert!(has(
            &b.decide(tx, "approved", &by, None)?,
            "base",
            "already denied"
        ));
        Ok(())
    });
}
#[test]
fn ws11_approval_case_cancel_rejects_expired_and_decided() {
    let t = setup();
    t.write(|tx| {
        let mut a = create(tx)?;
        assert!(a.cancel_by_agent(tx)?.is_empty());
        assert_eq!(a.status, "cancelled");
        assert!(has(&a.cancel_by_agent(tx)?, "base", "already cancelled"));
        let mut b = create(tx)?;
        due(tx, &b)?;
        assert!(has(&b.cancel_by_agent(tx)?, "base", "expired"));
        Ok(())
    });
}
#[test]
fn ws11_approval_case_decision_note_length() {
    let t = setup();
    t.write(|tx| {
        let mut a = create(tx)?;
        let by = User::find(tx.conn(), id("david"))?;
        assert!(has(
            &a.decide(tx, "denied", &by, Some(&"x".repeat(201)))?,
            "decision_note",
            "too long"
        ));
        assert_eq!(
            AgentApproval::find(tx.conn(), a.id)?.unwrap().status,
            "pending"
        );
        Ok(())
    });
}
#[test]
fn ws11_approval_case_inbox_per_decider() {
    let t = setup();
    let a = t.write(|tx| {
        tx.conn().execute("UPDATE agents SET owner_id=? WHERE id=?", params![id("kevin"),id("bender_agent")])?;
        let a = create(tx)?;
        assert_eq!(tx.conn().query_row("SELECT COUNT(*) FROM activity_items WHERE source_type='AgentApproval' AND source_id=?", [a.id], |r| r.get::<_,i64>(0))?, 0);
        Ok(a)
    });
    t.read(|c| {
        let mut actual = crate::sql::query_all(c,"SELECT user_id FROM activity_items WHERE source_type='AgentApproval' AND source_id=? AND event_type='agent_approval_request'",[a.id],|r|r.get::<_,i64>(0))?;
        actual.sort_unstable();
        let mut expected=vec![id("david"),id("jason"),id("kevin")];expected.sort_unstable();
        assert_eq!(actual,expected);Ok(())
    });
}
#[test]
fn ws11_approval_case_ownerless_admin_deciders() {
    let t = setup();
    t.write(|tx| {
        tx.conn().execute(
            "UPDATE agents SET owner_id=NULL WHERE id=?",
            [id("bender_agent")],
        )?;
        let a = create(tx)?;
        let mut actual = a.decider_ids(tx.conn())?;
        actual.sort_unstable();
        let mut expected = vec![id("david"), id("jason")];
        expected.sort_unstable();
        assert_eq!(actual, expected);
        Ok(())
    });
}
#[test]
fn ws11_approval_case_decision_enqueues_after_commit() {
    let t = setup();
    let aid = t.write(|tx| Ok(create(tx)?.id));
    t.sink.take();
    t.write(move |tx| {
        AgentApproval::find(tx.conn(), aid)?.unwrap().decide(
            tx,
            "approved",
            &User::find(tx.conn(), id("david"))?,
            None,
        )
    });
    assert_eq!(
        t.events()
            .iter()
            .filter_map(|e| e.as_job::<EventWebhookJob>())
            .count(),
        1
    );
}
#[test]
fn ws11_approval_case_decision_waits_for_outer_transaction() {
    let t = setup();
    let aid = t.write(|tx| Ok(create(tx)?.id));
    t.sink.take();
    let sink = t.sink.clone();
    t.write(move |tx| {
        tx.savepoint(|tx| {
            assert!(
                AgentApproval::find(tx.conn(), aid)?
                    .unwrap()
                    .decide(tx, "approved", &User::find(tx.conn(), id("david"))?, None)?
                    .is_empty()
            );
            assert!(sink.events().is_empty());
            Ok(())
        })?;
        assert!(sink.events().is_empty());
        Ok(())
    });
    assert_eq!(
        t.events()
            .iter()
            .filter_map(|e| e.as_job::<EventWebhookJob>())
            .count(),
        1
    );
}
#[test]
fn ws11_approval_case_second_stale_decision_no_event() {
    let t = setup();
    t.write(|tx|{let by=User::find(tx.conn(),id("david"))?;let mut a=create(tx)?;let mut stale=a.clone();assert!(a.decide(tx,"approved",&by,None)?.is_empty());assert_eq!(stale.status,"pending");let count=|c:&Connection|c.query_row("SELECT COUNT(*) FROM agent_events WHERE agent_approval_id=? AND event_type='approval_decided'",[a.id],|r|r.get::<_,i64>(0));let before=count(tx.conn())?;assert_eq!(before,1);assert!(!stale.decide(tx,"denied",&by,None)?.is_empty());assert_eq!(count(tx.conn())?,before);assert_eq!(AgentApproval::find(tx.conn(),a.id)?.unwrap().status,"approved");Ok(())});
}
#[test]
fn ws11_approval_case_cancel_and_expire_handle_inbox() {
    let t = setup();
    let mut a = t.write(create);
    let aid = a.id;
    t.write(move |tx| { assert!(a.cancel_by_agent(tx)?.is_empty()); Ok(()) });
    let (aid, bid) = t.write(move |tx| {
        let b = AgentApproval::create(
            tx,
            NewApproval {
                expires_at: Some(tx.now().since(jiff::SignedDuration::from_hours(1))),
                ..base()
            },
        )?;
        Ok((aid, b.id))
    });
    t.travel(7200);
    t.write(move|tx|{assert!(AgentApproval::find(tx.conn(),bid)?.unwrap().expire_if_due(tx)?);for mid in [aid,bid] {let total:i64=tx.conn().query_row("SELECT COUNT(*) FROM activity_items WHERE source_type='AgentApproval' AND source_id=?",[mid],|r|r.get(0))?;let handled:i64=tx.conn().query_row("SELECT COUNT(*) FROM activity_items WHERE source_type='AgentApproval' AND source_id=? AND handled_at IS NOT NULL",[mid],|r|r.get(0))?;assert!(total>0);assert_eq!(handled,total);}Ok(())});
}

#[test]
fn ws11r_approval_inbox_failure_retains_primary_record_like_rails() {
    let t = TestDb::new();
    t.write(|tx| {
        tx.conn().execute_batch("CREATE TEMP TRIGGER ws11r_reject_inbox BEFORE INSERT ON activity_items WHEN NEW.source_type='AgentApproval' BEGIN SELECT RAISE(ABORT, 'ws11r inbox failure'); END")?;
        Ok(())
    });
    let result = t.try_write(|tx| crate::AgentApproval::create(tx, crate::NewApproval {
        agent_id: id("bender_agent"), action: "deploy".into(), summary: "ws11r inbox failure".into(), external_id: Some("ws11r-inbox-failure".into()), ..Default::default()
    }));
    assert!(result.is_err(), "inbox insert was rejected");
    let count: i64 = t.read(|c| Ok(c.query_row("SELECT COUNT(*) FROM agent_approvals WHERE external_id='ws11r-inbox-failure'", [], |r| r.get(0))?));
    println!("WS11R approval inbox failure: persisted approvals = {count}");
    assert_eq!(count, 1, "Rails retains the approval after its after_create_commit fails");
}
