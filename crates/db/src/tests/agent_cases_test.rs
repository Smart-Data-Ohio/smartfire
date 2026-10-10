//! Named comparisons against pinned test/models/agent_test.rb.
//! The two HTML broadcast cases remain with WS11-ui; they are not counted here.
use super::*;
use crate::models::agent::STATUSES;
use crate::models::agent_delivery::{AgentEvent, NewEvent};
use crate::{Agent, AgentChanges, AgentGrant, AgentKind, NewAgent, NewGrant, User};
use rusqlite::params;
fn frozen() -> TestDb {
    let t = super::channel_thread_test::frozen();
    t.write(|tx| {
        tx.conn().execute("DELETE FROM agent_grants", [])?;
        tx.conn().execute("DELETE FROM agent_events", [])?;
        Ok(())
    });
    t.sink.take();
    t
}
fn agent(tx: &Tx<'_>) -> Result<Agent> {
    Ok(Agent::find(tx.conn(), id("bender_agent"))?.unwrap())
}
fn attributes() -> NewAgent {
    NewAgent {
        user_id: id("bender"),
        owner_id: Some(id("david")),
        kind: AgentKind::Workspace,
        ..Default::default()
    }
}
fn grant(tx: &Tx<'_>, room: Option<i64>, cap: &str) -> Result<AgentGrant> {
    AgentGrant::create(
        tx,
        NewGrant {
            agent_id: id("bender_agent"),
            granted_by_id: id("david"),
            room_id: room,
            capability: cap.into(),
            ..Default::default()
        },
    )
}
fn error(errors: crate::Errors, field: &str, text: &str) -> bool {
    errors.0.iter().any(|(f, m)| *f == field && m == text)
}
fn crypto() -> rails_compat::ar_encryption::ArEncryption {
    rails_compat::ar_encryption::ArEncryption::new(&rails_compat::Secrets::new(
        &"ws11 public test material ".repeat(8),
    ))
}

#[test]
fn ws11_agent_case_kind_defaults_to_personal() {
    assert_eq!(NewAgent::default().kind, AgentKind::Personal);
}

#[test]
fn ws11_agent_case_personal_requires_an_owner_on_create() {
    let t = frozen();
    t.read(|c| {
        let a = NewAgent {
            owner_id: None,
            kind: AgentKind::Personal,
            ..attributes()
        };
        assert!(error(
            Agent::validate(c, &a, None)?,
            "owner_id",
            "can't be blank"
        ));
        Ok(())
    });
}

#[test]
fn ws11_agent_case_personal_requires_an_owner_on_update() {
    let t = frozen();
    t.write(|tx| {
        let mut a = agent(tx)?;
        a.update(
            tx,
            AgentChanges {
                kind: Some(AgentKind::Personal),
                ..Default::default()
            },
        )?;
        assert!(
            a.update(
                tx,
                AgentChanges {
                    owner_id: Some(None),
                    ..Default::default()
                }
            )
            .is_err()
        );
        Ok(())
    });
}

#[test]
fn ws11_agent_case_workspace_requires_an_owner_on_create() {
    let t = frozen();
    t.read(|c| {
        assert!(error(
            Agent::validate(
                c,
                &NewAgent {
                    owner_id: None,
                    ..attributes()
                },
                None
            )?,
            "owner_id",
            "can't be blank"
        ));
        Ok(())
    });
}

#[test]
fn ws11_agent_case_ownerless_workspace_rows_from_the_backfill_stay_valid_on_update() {
    let t = frozen();
    t.write(|tx| {
        tx.conn().execute(
            "UPDATE agents SET owner_id=NULL WHERE id=?",
            [id("bender_agent")],
        )?;
        let mut a = agent(tx)?;
        a.save(tx)?;
        a.update(
            tx,
            AgentChanges {
                description: Some(Some("Backfilled bot".into())),
                ..Default::default()
            },
        )?;
        assert_eq!(agent(tx)?.description.as_deref(), Some("Backfilled bot"));
        Ok(())
    });
}

#[test]
fn ws11_agent_case_user_is_required_and_unique() {
    let t = frozen();
    t.read(|c| {
        assert!(error(
            Agent::validate(
                c,
                &NewAgent {
                    user_id: 0,
                    ..attributes()
                },
                None
            )?,
            "user",
            "must exist"
        ));
        assert!(error(
            Agent::validate(c, &attributes(), None)?,
            "user_id",
            "has already been taken"
        ));
        Ok(())
    });
}

#[test]
fn ws11_agent_case_belongs_to_bot_user_and_owner() {
    let t = frozen();
    t.read(|c| {
        let a = Agent::find(c, id("bender_agent"))?.unwrap();
        assert_eq!(User::find(c, a.user_id)?.id, id("bender"));
        assert_eq!(a.owner_id, Some(id("david")));
        assert_eq!(Agent::for_user(c, id("bender"))?.unwrap().id, a.id);
        Ok(())
    });
}

#[test]
fn ws11_agent_case_destroying_the_bot_user_removes_its_agent() {
    let t = frozen();
    t.write(|tx| User::find(tx.conn(), id("bender"))?.destroy(tx));
    t.read(|c| {
        assert!(Agent::for_user(c, id("bender"))?.is_none());
        Ok(())
    });
}

#[test]
fn ws11_agent_case_active_when_not_suspended_and_user_is_active() {
    let t = frozen();
    t.read(|c| {
        assert!(Agent::find(c, id("bender_agent"))?.unwrap().active(c)?);
        Ok(())
    });
}

#[test]
fn ws11_agent_case_inactive_when_suspended() {
    let t = frozen();
    t.write(|tx| {
        let mut a = agent(tx)?;
        a.update(
            tx,
            AgentChanges {
                suspended_at: Some(Some(tx.now())),
                ..Default::default()
            },
        )?;
        assert!(!a.active(tx.conn())?);
        Ok(())
    });
}

#[test]
fn ws11_agent_case_inactive_when_user_is_deactivated() {
    let t = frozen();
    t.write(|tx| {
        User::find(tx.conn(), id("bender"))?.update(
            tx,
            crate::UserChanges {
                status: Some(crate::models::user::Status::Deactivated),
                ..Default::default()
            },
        )?;
        assert!(!agent(tx)?.active(tx.conn())?);
        Ok(())
    });
}

#[test]
fn ws11_agent_case_legacy_capabilities_when_no_grants_have_ever_existed() {
    let t = frozen();
    t.read(|c| {
        assert!(
            Agent::find(c, id("bender_agent"))?
                .unwrap()
                .legacy_capabilities(c)?
        );
        Ok(())
    });
}

#[test]
fn ws11_agent_case_no_legacy_capabilities_once_any_grant_exists() {
    let t = frozen();
    t.write(|tx| {
        grant(tx, None, "post_messages")?;
        assert!(!agent(tx)?.legacy_capabilities(tx.conn())?);
        Ok(())
    });
}

#[test]
fn ws11_agent_case_revoking_the_last_grant_does_not_restore_the_legacy_fallback() {
    let t = frozen();
    t.write(|tx| {
        grant(tx, None, "post_messages")?.revoke(tx)?;
        let a = agent(tx)?;
        assert!(!a.legacy_capabilities(tx.conn())?);
        assert!(!a.can(tx.conn(), "post_messages", Some(id("watercooler")))?);
        Ok(())
    });
}

#[test]
fn ws11_agent_case_legacy_agent_keeps_read_post_and_react_but_nothing_else() {
    let t = frozen();
    t.read(|c| {
        let a = Agent::find(c, id("bender_agent"))?.unwrap();
        for cap in ["read_messages", "post_messages", "react"] {
            assert!(a.can(c, cap, Some(id("watercooler")))?)
        }
        for cap in ["manage_threads", "external_action"] {
            assert!(!a.can(c, cap, Some(id("watercooler")))?)
        }
        Ok(())
    });
}

#[test]
fn ws11_agent_case_room_grant_authorizes_only_that_room() {
    let t = frozen();
    t.write(|tx| {
        grant(tx, Some(id("watercooler")), "post_messages")?;
        let a = agent(tx)?;
        assert!(a.can(tx.conn(), "post_messages", Some(id("watercooler")))?);
        assert!(!a.can(tx.conn(), "post_messages", Some(id("designers")))?);
        assert!(!a.can(tx.conn(), "react", Some(id("watercooler")))?);
        Ok(())
    });
}

#[test]
fn ws11_agent_case_workspace_wide_grant_authorizes_every_room() {
    let t = frozen();
    t.write(|tx| {
        grant(tx, None, "post_messages")?;
        let a = agent(tx)?;
        for room in ["watercooler", "designers"] {
            assert!(a.can(tx.conn(), "post_messages", Some(id(room)))?)
        }
        Ok(())
    });
}

#[test]
fn ws11_agent_case_revoked_grants_do_not_authorize() {
    let t = frozen();
    t.write(|tx| {
        grant(tx, Some(id("watercooler")), "post_messages")?.revoke(tx)?;
        assert!(!agent(tx)?.can(tx.conn(), "post_messages", Some(id("watercooler")))?);
        Ok(())
    });
}

#[test]
fn ws11_agent_case_suspended_agent_cannot_do_anything_even_with_legacy_fallback() {
    let t = frozen();
    t.write(|tx| {
        crate::models::agent_lifecycle::suspend(tx, id("bender_agent"), &Default::default())?;
        for cap in ["post_messages", "read_messages"] {
            assert!(!agent(tx)?.can(tx.conn(), cap, Some(id("watercooler")))?)
        }
        Ok(())
    });
}

#[test]
fn ws11_agent_case_unknown_capabilities_are_denied() {
    let t = frozen();
    t.read(|c| {
        assert!(!Agent::find(c, id("bender_agent"))?.unwrap().can(
            c,
            "launch_missiles",
            Some(id("watercooler"))
        )?);
        Ok(())
    });
}

#[test]
fn ws11_agent_case_status_defaults_to_idle() {
    assert_eq!(NewAgent::default().status, "idle");
    let t = frozen();
    t.read(|c| {
        assert_eq!(Agent::find(c, id("bender_agent"))?.unwrap().status, "idle");
        Ok(())
    });
}

#[test]
fn ws11_agent_case_status_accepts_the_vocabulary_and_rejects_anything_else() {
    let t = frozen();
    t.read(|c| {
        for status in STATUSES {
            assert!(
                Agent::validate(
                    c,
                    &NewAgent {
                        status: status.into(),
                        ..attributes()
                    },
                    Some(id("bender_agent"))
                )?
                .is_empty()
            )
        }
        assert!(error(
            Agent::validate(
                c,
                &NewAgent {
                    status: "napping".into(),
                    ..attributes()
                },
                Some(id("bender_agent"))
            )?,
            "status",
            "is not included in the list"
        ));
        Ok(())
    });
}

#[test]
fn ws11_agent_case_status_note_allows_200_characters_at_most() {
    let t = frozen();
    t.read(|c| {
        assert!(
            Agent::validate(
                c,
                &NewAgent {
                    status_note: Some("x".repeat(200)),
                    ..attributes()
                },
                Some(id("bender_agent"))
            )?
            .is_empty()
        );
        assert!(
            !Agent::validate(
                c,
                &NewAgent {
                    status_note: Some("x".repeat(201)),
                    ..attributes()
                },
                Some(id("bender_agent"))
            )?
            .is_empty()
        );
        Ok(())
    });
}

#[test]
fn ws11_agent_case_description_allows_500_characters_at_most() {
    let t = frozen();
    t.read(|c| {
        assert!(
            Agent::validate(
                c,
                &NewAgent {
                    description: Some("x".repeat(500)),
                    ..attributes()
                },
                Some(id("bender_agent"))
            )?
            .is_empty()
        );
        assert!(
            !Agent::validate(
                c,
                &NewAgent {
                    description: Some("x".repeat(501)),
                    ..attributes()
                },
                Some(id("bender_agent"))
            )?
            .is_empty()
        );
        Ok(())
    });
}

#[test]
fn ws11_agent_case_status_changed_at_starts_blank_and_is_stamped_on_status_change_only() {
    let t = frozen();
    t.write(|tx| {
        let u = User::create_email_bot(tx)?;
        let mut a = Agent::create(
            tx,
            NewAgent {
                user_id: u.id,
                ..attributes()
            },
        )?;
        assert!(a.status_changed_at.is_none());
        a.update(
            tx,
            AgentChanges {
                status_note: Some(Some("just a note".into())),
                ..Default::default()
            },
        )?;
        assert!(a.status_changed_at.is_none());
        a.update(
            tx,
            AgentChanges {
                provider: Some(Some("Acme".into())),
                ..Default::default()
            },
        )?;
        assert!(a.status_changed_at.is_none());
        a.update(
            tx,
            AgentChanges {
                status: Some("working".into()),
                ..Default::default()
            },
        )?;
        assert_eq!(a.status_changed_at, Some(tx.now()));
        Ok(())
    });
}

#[test]
fn ws11_agent_case_touch_last_seen_at_throttles_to_once_per_minute() {
    let t = frozen();
    let first = t.write(|tx| {
        let mut a = agent(tx)?;
        assert!(a.last_seen_at.is_none());
        a.touch_last_seen(tx)?;
        let first = a.last_seen_at.unwrap();
        a.touch_last_seen(tx)?;
        assert_eq!(a.last_seen_at, Some(first));
        tx.conn().execute(
            "UPDATE agents SET last_seen_at=? WHERE id=?",
            params![tx.now().ago(jiff::SignedDuration::from_secs(61)), a.id],
        )?;
        Ok(first)
    });
    t.travel(1);
    t.write(move |tx| {
        let mut a = agent(tx)?;
        a.touch_last_seen(tx)?;
        assert!(a.last_seen_at.unwrap() > first);
        Ok(())
    });
}

#[test]
fn ws11_agent_case_kind_description_covers_personal_workspace_and_ownerless_agents() {
    let t = frozen();
    t.write(|tx| {
        let mut a = agent(tx)?;
        a.update(
            tx,
            AgentChanges {
                kind: Some(AgentKind::Personal),
                owner_id: Some(Some(id("kevin"))),
                ..Default::default()
            },
        )?;
        assert_eq!(a.kind_description(tx.conn())?, "Personal agent of Kevin");
        a.update(
            tx,
            AgentChanges {
                kind: Some(AgentKind::Workspace),
                ..Default::default()
            },
        )?;
        assert_eq!(
            a.kind_description(tx.conn())?,
            "Workspace agent, managed by Kevin"
        );
        tx.conn()
            .execute("UPDATE agents SET owner_id=NULL WHERE id=?", [a.id])?;
        assert_eq!(agent(tx)?.kind_description(tx.conn())?, "no owner recorded");
        Ok(())
    });
}

#[test]
fn ws11_agent_case_grants_summary_reports_legacy_access_when_no_grants_were_ever_recorded() {
    let t = frozen();
    t.read(|c| {
        assert_eq!(
            Agent::find(c, id("bender_agent"))?
                .unwrap()
                .grants_summary(c)?,
            "legacy access (no grants recorded)"
        );
        Ok(())
    });
}

#[test]
fn ws11_agent_case_grants_summary_counts_room_grants_and_flags_workspace_wide_grants() {
    let t = frozen();
    t.write(|tx| {
        for r in ["watercooler", "designers", "hq"] {
            grant(tx, Some(id(r)), "post_messages")?;
        }
        grant(tx, None, "read_messages")?;
        assert_eq!(
            agent(tx)?.grants_summary(tx.conn())?,
            "post_messages in 3 rooms, read_messages workspace-wide"
        );
        Ok(())
    });
}

#[test]
fn ws11_agent_case_grants_summary_omits_capabilities_with_no_active_grant() {
    let t = frozen();
    t.write(|tx| {
        grant(tx, Some(id("watercooler")), "react")?.revoke(tx)?;
        grant(tx, None, "read_messages")?;
        assert_eq!(
            agent(tx)?.grants_summary(tx.conn())?,
            "read_messages workspace-wide"
        );
        Ok(())
    });
}

#[test]
fn ws11_agent_case_grants_summary_reports_no_active_grants_once_everything_is_revoked() {
    let t = frozen();
    t.write(|tx| {
        grant(tx, None, "read_messages")?.revoke(tx)?;
        assert_eq!(agent(tx)?.grants_summary(tx.conn())?, "no active grants");
        Ok(())
    });
}

#[test]
fn ws11_agent_case_activity_summary_counts_the_last_24_hours_from_the_ledger() {
    let t = frozen();
    t.write(|tx| {
        let mut old = 0;
        for (kind, outcome) in [
            ("mention", "delivered"),
            ("reply", "acknowledged"),
            ("posted", "delivered"),
            ("delivery_suppressed_rate_limit", "suppressed"),
            ("mention", "delivered"),
        ] {
            old = AgentEvent::create(
                tx,
                NewEvent {
                    agent_id: id("bender_agent"),
                    room_id: Some(id("watercooler")),
                    event_type: kind.into(),
                    outcome: Some(outcome.into()),
                    ..Default::default()
                },
            )?
            .id;
        }
        tx.conn().execute(
            "UPDATE agent_events SET created_at=? WHERE id=?",
            params![tx.now().ago(jiff::SignedDuration::from_hours(25)), old],
        )?;
        assert_eq!(
            agent(tx)?.activity_summary(tx.conn(), tx.now())?,
            "1 delivered, 1 acknowledged, 1 posted, 1 suppressed"
        );
        Ok(())
    });
}

#[test]
fn ws11_agent_case_for_directory_lists_active_first_excludes_deactivated_users() {
    let t = frozen();
    t.write(|tx| {
        let mut u = User::create_email_bot(tx)?;
        u.update(
            tx,
            crate::UserChanges {
                name: Some("Zed Suspended".into()),
                ..Default::default()
            },
        )?;
        let mut s = Agent::create(
            tx,
            NewAgent {
                user_id: u.id,
                ..attributes()
            },
        )?;
        s.update(
            tx,
            AgentChanges {
                suspended_at: Some(Some(tx.now())),
                ..Default::default()
            },
        )?;
        let mut gone = User::create_email_bot(tx)?;
        gone.update(
            tx,
            crate::UserChanges {
                name: Some("Aaron Gone".into()),
                ..Default::default()
            },
        )?;
        Agent::create(
            tx,
            NewAgent {
                user_id: gone.id,
                ..attributes()
            },
        )?;
        gone.deactivate(tx)?;
        assert_eq!(
            Agent::for_directory(tx.conn())?
                .iter()
                .map(|a| a.id)
                .collect::<Vec<_>>(),
            vec![id("bender_agent"), s.id]
        );
        Ok(())
    });
}

#[test]
fn ws11_agent_case_note_only_change_still_broadcasts_the_badge() {
    let t = frozen();
    t.write(|tx| {
        agent(tx)?.update(
            tx,
            AgentChanges {
                status_note: Some(Some("still here".into())),
                ..Default::default()
            },
        )
    });
    assert_eq!(
        t.events()
            .iter()
            .filter(|e| matches!(e,Event::Broadcast(b) if b.kind=="Agent#sync_status"))
            .count(),
        1
    );
}

#[test]
fn ws11_agent_case_touch_last_seen_at_writes_at_most_once_when_the_throttle_races() {
    let t = frozen();
    t.write(|tx| {
        let mut a = agent(tx)?;
        assert!(a.last_seen_at.is_none());
        let winner = tx.now().ago(jiff::SignedDuration::from_secs(10));
        tx.conn().execute(
            "UPDATE agents SET last_seen_at=? WHERE id=?",
            params![winner, a.id],
        )?;
        a.touch_last_seen(tx)?;
        assert_eq!(agent(tx)?.last_seen_at, Some(winner));
        Ok(())
    });
}

#[test]
fn ws11_agent_case_signing_secret_generation_mints_exactly_one_secret_when_raced() {
    let t = frozen();
    t.write(|tx| {
        let stale = agent(tx)?;
        tx.conn().execute(
            "UPDATE agents SET webhook_signing_secret=NULL WHERE id=?",
            [stale.id],
        )?;
        let crypto = crypto();
        let winner =
            crate::models::agent_access::ensure_webhook_signing_secret(tx, &crypto, stale.id)?;
        let ciphertext: String = tx.conn().query_row(
            "SELECT webhook_signing_secret FROM agents WHERE id=?",
            [stale.id],
            |r| r.get(0),
        )?;
        assert_eq!(
            crate::models::agent_access::ensure_webhook_signing_secret(tx, &crypto, stale.id)?,
            winner
        );
        assert_eq!(
            tx.conn().query_row(
                "SELECT webhook_signing_secret FROM agents WHERE id=?",
                [stale.id],
                |r| r.get::<_, String>(0)
            )?,
            ciphertext
        );
        assert_eq!(crypto.decrypt(&ciphertext).unwrap(), winner);
        Ok(())
    });
}

#[test]
fn ws11_agent_case_signing_secret_reset_takes_the_row_lock() {
    let t = frozen();
    t.write(|tx| {
        assert!(tx.in_transaction());
        let crypto = crypto();
        let first = crate::models::agent_access::ensure_webhook_signing_secret(
            tx,
            &crypto,
            id("bender_agent"),
        )?;
        let reset = crate::models::agent_access::reset_webhook_signing_secret(
            tx,
            &crypto,
            id("bender_agent"),
        )?;
        assert!(!reset.is_empty());
        assert_ne!(first, reset);
        assert_eq!(
            crate::models::agent_access::ensure_webhook_signing_secret(
                tx,
                &crypto,
                id("bender_agent")
            )?,
            reset
        );
        Ok(())
    });
}

#[test]
fn ws11_agent_case_last_seen_at_touch_alone_broadcasts_nothing() {
    let t = frozen();
    t.write(|tx| agent(tx)?.touch_last_seen(tx));
    assert!(t.events().is_empty());
}

#[test]
fn ws11_agent_case_unrelated_updates_broadcast_nothing() {
    let t = frozen();
    t.write(|tx| {
        agent(tx)?.update(
            tx,
            AgentChanges {
                provider: Some(Some("Acme".into())),
                runtime: Some(Some("CLI".into())),
                description: Some(Some("hi".into())),
                ..Default::default()
            },
        )
    });
    assert!(t.events().is_empty());
}
