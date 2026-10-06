use super::*;
use crate::models::agent_event_polling::poll;
use crate::{AgentGrant, NewGrant, Room};
use rusqlite::params;
use serde_json::{Value, json};

#[test]
fn ws11_poll_payload_matrix_and_dropped_page_cursor_match_rails() {
    let t = super::channel_thread_test::frozen();
    t.clock
        .travel_to(crate::Timestamp::parse_db("2026-03-02 16:00:00").unwrap());
    t.write(|tx| {
        let gold:Value=serde_json::from_str(include_str!("../../../../vectors/agents_event_polling_contract.json")).unwrap();
        let agent=id("bender_agent");let room=486777696;let actor=127326141;
        tx.conn().execute("DELETE FROM agent_events",[])?;
        tx.conn().execute("DELETE FROM agent_grants",[])?;
        Room::find(tx.conn(),room)?.grant_to(tx,&[id("bender")])?;
        Room::find(tx.conn(),id("designers"))?.grant_to(tx,&[id("bender")])?;
        tx.conn().execute("INSERT INTO agents(id,user_id,owner_id,kind,created_at,updated_at) VALUES (900010002,?,?,'personal',?,?)",params![actor,actor,tx.now(),tx.now()])?;
        for (approval, owner, summary, note, expiry) in [(900010001,agent,"Ship <>&",Some(""),tx.now().since(jiff::SignedDuration::from_hours(1))),(900010002,900010002,"Other",None,tx.now().since(jiff::SignedDuration::from_hours(24)))] {
            tx.conn().execute("INSERT INTO agent_approvals(id,agent_id,action,summary,status,decision_note,expires_at,created_at,updated_at) VALUES (?,?,'release',?,'pending',?,?,?,?)",params![approval,owner,summary,note,expiry,tx.now(),tx.now()])?;
        }
        tx.conn().execute("INSERT INTO channel_threads(id,room_id,creator_id,name,work_status,work_owner_id,last_activity_at,created_at,updated_at) VALUES (900020001,?,?,'Inspect','planned',?,?,?,?)",params![room,actor,id("bender"),tx.now(),tx.now(),tx.now()])?;
        for (event,kind,event_room,event_actor,message,approval,metadata) in [
            (900000001,"reply",Some(room),Some(actor),Some(136976342),None,json!({"hop":2})),
            (900000002,"github_action_completed",None,None,None,None,json!({"action":"github.comment","status":false,"url":null,"message":""})),
            (900000003,"fizzy_action_completed",Some(room),Some(actor),None,None,json!([])),
            (900000004,"approval_decided",None,None,None,None,json!({"approval_id":900010001,"decided_by":"Fallback","note":"Fallback"})),
            (900000005,"approval_decided",None,None,None,Some(900010001),json!({})),
            (900000006,"approval_decided",None,None,None,None,json!({"approval_id":900010002})),
            (900000007,"work_assigned",Some(room),Some(actor),None,None,json!({"thread_id":900020001,"assigned_by":"David"})),
            (900000008,"work_handed_off",Some(room),None,None,None,json!({"thread_id":900020001,"handoff":{"id":9,"summary":"<>&","links":[],"open_questions":[],"sender_name":"Sender","receiver_agent_id":agent,"secret":"omit"}})),
            (900000009,"work_unassigned",Some(room),None,None,None,json!({"thread_id":0,"work_snapshot":{"id":7,"title":"Deleted"}})),
            (900000010,"work_assigned",Some(room),None,None,None,json!({"thread_id":0,"work_snapshot":{"id":7}})),
            (900000011,"work_unassigned",Some(room),None,None,None,json!({"thread_id":0,"work_snapshot":[]})),
            (900000012,"slash_command",Some(room),Some(actor),None,None,json!({"thread_id":900020001,"command":"inspect","arguments":""})),
            (900000013,"slash_command",Some(room),None,None,None,json!([])),
            (900000014,"mention",None,None,Some(136976342),None,json!({})),
            (900000015,"approval_decided",None,None,None,None,json!({"approval_id":0})),
        ] {
            tx.conn().execute("INSERT INTO agent_events(id,agent_id,event_type,outcome,room_id,actor_id,message_id,agent_approval_id,metadata,hop,created_at) VALUES (?,?,?,'delivered',?,?,?,?,?,?,?)",params![event,agent,kind,event_room,event_actor,message,approval,metadata,metadata.get("hop").and_then(Value::as_i64).unwrap_or(0),tx.now()])?;
        }
        let check = |key:&str,since:Value,limit:Value|->crate::Result<()> {
            let mut calls=vec![];
            let mut actual=poll(tx.conn(),agent,Some(&since),Some(&limit),tx.now(),&Default::default(),|message|{calls.push(message.id);Ok(json!({"presented_message_id":message.id}))})?;
            actual["presenter_calls"]=json!(calls);
            assert_eq!(actual,gold["pages"][key],"{key}");
            Ok(())
        };
        for (key,since,limit) in [("all",json!(0),json!(100)),("first",json!("0tail"),json!("1junk")),("dropped",json!(900000013),json!(100)),("empty",json!(900000015),json!(100))] {check(key,since,limit)?;}
        tx.conn().execute("UPDATE agents SET suspended_at=? WHERE id=?",params![tx.now(),agent])?;
        check("suspended",json!(0),json!(100))?;
        tx.conn().execute("UPDATE agents SET suspended_at=NULL WHERE id=?",[agent])?;
        tx.conn().execute("UPDATE rooms SET deleted_at=? WHERE id=?",params![tx.now(),room])?;
        check("soft_deleted_room",json!(0),json!(100))?;
        tx.conn().execute("UPDATE rooms SET deleted_at=NULL WHERE id=?",[room])?;
        AgentGrant::create(tx,NewGrant {agent_id:agent,granted_by_id:actor,capability:"read_messages".into(),room_id:Some(room),revoked_at:Some(tx.now())})?;
        check("revoked",json!(0),json!(100))?;
        AgentGrant::create(tx,NewGrant {agent_id:agent,granted_by_id:actor,capability:"read_messages".into(),room_id:Some(room),revoked_at:None})?;
        tx.conn().execute("INSERT INTO agent_events(id,agent_id,event_type,outcome,room_id,message_id,created_at) VALUES (900000016,?,'reply','delivered',?,136976342,?)",params![agent,id("designers"),tx.now()])?;
        check("event_room_denied",json!(0),json!(100))?;
        Ok(())
    });
}
