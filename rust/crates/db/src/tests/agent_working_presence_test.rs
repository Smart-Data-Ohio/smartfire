use super::*;
use crate::models::agent_working_presence as presence;
use crate::{Agent, Timestamp};
use serde_json::{Value, json};

#[test]
fn ws11_presence_service_set_clear_unicode_boundary_and_errors_match_rails() {
    let t = super::channel_thread_test::frozen();
    t.clock
        .travel_to(Timestamp::parse_db("2026-03-02 16:00:00").unwrap());
    t.write(|tx| {
        let vectors:Value=serde_json::from_str(include_str!("../../../../vectors/agents_working_presence_contract.json")).unwrap();
        let boundary="é".repeat(140);let oversized="é".repeat(141);
        for (key,text) in [("set",Some(" Thinking… ")),("clear",Some(" \t\0")),("nil",None),("boundary",Some(boundary.as_str())),("oversized",Some(oversized.as_str()))] {
            let result=presence::set(tx,id("bender_agent"),text)?;
            let agent=Agent::find(tx.conn(),id("bender_agent"))?.unwrap();
            assert_eq!(json!({"status":result.status,"payload":result.payload,"error":result.error,"stored":agent.working_presence,"expires_at":agent.working_presence_expires_at.map(crate::models::agent_payloads::json_time)}),vectors["results"][key],"{key}");
        }
        Ok(())
    });
}

#[test]
fn ws11_presence_service_rollback_and_nonvalidation_error_are_preserved() {
    let t = TestDb::new();
    let before = t
        .read(|conn| Agent::find(conn, id("bender_agent")))
        .unwrap();
    assert!(
        t.try_write(|tx| {
            assert!(presence::set(tx, id("bender_agent"), Some("Running…"))?.is_ok());
            Err::<(), _>(crate::Error::Other("WS11 rollback".into()))
        })
        .is_err()
    );
    t.read(move |conn| {
        let agent = Agent::find(conn, id("bender_agent"))?.unwrap();
        assert_eq!(agent.working_presence, before.working_presence);
        assert_eq!(
            agent.working_presence_expires_at,
            before.working_presence_expires_at
        );
        Ok(())
    });
    assert!(matches!(
        t.try_write(|tx| presence::set(tx, 0, None)),
        Err(crate::Error::RecordNotFound("Agent"))
    ));
}
