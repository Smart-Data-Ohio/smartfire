//! Named case ports of pinned test/models/agent_grant_test.rb.
use super::*;
use crate::{AgentGrant, NewGrant};
use jiff::SignedDuration;
use rusqlite::params;
fn grant(room_id: Option<i64>, capability: &str) -> NewGrant {
    NewGrant {
        agent_id: id("bender_agent"),
        granted_by_id: id("david"),
        room_id,
        capability: capability.into(),
        ..Default::default()
    }
}
fn frozen() -> TestDb {
    let t = super::channel_thread_test::frozen();
    t.write(|tx| {
        tx.conn().execute("DELETE FROM agent_grants", [])?;
        Ok(())
    });
    t
}
fn has_error(errors: crate::Errors, field: &str, message: &str) -> bool {
    errors.0.iter().any(|(f, m)| *f == field && m == message)
}
#[test]
fn ws11_grant_case_known_capability_required() {
    let t = frozen();
    t.read(|conn| {
        assert!(has_error(
            AgentGrant::validate(conn, &grant(None, "launch_missiles"), None)?,
            "capability",
            "is not included in the list"
        ));
        Ok(())
    });
}
#[test]
fn ws11_grant_case_all_documented_capabilities_valid() {
    let t = frozen();
    t.read(|conn| {
        for capability in [
            "read_messages",
            "post_messages",
            "react",
            "manage_threads",
            "external_action",
            "fizzy",
        ] {
            AgentGrant::validate(conn, &grant(Some(id("watercooler")), capability), None)?
                .into_result()?;
        }
        AgentGrant::validate(conn, &grant(None, "dm_anyone"), None)?.into_result()
    });
}
#[test]
fn ws11_grant_case_dm_anyone_workspace_only() {
    let t = frozen();
    t.read(|conn| {
        assert!(has_error(
            AgentGrant::validate(conn, &grant(Some(id("watercooler")), "dm_anyone"), None)?,
            "room",
            "must be blank: dm_anyone is granted workspace-wide only"
        ));
        Ok(())
    });
}
#[test]
fn ws11_grant_case_enforced_vocabulary_matches_rails() {
    assert_eq!(
        crate::models::agent_access::CAPABILITIES,
        [
            "read_messages",
            "post_messages",
            "react",
            "manage_threads",
            "external_action",
            "fizzy",
            "dm_anyone"
        ]
    );
}
#[test]
fn ws11_grant_case_optional_room_means_workspace() {
    let t = frozen();
    t.write(|tx| {
        let wide = AgentGrant::create(tx, grant(None, "post_messages"))?;
        assert!(wide.room_id.is_none());
        assert!(wide.workspace_wide());
        assert!(!AgentGrant::create(tx, grant(Some(id("watercooler")), "react"))?.workspace_wide());
        Ok(())
    });
}
#[test]
fn ws11_grant_case_active_duplicate_rejected() {
    let t = frozen();
    t.write(|tx| {
        AgentGrant::create(tx, grant(Some(id("watercooler")), "post_messages"))?;
        assert!(has_error(
            AgentGrant::validate(
                tx.conn(),
                &grant(Some(id("watercooler")), "post_messages"),
                None
            )?,
            "capability",
            "has already been granted"
        ));
        Ok(())
    });
}
fn sql_duplicate(room: Option<i64>) {
    let t = frozen();
    t.write(move|tx| {
        AgentGrant::create(tx,grant(room,"post_messages"))?;
        let error=tx.conn().execute("INSERT INTO agent_grants(agent_id,room_id,capability,granted_by_id,created_at,updated_at) VALUES (?,?,'post_messages',?,?,?)",params![id("bender_agent"),room,id("david"),tx.now(),tx.now()]).unwrap_err();
        assert!(matches!(error,rusqlite::Error::SqliteFailure(error,_) if error.extended_code==rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE));Ok(())
    });
}
#[test]
fn ws11_grant_case_database_rejects_active_workspace_duplicate() {
    sql_duplicate(None);
}
#[test]
fn ws11_grant_case_database_rejects_active_scoped_duplicate() {
    sql_duplicate(Some(id("watercooler")));
}
#[test]
fn ws11_grant_case_other_room_and_workspace_do_not_conflict() {
    let t = frozen();
    t.write(|tx| {
        AgentGrant::create(tx, grant(Some(id("watercooler")), "post_messages"))?;
        for room in [Some(id("designers")), None] {
            AgentGrant::validate(tx.conn(), &grant(room, "post_messages"), None)?.into_result()?;
        }
        Ok(())
    });
}
#[test]
fn ws11_grant_case_regrant_after_revocation() {
    let t = frozen();
    t.write(|tx| {
        AgentGrant::create(tx, grant(Some(id("watercooler")), "post_messages"))?.revoke(tx)?;
        AgentGrant::validate(
            tx.conn(),
            &grant(Some(id("watercooler")), "post_messages"),
            None,
        )?
        .into_result()
    });
}
#[test]
fn ws11_grant_case_revoke_timestamp_idempotent() {
    let t = frozen();
    let mut row = t.write(|tx| AgentGrant::create(tx, grant(None, "post_messages")));
    assert!(row.active());
    assert!(!row.revoked());
    let (stamp, mut row) = t.write(move |tx| {
        row.revoke(tx)?;
        assert!(row.revoked());
        assert!(!row.active());
        Ok((row.revoked_at.unwrap(), row))
    });
    t.clock.travel(SignedDuration::from_secs(1));
    t.write(move |tx| {
        row.revoke(tx)?;
        assert_eq!(row.revoked_at, Some(stamp));
        assert_eq!(
            AgentGrant::find(tx.conn(), row.id)?.unwrap().revoked_at,
            Some(stamp)
        );
        Ok(())
    });
}
#[test]
fn ws11_grant_case_active_and_revoked_scopes() {
    let t = frozen();
    t.write(|tx| {
        let active = AgentGrant::create(tx, grant(None, "post_messages"))?;
        let mut revoked = AgentGrant::create(tx, grant(None, "react"))?;
        revoked.revoke(tx)?;
        assert_eq!(
            AgentGrant::all_active(tx.conn())?
                .into_iter()
                .map(|g| g.id)
                .collect::<Vec<_>>(),
            vec![active.id]
        );
        assert_eq!(
            AgentGrant::all_revoked(tx.conn())?
                .into_iter()
                .map(|g| g.id)
                .collect::<Vec<_>>(),
            vec![revoked.id]
        );
        Ok(())
    });
}
