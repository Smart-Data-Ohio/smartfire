use super::*;
use crate::models::{
    agent_access, agent_approvals,
    agent_delivery::{AgentEvent, NewEvent},
    agent_direct_messages::{self, DirectMessageResult},
    agent_event_access, agent_lifecycle,
    agent_posting::DriveInput,
    audit_log::Context,
};
use crate::{
    Agent, AgentChanges, AgentCredential, AgentGrant, GrantChanges, NewGrant, NewMessage, Room,
    User,
};
use rusqlite::params;

fn personal() -> TestDb {
    let t = super::channel_thread_test::frozen();
    t.write(|tx| {
        tx.conn().execute(
            "DELETE FROM agent_grants WHERE agent_id=?",
            [id("bender_agent")],
        )?;
        tx.conn().execute(
            "UPDATE agents SET kind='personal',owner_id=? WHERE id=?",
            params![id("david"), id("bender_agent")],
        )?;
        Ok(())
    });
    t
}

fn plant_legacy_grants(
    conn: &Connection,
    now: crate::Timestamp,
    revoked: bool,
) -> Result<Vec<i64>> {
    let direct = conn.query_row("INSERT INTO rooms(name,type,creator_id,created_at,updated_at) VALUES ('Legacy direct','Rooms::Direct',?,?,?) RETURNING id", params![id("david"),now,now], |row| row.get::<_,i64>(0))?;
    let mut grants = Vec::new();
    for (cap, room) in [
        ("external_action", None),
        ("fizzy", Some(id("watercooler"))),
        ("dm_anyone", None),
        ("dm_anyone", Some(id("watercooler"))),
        ("read_messages", None),
        ("react", Some(direct)),
        ("legacy_action", Some(id("watercooler"))),
    ] {
        grants.push(conn.query_row("INSERT INTO agent_grants(agent_id,room_id,capability,granted_by_id,revoked_at,created_at,updated_at) VALUES (?,?,?,?,?,?,?) RETURNING id", params![id("bender_agent"),room,cap,id("david"),revoked.then_some(now),now,now], |row| row.get(0))?);
    }
    Ok(grants)
}

fn legacy_personal(revoked: bool) -> TestDb {
    let t = personal();
    t.write(move |tx| {
        tx.conn()
            .execute_batch("DROP TRIGGER personal_agent_grant_insert;")?;
        plant_legacy_grants(tx.conn(), tx.now(), revoked)?;
        Ok(())
    });
    t
}

#[test]
fn review_s1_legacy_grant_revocation_never_validates_the_old_shape() {
    let t = legacy_personal(false);
    t.write(|tx| {
        let rows = crate::sql::query_all(
            tx.conn(),
            "SELECT id FROM agent_grants WHERE agent_id=? ORDER BY id",
            [id("bender_agent")],
            |row| row.get::<_, i64>(0),
        )?;
        for grant_id in rows {
            let mut grant = AgentGrant::find(tx.conn(), grant_id)?.unwrap();
            let shape = (grant.agent_id, grant.room_id, grant.capability.clone());
            grant.revoke(tx)?;
            let stamp = grant.revoked_at;
            grant.revoke(tx)?;
            let current = AgentGrant::find(tx.conn(), grant_id)?.unwrap();
            assert_eq!(current.revoked_at, stamp);
            assert!(current.revoked());
            assert_eq!(
                (current.agent_id, current.room_id, current.capability),
                shape
            );
        }
        Ok(())
    });
}

#[test]
fn review_s1_legacy_grant_update_can_revoke_without_changing_shape() {
    let t = legacy_personal(false);
    t.write(|tx| {
        let rows = crate::sql::query_all(
            tx.conn(),
            "SELECT id FROM agent_grants WHERE agent_id=? ORDER BY id",
            [id("bender_agent")],
            |row| row.get::<_, i64>(0),
        )?;
        for grant_id in rows {
            let mut grant = AgentGrant::find(tx.conn(), grant_id)?.unwrap();
            grant.update(
                tx,
                GrantChanges {
                    revoked_at: Some(Some(tx.now())),
                    ..Default::default()
                },
            )?;
            assert!(AgentGrant::find(tx.conn(), grant_id)?.unwrap().revoked());
            assert!(matches!(
                grant.update(
                    tx,
                    GrantChanges {
                        capability: Some("another_legacy_action".into()),
                        ..Default::default()
                    }
                ),
                Err(crate::Error::RecordInvalid(_))
            ));
        }
        Ok(())
    });
}

#[test]
fn review_s1_legacy_noop_shape_updates_permit_revocation_and_suspension() {
    let t = legacy_personal(false);
    t.write(|tx| {
        assert_eq!(tx.conn().execute("UPDATE agent_grants SET agent_id=agent_id,capability=capability,room_id=room_id,revoked_at=? WHERE agent_id=?",params![tx.now(),id("bender_agent")])?,7);
        assert_eq!(tx.conn().execute("UPDATE agents SET kind=kind,suspended_at=? WHERE id=?",params![tx.now(),id("bender_agent")])?,1);
        assert!(Agent::find(tx.conn(),id("bender_agent"))?.unwrap().suspended());
        Ok(())
    });
}

#[test]
fn review_s1_legacy_grants_cannot_block_suspension_or_agent_saves() {
    for revoked in [false, true] {
        let t = legacy_personal(revoked);
        t.write(|tx| {
            let mut agent = Agent::find(tx.conn(), id("bender_agent"))?.unwrap();
            agent.update(
                tx,
                AgentChanges {
                    kind: Some(crate::AgentKind::Personal),
                    description: Some(Some("Existing personal agent".into())),
                    ..Default::default()
                },
            )?;
            agent.description = Some("Saved directly".into());
            agent.save(tx)?;
            agent_lifecycle::suspend(tx, agent.id, &Context::default())?;
            assert!(Agent::find(tx.conn(), agent.id)?.unwrap().suspended());
            assert!(!crate::sql::exists(
                tx.conn(),
                "SELECT 1 FROM agent_grants WHERE agent_id=? AND revoked_at IS NULL",
                [agent.id]
            )?);
            Ok(())
        });
    }
}

#[test]
fn review_s1_legacy_grants_cannot_block_the_kill_switch() {
    for revoked in [false, true] {
        let t = legacy_personal(revoked);
        t.write(|tx| {
            let approval = crate::AgentApproval::create(
                tx,
                crate::NewApproval {
                    agent_id: id("bender_agent"),
                    action: "deploy".into(),
                    summary: "Pending legacy action".into(),
                    ..Default::default()
                },
            )?;
            assert_eq!(
                agent_lifecycle::kill_switch(tx, id("bender_agent"), &Context::default())?,
                1
            );
            assert!(
                Agent::find(tx.conn(), id("bender_agent"))?
                    .unwrap()
                    .suspended()
            );
            assert_eq!(
                crate::AgentApproval::find(tx.conn(), approval.id)?
                    .unwrap()
                    .status,
                "cancelled"
            );
            assert!(!crate::sql::exists(
                tx.conn(),
                "SELECT 1 FROM agent_grants WHERE agent_id=? AND revoked_at IS NULL",
                [id("bender_agent")]
            )?);
            Ok(())
        });
    }
}

#[test]
fn review_s1_legacy_grants_cannot_block_owner_deactivation() {
    for revoked in [false, true] {
        let t = legacy_personal(revoked);
        t.write(|tx| {
            User::find(tx.conn(), id("david"))?.deactivate(tx)?;
            assert!(User::find(tx.conn(), id("david"))?.is_deactivated());
            assert!(
                Agent::find(tx.conn(), id("bender_agent"))?
                    .unwrap()
                    .suspended()
            );
            assert!(!crate::sql::exists(
                tx.conn(),
                "SELECT 1 FROM agent_grants WHERE agent_id=? AND revoked_at IS NULL",
                [id("bender_agent")]
            )?);
            Ok(())
        });
    }
}

#[test]
fn review_s1_legacy_grants_cannot_block_owner_ban() {
    for revoked in [false, true] {
        let t = legacy_personal(revoked);
        t.write(|tx| {
            User::find(tx.conn(), id("david"))?.ban(tx)?;
            assert!(User::find(tx.conn(), id("david"))?.is_banned());
            assert!(
                Agent::find(tx.conn(), id("bender_agent"))?
                    .unwrap()
                    .suspended()
            );
            assert!(!crate::sql::exists(
                tx.conn(),
                "SELECT 1 FROM agent_grants WHERE agent_id=? AND revoked_at IS NULL",
                [id("bender_agent")]
            )?);
            Ok(())
        });
    }
}

#[test]
fn review_s1_migration_preserves_legacy_rows_is_additive_and_controls_still_work() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("legacy.sqlite3");
    let seed = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../parity/.seed/default/db/production.sqlite3");
    std::fs::copy(seed, &path).unwrap();
    let mut conn = Connection::open(&path).unwrap();
    conn.execute_batch("DROP TRIGGER personal_agent_grant_insert; DROP TRIGGER personal_agent_grant_update; DROP TRIGGER personal_agent_kind_update; DELETE FROM schema_migrations WHERE version='20261012220000';").unwrap();
    conn.execute(
        "DELETE FROM agent_grants WHERE agent_id=?",
        [id("bender_agent")],
    )
    .unwrap();
    conn.execute(
        "UPDATE agents SET kind='personal' WHERE id=?",
        [id("bender_agent")],
    )
    .unwrap();
    let stamp = crate::Timestamp::from_jiff("2026-03-02T16:00:00Z".parse().unwrap());
    let live = plant_legacy_grants(&conn, stamp, false).unwrap();
    let revoked = plant_legacy_grants(&conn, stamp, true).unwrap();
    let valid: i64 = conn.query_row("INSERT INTO agent_grants(agent_id,room_id,capability,granted_by_id,created_at,updated_at) VALUES (?,?,'read_messages',?,?,?) RETURNING id",params![id("bender_agent"),id("watercooler"),id("david"),stamp,stamp],|row|row.get(0)).unwrap();
    let workspace_bot: i64 = conn.query_row(
        "INSERT INTO users(name,role,created_at,updated_at) VALUES ('Legacy workspace bot',2,?,?) RETURNING id",
        params![stamp,stamp],|row|row.get(0),
    ).unwrap();
    let workspace_agent: i64 = conn.query_row(
        "INSERT INTO agents(user_id,owner_id,kind,created_at,updated_at) VALUES (?,?,'workspace',?,?) RETURNING id",
        params![workspace_bot,id("david"),stamp,stamp],|row|row.get(0),
    ).unwrap();
    let workspace_grant: i64 = conn.query_row(
        "INSERT INTO agent_grants(agent_id,capability,granted_by_id,created_at,updated_at) VALUES (?,'external_action',?,?,?) RETURNING id",
        params![workspace_agent,id("david"),stamp,stamp],|row|row.get(0),
    ).unwrap();
    let jobs = |conn: &Connection| {
        conn.query_row("SELECT COALESCE(json_group_array(json(row)), '[]') FROM (SELECT json_object('id',id,'class',job_class,'arguments',arguments,'run_at',run_at) row FROM background_jobs ORDER BY id)",[],|row|row.get::<_,String>(0)).unwrap()
    };
    let before = jobs(&conn);
    let before_path = dir.path().join("before.sqlite3");
    conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")
        .unwrap();
    std::fs::copy(&path, &before_path).unwrap();
    crate::migrations::migrate_preserving_existing_foreign_key_violations(&mut conn).unwrap();
    conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")
        .unwrap();
    let verified = crate::additive::verify(&before_path, &path).unwrap();
    assert!(verified.contains("MATCH:"), "{verified}");
    assert_eq!(jobs(&conn), before);
    for grant_id in live {
        let grant = AgentGrant::find(&conn, grant_id).unwrap().unwrap();
        assert_eq!(grant.revoked_at, None);
        assert_eq!(grant.updated_at, stamp);
        if ["external_action", "fizzy", "dm_anyone", "legacy_action"]
            .contains(&grant.capability.as_str())
        {
            assert!(
                !crate::models::agent_access::has_capability_anywhere(
                    &conn,
                    grant.agent_id,
                    &grant.capability
                )
                .unwrap()
            );
        } else {
            assert!(
                !crate::models::agent_access::capability_for_agent(
                    &conn,
                    grant.agent_id,
                    &grant.capability,
                    grant.room_id
                )
                .unwrap()
            );
        }
    }
    for grant_id in revoked {
        let grant = AgentGrant::find(&conn, grant_id).unwrap().unwrap();
        assert_eq!(grant.revoked_at, Some(stamp));
        assert_eq!(grant.updated_at, stamp);
    }
    assert!(AgentGrant::find(&conn, valid).unwrap().unwrap().active());
    let untouched = AgentGrant::find(&conn, workspace_grant).unwrap().unwrap();
    assert!(untouched.active());
    assert_eq!(untouched.updated_at, stamp);

    drop(conn);
    let clock = TestClock::new();
    let sink = RecordingSink::new();
    let env = Env {
        clock: Arc::new(clock.clone()),
        sink: Arc::new(sink.clone()),
        rich_text: Arc::new(BasicRichText),
        ..Default::default()
    };
    let mut config = Config::new(path);
    config.environment = "test".into();
    config.readers = 2;
    let t = TestDb {
        db: Database::open(config, env).unwrap(),
        sink,
        clock,
        _dir: dir,
    };
    t.write(|tx| {
        let rows = crate::sql::query_all(
            tx.conn(),
            "SELECT id FROM agent_grants WHERE agent_id=?",
            [id("bender_agent")],
            |row| row.get::<_, i64>(0),
        )?;
        for grant_id in rows {
            AgentGrant::find(tx.conn(), grant_id)?.unwrap().revoke(tx)?;
        }
        agent_lifecycle::kill_switch(tx, id("bender_agent"), &Context::default())?;
        User::find(tx.conn(), id("david"))?.deactivate(tx)?;
        User::find(tx.conn(), id("david"))?.ban(tx)?;
        assert!(
            Agent::find(tx.conn(), id("bender_agent"))?
                .unwrap()
                .suspended()
        );
        Ok(())
    });
}

#[test]
fn review_s1_first_owner_dm_rechecks_suspension_after_authentication() {
    let t = personal();
    let secret = t.write(|tx| {
        let (_, secret) = AgentCredential::create_with_secret(
            tx,
            id("bender_agent"),
            "DM race",
            id("david"),
            None,
        )?;
        assert!(agent_access::authenticate_identity(tx, &secret, "127.0.0.1")?.is_some());
        assert!(Room::find_direct_for(tx.conn(), &[id("bender"), id("david")])?.is_none());
        Ok(secret)
    });
    t.write(|tx| agent_lifecycle::suspend(tx, id("bender_agent"), &Context::default()));
    t.write(move |tx| {
        assert!(AgentCredential::authenticate(tx.conn(), &secret, tx.now())?.is_some());
        let before: i64 = tx
            .conn()
            .query_row("SELECT COUNT(*) FROM messages", [], |row| row.get(0))?;
        let result = agent_direct_messages::open_and_post(
            tx,
            id("bender_agent"),
            id("david"),
            NewMessage {
                markdown_source: Some("Must not be posted".into()),
                ..Default::default()
            },
            DriveInput::default(),
            &Context::default(),
        )?;
        assert!(matches!(result,DirectMessageResult::Denied(denial) if denial.status==403));
        assert!(Room::find_direct_for(tx.conn(), &[id("bender"), id("david")])?.is_none());
        assert_eq!(
            tx.conn()
                .query_row("SELECT COUNT(*) FROM messages", [], |row| row
                    .get::<_, i64>(0))?,
            before
        );
        Ok(())
    });
}

#[test]
fn review_s1_completion_revocation_after_authorized_request_hides_old_events() {
    let t = super::channel_thread_test::frozen();
    t.write(|tx| {
        tx.conn().execute(
            "DELETE FROM agent_grants WHERE agent_id=?",
            [id("bender_agent")],
        )?;
        tx.conn().execute(
            "DELETE FROM agent_events WHERE agent_id=?",
            [id("bender_agent")],
        )?;
        Room::find(tx.conn(), id("watercooler"))?.grant_to(tx, &[id("bender")])?;
        let request = agent_approvals::ApprovalRequest {
            room_id: Some(id("watercooler")),
            action: "deploy".into(),
            summary: "An action started before revocation".into(),
            ..Default::default()
        };
        assert_eq!(
            agent_approvals::create(tx, id("bender_agent"), request.clone(), None)?.status,
            403
        );
        let mut grant = AgentGrant::create(
            tx,
            NewGrant {
                agent_id: id("bender_agent"),
                room_id: Some(id("watercooler")),
                capability: "external_action".into(),
                granted_by_id: id("david"),
                ..Default::default()
            },
        )?;
        let created = agent_approvals::create(tx, id("bender_agent"), request, None)?;
        assert_eq!(created.status, 201);
        let approval_id = created.payload.unwrap()["id"].as_i64().unwrap();
        let event = AgentEvent::create(
            tx,
            NewEvent {
                agent_id: id("bender_agent"),
                agent_approval_id: Some(approval_id),
                room_id: Some(id("watercooler")),
                event_type: "approval_decided".into(),
                outcome: Some("delivered".into()),
                ..Default::default()
            },
        )?;
        assert_eq!(
            agent_event_access::readable_page(tx.conn(), id("bender_agent"), 0, None)?
                .iter()
                .map(|event| event.id)
                .collect::<Vec<_>>(),
            vec![event.id]
        );
        grant.revoke(tx)?;
        assert!(
            agent_event_access::readable_page(tx.conn(), id("bender_agent"), 0, None)?.is_empty()
        );
        assert!(matches!(
            agent_event_access::acknowledge(tx, id("bender_agent"), event.id)?,
            agent_event_access::Acknowledgment::Forbidden
        ));
        assert!(crate::AgentApproval::find(tx.conn(), approval_id)?.is_some());
        Ok(())
    });
}

#[test]
fn review_s1_incompatible_legacy_grants_are_ineffective_in_active_scopes_and_summaries() {
    let t = legacy_personal(false);
    t.write(|tx| {
        let agent = Agent::find(tx.conn(), id("bender_agent"))?.unwrap();
        for capability in crate::models::agent_access::CAPABILITIES {
            assert!(!crate::models::agent_access::has_capability_anywhere(tx.conn(), agent.id, capability)?);
            assert!(!agent.can(tx.conn(), capability, Some(id("watercooler")))?);
        }
        assert!(agent.grant_lines(tx.conn())?.unwrap().is_empty());
        assert!(AgentGrant::all_active(tx.conn())?.iter().all(|grant| grant.agent_id != agent.id));
        let own = crate::Room::find_or_create_direct_for(tx, &[id("bender"), id("david")], id("david"))?;
        tx.conn().execute("INSERT INTO agent_grants(agent_id,room_id,capability,granted_by_id,created_at,updated_at) VALUES (?,?,'react',?,?,?)",params![agent.id,own.id,id("david"),tx.now(),tx.now()])?;
        assert!(agent.can(tx.conn(), "react", Some(own.id))?);
        assert!(agent.grant_lines(tx.conn())?.unwrap().is_empty());
        assert!(AgentGrant::all_active(tx.conn())?.iter().all(|grant| grant.agent_id != agent.id));
        let valid = AgentGrant::create(tx, NewGrant { agent_id: agent.id, room_id: Some(id("watercooler")), capability: "read_messages".into(), granted_by_id: id("david"), ..Default::default() })?;
        let active = AgentGrant::all_active(tx.conn())?.into_iter().filter(|grant| grant.agent_id == agent.id).map(|grant| grant.id).collect::<Vec<_>>();
        assert_eq!(active, [valid.id]);
        let lines = agent.grant_lines(tx.conn())?.unwrap();
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].capability, "read_messages");
        assert!(!lines[0].workspace_wide);
        assert_eq!(lines[0].room_count, 1);
        Ok(())
    });
}
