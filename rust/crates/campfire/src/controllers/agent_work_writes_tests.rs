//! Committed Rails work-write requests, permission precedence and durable effects.
use super::agent_http_tests::AGENT;
use super::presenters::test_support::TestApp;
use campfire_db::{Agent, AgentKind, NewAgent, Tx, User, Webhook};
use serde_json::{Value, json};

pub(super) fn fixture(tx: &mut Tx<'_>, config: &Value) -> campfire_db::Result<()> {
    tx.conn().execute("UPDATE agents SET daily_board_post_cap=?,suspended_at=? WHERE id=?",rusqlite::params![config["board_cap"].as_i64(),if config["inactive"]==true {Some(tx.now())} else {None},AGENT])?;
    if config["credential_revoked"]==true {tx.conn().execute("UPDATE agent_credentials SET revoked_at=? WHERE agent_id=?",rusqlite::params![tx.now(),AGENT])?;}
    if config["credential_expired"]==true {tx.conn().execute("UPDATE agent_credentials SET expires_at=? WHERE agent_id=?",rusqlite::params![tx.now(),AGENT])?;}
    let user=User::create_bot(tx,"Receiver",None)?;
    tx.conn().execute("UPDATE users SET id=1901100001 WHERE id=?",[user.id])?;
    Webhook::create(tx,1901100001,Some("https://receiver.example.test/hook"))?;
    tx.conn().execute("UPDATE sqlite_sequence SET seq=1901100001 WHERE name='agents'",[])?;
    let receiver=Agent::create(tx,NewAgent{user_id:1901100001,owner_id:Some(127326141),kind:AgentKind::Workspace,..Default::default()})?;
    assert_eq!(receiver.id,1901100002);
    tx.conn().execute("INSERT INTO memberships(room_id,user_id,created_at,updated_at) VALUES(486777696,1901100001,?,?)",[tx.now(),tx.now()])?;
    for cap in ["read_messages","post_messages","manage_threads"] {
        tx.conn().execute("INSERT INTO agent_grants(agent_id,capability,room_id,granted_by_id,created_at,updated_at) VALUES(1901100002,?,486777696,127326141,?,?)",rusqlite::params![cap,tx.now(),tx.now()])?;
    }
    match config["receiver"].as_str() {
        Some("inactive")=>{tx.conn().execute("UPDATE agents SET suspended_at=? WHERE id=1901100002",[tx.now()])?;}
        Some("outside")=>{tx.conn().execute("DELETE FROM memberships WHERE user_id=1901100001",[])?;}
        Some(flag)=>{
            let cap=match flag {"no_post"=>"post_messages","no_manage"=>"manage_threads","no_read"=>"read_messages",_=>panic!("unknown receiver flag")};
            tx.conn().execute("DELETE FROM agent_grants WHERE agent_id=1901100002 AND capability=?",[cap])?;
        }
        None=>{}
    }
    if config["untracked"]==true {tx.conn().execute("UPDATE channel_threads SET work_status=NULL,work_owner_id=NULL WHERE id=1900700020",[])?;}
    if let Some(result)=config["result"].as_str() {tx.conn().execute("UPDATE channel_threads SET result_markdown=? WHERE id=1900700020",[result])?;}
    if config["rule"]==true {
        campfire_db::BoardTagAssignment::create(tx,campfire_db::NewBoardTagAssignment{room_id:486777696,tag:"launch".into(),assignee_id:1901100001,created_by_id:127326141})?;
    }
    for i in 0..config["size"].as_i64().unwrap_or(0) {
        tx.conn().execute("INSERT INTO channel_threads(id,name,room_id,creator_id,work_owner_id,work_status,last_activity_at,created_at,updated_at) VALUES(?, ?,486777696,394959859,394959859,'planned',?,?,?)",rusqlite::params![1901200000+i,format!("Other {i}"),tx.now(),tx.now(),tx.now()])?;
    }
    for (i,table) in ["channel_threads","work_thread_events","work_handoffs","agent_events","audit_logs"].into_iter().enumerate() {
        tx.conn().execute("DELETE FROM sqlite_sequence WHERE name=?",[table])?;
        tx.conn().execute("INSERT INTO sqlite_sequence(name,seq) VALUES(?,?)",rusqlite::params![table,1901300000+i as i64*1000])?;
    }
    tx.conn().execute("DELETE FROM background_jobs",[])?;
    Ok(())
}

fn rows(conn:&campfire_db::Connection,sql:&str)->campfire_db::Result<Vec<Value>> {
    let mut stmt=conn.prepare(sql)?;
    let strings=stmt.query_map([],|r|r.get::<_,String>(0))?.collect::<std::result::Result<Vec<_>,_>>()?;
    Ok(strings.into_iter().map(|s|serde_json::from_str(&s).unwrap()).collect())
}
pub(super) async fn assert_state(app:&TestApp,expected:&Value,name:&str) {
    let actual=app.db().read(|conn| {
        let id=if campfire_db::ChannelThread::find_by_id(conn,1901300001)?.is_some(){1901300001}else{1900700020};
        let thread=campfire_db::ChannelThread::find(conn,id)?;
        let tags=thread.tag_names(conn)?;
        let json_time=|t:campfire_db::Timestamp|format!("{}.{:03}Z",t.jiff().strftime("%Y-%m-%dT%H:%M:%S"),t.subsec_microsecond()/1000);
        let time=|t:Option<campfire_db::Timestamp>|t.map(json_time);
        let messages=rows(conn,&format!("SELECT json_object('id',id,'creator_id',creator_id,'thread_id',thread_id,'markdown',markdown_source,'opener',json(CASE board_post_opener WHEN 1 THEN 'true' ELSE 'false' END)) FROM messages WHERE thread_id={id} ORDER BY id"))?;
        let history=rows(conn,"SELECT json_object('id',id,'thread_id',channel_thread_id,'kind',event_type,'actor',actor_id,'from_owner',from_owner_id,'to_owner',to_owner_id,'from_status',from_status,'to_status',to_status,'metadata',json(metadata)) FROM work_thread_events WHERE id>1901301000 ORDER BY id")?;
        let ledger=rows(conn,"SELECT json_object('id',id,'agent',agent_id,'room_id',room_id,'kind',event_type,'actor',actor_id,'outcome',outcome,'hop',hop,'metadata',json(metadata),'webhook_status',webhook_status) FROM agent_events WHERE id>1901303000 ORDER BY id")?;
        let handoffs=rows(conn,"SELECT json_object('id',id,'thread_id',channel_thread_id,'sender_id',sender_id,'receiver_agent_id',receiver_agent_id,'summary',summary,'links',json(links),'open_questions',json(open_questions)) FROM work_handoffs WHERE id>1901302000 ORDER BY id")?;
        let audit=rows(conn,"SELECT json_object('action',action,'actor_id',actor_id,'actor_label',actor_label,'target_type',target_type,'target_id',target_id,'target_label',target_label,'details',json(details),'ip_address',ip_address,'user_agent',user_agent) FROM audit_logs WHERE id>1901304000 ORDER BY id")?;
        let jobs=rows(conn,"SELECT json_object('class',job_class,'args',json(arguments)) FROM background_jobs ORDER BY id")?;
        Ok(json!({"thread":{"id":id,"title":thread.name,"room_id":thread.room_id,"creator_id":thread.creator_id,"owner":thread.work_owner_id,"work_status":thread.work_status,"tags":tags,"result":thread.result_markdown,"result_updated_at":time(thread.result_updated_at),"result_updated_by":thread.result_updated_by_id,"run_url":thread.run_url,"updated_at":json_time(thread.updated_at),"work_status_changed_at":time(thread.work_status_changed_at)},"messages":messages,"history":history,"ledger":ledger,"handoffs":handoffs,"audit":audit,"jobs":jobs}))
    }).await.unwrap();
    assert_eq!(&actual,expected,"{name}: committed state");
    app.db().read(|conn| {
        let chains=rows(conn,"SELECT json_object('chain',chain_id,'hop',hop) FROM agent_events WHERE id>1901303000 ORDER BY id")?;
        if let Some(first)=chains.first() {
            assert!(uuid::Uuid::parse_str(first["chain"].as_str().unwrap()).is_ok());
            assert!(chains.iter().all(|row|row["chain"]==first["chain"]));
        }
        Ok(())
    }).await.unwrap();
}

async fn group(surface:&str,success_only:bool) {
    let vectors:Value=serde_json::from_str(include_str!("../../../../vectors/agent_work_writes_http.json")).unwrap();
    for case in vectors["cases"].as_array().unwrap().iter().filter(|c|c["surface"]==surface && (!success_only || c["name"]==format!("{surface}_success"))) {
        super::agent_reads_tests::check(case).await;
    }
}
macro_rules! surface_tests {
    ($($name:ident => $surface:literal),* $(,)?)=>{$(
        #[tokio::test] async fn $name(){group($surface,false).await;}
    )*};
}
surface_tests!{
    agent_work_writes_rest_create=>"rest_create",
    agent_work_writes_rest_update=>"rest_update",
    agent_work_writes_rest_result=>"rest_result",
    agent_work_writes_rest_handoff=>"rest_handoff",
    agent_work_writes_mcp_create=>"mcp_create",
    agent_work_writes_mcp_update=>"mcp_update",
    agent_work_writes_mcp_board_update=>"mcp_board_update",
    agent_work_writes_mcp_result=>"mcp_result",
    agent_work_writes_mcp_handoff=>"mcp_handoff",
}
