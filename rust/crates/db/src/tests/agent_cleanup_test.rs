use super::*;
use crate::Timestamp;
use crate::{
    Agent, AgentApproval, AgentCredential, AgentGrant, CredentialChanges, GrantChanges,
    NewApproval, NewCredential, NewGrant,
};
use rusqlite::params;
use serde_json::{Value, json};
fn gold() -> Value {
    serde_json::from_str(include_str!(
        "../../../../vectors/agents_cleanup_contract.json"
    ))
    .unwrap()
}
fn credential() -> NewCredential {
    NewCredential {
        agent_id: id("bender_agent"),
        created_by_id: id("david"),
        name: "Original".into(),
        token_digest: "ws11-public-cleanup-digest".into(),
        token_last_four: "disp".into(),
        ..Default::default()
    }
}
fn grant() -> NewGrant {
    NewGrant {
        agent_id: id("bender_agent"),
        granted_by_id: id("david"),
        capability: "read_messages".into(),
        ..Default::default()
    }
}
fn errors(error: crate::Error) -> Value {
    let crate::Error::RecordInvalid(errors) = error else {
        panic!("{error:?}")
    };
    let mut result = serde_json::Map::new();
    for (key, value) in errors.0 {
        result
            .entry(key.to_string())
            .or_insert_with(|| json!([]))
            .as_array_mut()
            .unwrap()
            .push(json!(value));
    }
    Value::Object(result)
}
#[test]
fn ws11_cleanup_credential_and_grant_updates_validate_and_preserve_noop_stamps() {
    let t = super::channel_thread_test::frozen();
    t.clock
        .travel_to(Timestamp::parse_db("2026-03-02 16:00:00").unwrap());
    let (mut c, mut g) = t.write(|tx| {
        tx.conn().execute("DELETE FROM agent_credentials", [])?;
        tx.conn().execute("DELETE FROM agent_grants", [])?;
        let mut c = AgentCredential::create(tx, credential())?;
        c.update(
            tx,
            CredentialChanges {
                name: Some("Original".into()),
                ..Default::default()
            },
        )?;
        assert_eq!(
            json!(c.updated_at == c.created_at),
            gold()["results"]["credential_same_stamp"]
        );
        Ok((c, AgentGrant::create(tx, grant())?))
    });
    t.clock.travel(jiff::SignedDuration::from_secs(1));
    t.write(move|tx| {
        c.update(tx,CredentialChanges{name:Some("Renamed".into()),expires_at:Some(Some(tx.now().since(jiff::SignedDuration::from_hours(1)))),..Default::default()})?;
        assert_eq!(json!({"name":c.name,"expires_at":c.expires_at.map(crate::models::agent_payloads::json_time),"revoked_at":c.revoked_at.map(crate::models::agent_payloads::json_time),"updated_at":crate::models::agent_payloads::json_time(c.updated_at)}),gold()["results"]["credential_update"]);
        let invalid=c.update(tx,CredentialChanges{name:Some(" ".into()),..Default::default()}).unwrap_err();
        assert_eq!(errors(invalid),gold()["results"]["credential_invalid"]);
        assert_eq!(AgentCredential::find(tx.conn(),c.id)?.unwrap().name,"Renamed");
        g.update(tx,GrantChanges{capability:Some("post_messages".into()),..Default::default()})?;
        assert_eq!(json!({"capability":g.capability,"room_id":g.room_id,"revoked_at":g.revoked_at.map(crate::models::agent_payloads::json_time),"updated_at":crate::models::agent_payloads::json_time(g.updated_at)}),gold()["results"]["grant_update"]);
        let mut other=AgentGrant::create(tx,NewGrant{capability:"react".into(),..grant()})?;
        assert_eq!(errors(other.update(tx,GrantChanges{capability:Some("post_messages".into()),..Default::default()}).unwrap_err()),gold()["results"]["grant_duplicate"]);
        other.revoke(tx)?;other.update(tx,GrantChanges{capability:Some("post_messages".into()),..Default::default()})?;
        assert_eq!(json!(other.revoked_at.is_some() && other.capability=="post_messages"),gold()["results"]["revoked_update_allowed"]);
        c.destroy(tx)?;g.destroy(tx)?;
        assert_eq!(json!([AgentCredential::find(tx.conn(),c.id)?.is_some(),AgentGrant::find(tx.conn(),g.id)?.is_some()]),gold()["results"]["rows_destroyed"]);
        Ok(())
    });
}
#[test]
fn ws11_cleanup_agent_destroy_runs_declared_dependents_and_preserves_rollback() {
    let t = super::channel_thread_test::frozen();
    let approval=t.write(|tx| {
        AgentCredential::create(tx,credential())?;AgentGrant::create(tx,grant())?;
        tx.conn().execute("INSERT INTO agent_slash_commands(agent_id,room_id,name,description,created_at,updated_at) VALUES (?,?,'cleanup','cleanup',?,?)",params![id("bender_agent"),id("watercooler"),tx.now(),tx.now()])?;
        tx.conn().execute("INSERT INTO agent_events(agent_id,event_type,outcome,created_at) VALUES (?,'github_action_completed','delivered',?)",params![id("bender_agent"),tx.now()])?;
        AgentApproval::create(tx,NewApproval{agent_id:id("bender_agent"),action:"deploy".into(),summary:"Cleanup".into(),..Default::default()})
    });
    assert!(
        t.try_write(|tx| {
            Agent::find(tx.conn(), id("bender_agent"))?
                .unwrap()
                .destroy(tx)?;
            Err::<(), _>(crate::Error::Other("rollback".into()))
        })
        .is_err()
    );
    assert_eq!(t.read(|conn|Ok(conn.query_row("SELECT COUNT(*) FROM agent_credentials WHERE token_digest='ws11-public-cleanup-digest'",[],|r|r.get::<_,i64>(0))?)),1);
    t.write(|tx| {
        Agent::find(tx.conn(), id("bender_agent"))?
            .unwrap()
            .destroy(tx)
    });
    t.read(move|conn| {
        let count=|table:&str|->Result<i64>{Ok(conn.query_row(&format!("SELECT COUNT(*) FROM {table} WHERE agent_id=?"),[id("bender_agent")],|r|r.get(0))?)};
        assert_eq!(json!({"agent":Agent::find(conn,id("bender_agent"))?.is_some(),"credentials":count("agent_credentials")?,"grants":count("agent_grants")?,"commands":count("agent_slash_commands")?,"events":count("agent_events")?,"approvals":count("agent_approvals")?,"inbox":conn.query_row("SELECT COUNT(*) FROM activity_items WHERE source_type='AgentApproval' AND source_id=?",[approval.id],|r|r.get::<_,i64>(0))?}),gold()["results"]["agent_destroy"]);
        Ok(())
    });
}
#[test]
fn ws11_cleanup_backfill_is_ownerless_workspace_one_shot_like_the_migration() {
    let t = super::channel_thread_test::frozen();
    t.write(|tx| {
        tx.conn().execute("DELETE FROM agent_slash_commands",[])?;tx.conn().execute("DELETE FROM agents",[])?;
        Agent::backfill_existing_bots(tx)?;
        let matching:bool=tx.conn().query_row("SELECT (SELECT COUNT(*) FROM agents)=(SELECT COUNT(*) FROM users WHERE role=2) AND NOT EXISTS(SELECT 1 FROM agents a LEFT JOIN users u ON u.id=a.user_id WHERE u.role!=2)",[],|r|r.get(0))?;
        let workspace:bool=tx.conn().query_row("SELECT NOT EXISTS(SELECT 1 FROM agents WHERE kind!='workspace')",[],|r|r.get(0))?;
        let ownerless:bool=tx.conn().query_row("SELECT NOT EXISTS(SELECT 1 FROM agents WHERE owner_id IS NOT NULL)",[],|r|r.get(0))?;
        let statuses=crate::sql::query_all(tx.conn(),"SELECT DISTINCT status FROM agents",[],|r|r.get::<_,String>(0))?;
        assert_eq!(json!({"same_users":matching,"workspace":workspace,"ownerless":ownerless,"statuses":statuses}),gold()["results"]["backfill"]);
        let error=Agent::backfill_existing_bots(tx).unwrap_err();
        assert!(matches!(error,crate::Error::Sqlite(rusqlite::Error::SqliteFailure(error,_)) if error.extended_code==rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE));
        assert_eq!(gold()["results"]["backfill_again"],"RecordNotUnique");
        Ok(())
    });
}
