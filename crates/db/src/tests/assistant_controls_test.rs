use super::*;
use crate::{Agent, AgentChanges, AgentKind, WakePolicy};

#[test]
fn assistants_migration_preserves_existing_wake_behavior_and_defaults_new_personal_agents() {
    let catalog = crate::migrations::catalog();
    let old = crate::schema::generate(&catalog[..catalog.len()-1]).unwrap();
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    conn.execute_batch(&old.schema_sql).unwrap();
    for (user,kind) in [(1,"personal"),(2,"workspace")] {
        conn.execute("INSERT INTO agents(user_id,owner_id,kind,created_at,updated_at) VALUES(?,2,?,'2026-11-09','2026-11-09')",rusqlite::params![user,kind]).unwrap();
    }
    conn.execute_batch(&catalog.last().unwrap().sql).unwrap();
    assert_eq!(conn.query_row("SELECT COUNT(*) FROM agents WHERE wake_policy='anyone' AND paused_at IS NULL AND paused_by_id IS NULL",[],|r|r.get::<_,i64>(0)).unwrap(),2);
    conn.execute("INSERT INTO agents(user_id,owner_id,kind,created_at,updated_at) VALUES(3,2,'personal','2026-11-09','2026-11-09')",[]).unwrap();
    assert_eq!(conn.query_row("SELECT wake_policy FROM agents WHERE user_id=3",[],|r|r.get::<_,WakePolicy>(0)).unwrap(),WakePolicy::Owner);
    assert!(conn.execute("UPDATE agents SET wake_policy='invalid'",[]).is_err());
}

#[test]
fn assistants_workspace_wake_and_suspension_override_pause() {
    let t=super::channel_thread_test::frozen();
    t.write(|tx| {
        let mut agent=Agent::find(tx.conn(),id("bender_agent"))?.unwrap();
        assert_eq!(agent.kind,AgentKind::Workspace);
        agent.update(tx,AgentChanges {wake_policy:Some(WakePolicy::Owner),paused_at:Some(Some(tx.now())),paused_by_id:Some(Some(id("david"))),..Default::default()})?;
        assert_eq!(agent.wake_policy,WakePolicy::Anyone);
        assert!(agent.message_can_wake(id("kevin")));
        assert!(agent.paused());
        assert!(agent.active(tx.conn())?);
        agent.update(tx,AgentChanges {suspended_at:Some(Some(tx.now())),..Default::default()})?;
        assert!(!agent.paused());
        assert!(!agent.active(tx.conn())?);
        Ok(())
    });
}
