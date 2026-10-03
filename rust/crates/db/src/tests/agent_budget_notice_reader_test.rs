use super::*;
use crate::{Agent, AgentBudgetNotice, Timestamp};
use serde_json::{Value, json};
#[test]
fn ws11_next3_budget_notice_typed_reader_matches_rails() {
    let gold: Value = serde_json::from_str(include_str!(
        "../../../../vectors/agent_budget_notice_reader.json"
    ))
    .unwrap();
    for kind in ["owner", "inactive_owner", "bot_owner", "ownerless"] {
        let t = super::channel_thread_test::frozen();
        t.clock
            .travel_to(Timestamp::parse_db("2026-03-02 16:00:00").unwrap());
        t.write(move|tx|{
            let owner=match kind {"ownerless"=>None,"bot_owner"=>Some(id("bender")),_=>Some(id("david"))};
            tx.conn().execute("UPDATE agents SET owner_id=?,daily_message_cap=2,daily_board_post_cap=NULL,daily_external_action_cap=7 WHERE id=?",rusqlite::params![owner,id("bender_agent")])?;
            if kind=="inactive_owner" {tx.conn().execute("UPDATE users SET status=1 WHERE id=?",[id("david")])?;}
            for (i,cap) in ["messages","board_posts","external_actions"].into_iter().enumerate() {
                tx.conn().execute("INSERT INTO agent_budget_notices(id,agent_id,cap,day,created_at,updated_at) VALUES(?,?,?,'2026-03-02',?,?)",rusqlite::params![2106900000+i as i64,id("bender_agent"),cap,tx.now(),tx.now()])?;
            }
            Ok(())
        });
        t.read(|conn|{
            assert!(AgentBudgetNotice::find_by_id(conn,-10)?.is_none());
            assert!(AgentBudgetNotice::find(conn,-10).is_err());
            let ids=vec![2106900002,2106900000,2106900001,-10];
            let rows=AgentBudgetNotice::for_ids(conn,&ids)?;
            let agent=Agent::find(conn,id("bender_agent"))?.unwrap();
            let actual=rows.iter().map(|notice|Ok(json!({"id":notice.id,"agent_id":notice.agent_id,"cap":notice.cap,"day":notice.day.to_string(),"cap_label":notice.cap_label(),"budget_limit":notice.budget_limit_for_agent(&agent)?,"recipients":notice.activity_recipient_ids(conn)?,"created_at":notice.created_at.jiff().strftime("%Y-%m-%dT%H:%M:%S%.6fZ").to_string(),"updated_at":notice.updated_at.jiff().strftime("%Y-%m-%dT%H:%M:%S%.6fZ").to_string()}))).collect::<Result<Vec<_>>>()?;
            assert_eq!(json!(actual),gold["results"][kind],"{kind}: typed notice and current audience");
            Ok(())
        });
    }
}
