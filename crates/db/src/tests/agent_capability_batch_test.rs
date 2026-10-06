use super::*;

#[test]
fn ws11_next3_batched_capabilities_preserve_current_user_policy() {
    use crate::models::agent_access::{self, CAPABILITIES};
    for mode in [
        "legacy",
        "global",
        "room",
        "revoked",
        "suspended",
        "inactive",
        "deleted_room",
        "missing_room",
        "human",
    ] {
        let t = super::channel_thread_test::frozen();
        t.write(move|tx|{
            tx.conn().execute("DELETE FROM agent_grants WHERE agent_id=?",[id("bender_agent")])?;
            match mode {
                "global"|"room"|"revoked"=>{
                    tx.conn().execute("INSERT INTO agent_grants(agent_id,capability,room_id,granted_by_id,revoked_at,created_at,updated_at) VALUES(?,'manage_threads',?,?,?, ?,?)",rusqlite::params![id("bender_agent"),if mode=="room" {Some(id("watercooler"))} else {None},id("david"),if mode=="revoked" {Some(tx.now())} else {None},tx.now(),tx.now()])?;
                }
                "suspended"=>{tx.conn().execute("UPDATE agents SET suspended_at=? WHERE id=?",rusqlite::params![tx.now(),id("bender_agent")])?;}
                "inactive"=>{tx.conn().execute("UPDATE users SET status=1 WHERE id=?",[id("bender")])?;}
                "deleted_room"=>{tx.conn().execute("UPDATE rooms SET deleted_at=? WHERE id=?",rusqlite::params![tx.now(),id("watercooler")])?;}
                _=>{}
            }
            let user=if mode=="human" {id("david")} else {id("bender")};
            let room=if mode=="missing_room" {-10} else {id("watercooler")};
            let facts=agent_access::capabilities_for_user_in_room(tx.conn(),user,room,&CAPABILITIES)?;
            for cap in CAPABILITIES {
                assert_eq!(facts[cap],agent_access::capability_for_user(tx.conn(),user,cap,room)?,"{mode} {cap}: batch retains current-state authorization");
            }
            Ok(())
        });
    }
}
