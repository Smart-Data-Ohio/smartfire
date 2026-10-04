//! Individually audited original API declarations, with actual requests and committed producers.
use super::agent_http_tests::{AGENT, SECRET};
use super::presenters::test_support::{Req, TestApp};
use campfire_db::models::channel_thread::WorkChanges;
use campfire_db::{
    AgentCredential, ChannelThread, Message, NewChannelThread, NewCredential, NewMessage,
    NewWorkThreadLink, Room, ThreadMembership, User, WorkThreadLink,
};
use campfire_kit::Method;
use campfire_richtext::dom::{Dom, NodeId};
use serde_json::{Value, json};
const ROOM: i64 = 486777696;
const DAVID: i64 = 127326141;
const BOT: i64 = 394959859;

async fn action(
    app: &TestApp,
    step: &Value,
    delivery: Option<&crate::integrations::Next6Delivery>,
) -> Option<Value> {
    if step["action"] == "deliver" {
        return Some(
            delivery
                .expect("recording starts before every delivery producer")
                .deliver(app, step["agent"].as_i64().unwrap())
                .await,
        );
    }
    if step["action"] == "inbox" {
        let reply = app
            .david()
            .write(Req::new(Method::GET, "/activity").header("accept", "text/html"))
            .await;
        let html = reply.text();
        let items=app.db().read(move|conn| {
            let mut q=conn.prepare("SELECT id,source_id,event_type FROM activity_items WHERE user_id=127326141 AND source_type='WorkThreadEvent' AND source_id>1901301000 ORDER BY id")?;
            let v=q.query_map([],|r| {let id:i64=r.get(0)?;Ok(json!({"id":id,"source_id":r.get::<_,i64>(1)?,"event_type":r.get::<_,String>(2)?,"present":html.contains(&format!("id=\"activity_item_{id}\""))}))})?.collect::<std::result::Result<Vec<_>,_>>()?; Ok(v)
        }).await.unwrap();
        return Some(json!({"status":reply.status.as_u16(),"items":items}));
    }
    let step = step.clone();
    let crypto = app.booted.app.ar_encryption.clone();
    app.db().write(move|tx| {
        let id=step["id"].as_i64().unwrap_or(1900700020);
        match step["action"].as_str().unwrap() {
            "post"=> {
                tx.conn().execute("UPDATE sqlite_sequence SET seq=? WHERE name='channel_threads'",[id-1])?;
                let created=ChannelThread::create_board_post(tx,NewChannelThread{room_id:step["room"].as_i64().unwrap_or(ROOM),creator_id:DAVID,name:Some(step["title"].as_str().unwrap().into()),work_status:Some(step["status"].as_str().unwrap().into()),work_owner_id:step["owner"].as_i64(),..Default::default()},step["body"].as_str().map(str::to_owned))?;
                assert_eq!(created.id,id);
            },
            "age"=> {let at=tx.now().since(jiff::SignedDuration::from_hours(-step["hours"].as_i64().unwrap()));tx.conn().execute("UPDATE channel_threads SET last_activity_at=?,updated_at=? WHERE id=?",rusqlite::params![at,at,id])?;},
            "reply"=> {ChannelThread::find(tx.conn(),id)?.post_message(tx,DAVID,NewMessage{markdown_source:Some(step["text"].as_str().unwrap().into()),..Default::default()})?;},
            "assign"=> {
                tx.conn().execute("UPDATE channel_threads SET work_status=NULL,work_owner_id=NULL,name=?,creator_id=? WHERE id=?",rusqlite::params![step["title"].as_str(),DAVID,id])?;
                ThreadMembership::join(tx,id,DAVID)?;
                let human=User::find(tx.conn(),DAVID)?;
                ChannelThread::find(tx.conn(),id)?.update_work(tx,&human,WorkChanges{status:Some(Some("planned".into())),owner_id:Some(json!(BOT))})?;
            },
            "unassign"|"reassign"=> {let human=User::find(tx.conn(),DAVID)?;ChannelThread::find(tx.conn(),id)?.update_work(tx,&human,WorkChanges{owner_id:Some(if step["action"]=="unassign" {Value::Null}else{json!(BOT)}),..Default::default()})?;},
            "links"=> {
                if step["pr"]==true {
                    tx.conn().execute("INSERT INTO github_pull_requests(owner,repo,number,title,state,html_url,private,created_at,updated_at) VALUES('rails','rails',7,'Fix login','open','https://github.com/rails/rails/pull/7',0,?,?)",[tx.now(),tx.now()])?;
                    let pr=tx.conn().last_insert_rowid();
                    WorkThreadLink::create(tx,NewWorkThreadLink{channel_thread_id:id,created_by_id:DAVID,kind:Some("pull_request".into()),github_pull_request_id:Some(pr),..Default::default()})?;
                }
                let event=tx.conn().query_row("SELECT id FROM events WHERE title='Watercooler sync'",[],|r|r.get(0))?;
                WorkThreadLink::create(tx,NewWorkThreadLink{channel_thread_id:id,created_by_id:DAVID,kind:Some("event".into()),event_id:Some(event),..Default::default()})?;
                if step["event_only"]!=true {WorkThreadLink::create(tx,NewWorkThreadLink{channel_thread_id:id,created_by_id:DAVID,kind:Some("drive_file".into()),url:Some("https://drive.google.com/file/d/1AbcDefGhIjKlMnOpQrSt/view".into()),title:step["drive_title"].as_str().map(str::to_owned),..Default::default()})?;}
            },
            "join"=> {Room::find(tx.conn(),step["room"].as_i64().unwrap())?.grant_to(tx,&[BOT])?;},
            "leave"=> {let m=campfire_db::Membership::find_by_room_and_user(tx.conn(),step["room"].as_i64().unwrap(),BOT)?.unwrap();m.destroy(tx)?;},
            "revoke"=> {tx.conn().execute("UPDATE agent_grants SET revoked_at=?,updated_at=? WHERE agent_id=? AND capability=? AND (? IS NULL OR room_id=?)",rusqlite::params![tx.now(),tx.now(),AGENT,step["cap"].as_str(),step["room"].as_i64(),step["room"].as_i64()])?;},
            "read_elsewhere"=> {tx.conn().execute("INSERT INTO agent_grants(agent_id,room_id,capability,granted_by_id,created_at,updated_at) VALUES(773018776,201306877,'read_messages',127326141,?,?)",[tx.now(),tx.now()])?;},
            "hook"=> {
                let aid=step["agent"].as_i64().unwrap_or(AGENT);let secret=crypto.encrypt("ws11-next6-public-signing-material");
                tx.conn().execute("UPDATE webhooks SET url='http://93.184.216.34:8080/hook',signing_secret=? WHERE user_id=(SELECT user_id FROM agents WHERE id=?)",rusqlite::params![secret,aid])?;
                tx.conn().execute("UPDATE agents SET webhook_signing_secret=? WHERE id=?",rusqlite::params![secret,aid])?;
            },
            "mention"=> {for (n,body) in step["bodies"].as_array().unwrap().iter().enumerate() {Message::create(tx,NewMessage{room_id:ROOM,creator_id:DAVID,body:Some(body.as_str().unwrap().into()),client_message_id:Some(format!("work-rate-{}",step["offset"].as_u64().unwrap_or(0)+n as u64)),..Default::default()})?;}},
            other=>panic!("unknown action {other}"),
        }
        Ok(())
    }).await.unwrap();
    None
}
fn nodes(dom: &Dom, root: NodeId, class: &str) -> Vec<NodeId> {
    dom.descendants(root)
        .into_iter()
        .filter(|&n| {
            dom.attr(n, "class")
                .is_some_and(|c| c.split_whitespace().any(|c| c == class))
        })
        .collect()
}
fn text(dom: &Dom, ids: Vec<NodeId>) -> String {
    ids.into_iter()
        .map(|n| {
            dom.text_content(n)
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect::<Vec<_>>()
        .join(" ")
}
fn projection(html: &str, name: &str) -> Value {
    if name == "history" {
        return serde_json::from_str::<Value>(html).unwrap()["thread"]["work_history"].clone();
    }
    let mut dom = Dom::new();
    let root = dom.parse_fragment(html).unwrap();
    match name {
        "human_created" => {
            json!({"text":dom.text_content(root).split_whitespace().collect::<Vec<_>>().join(" ")})
        }
        "note" => json!({"note":dom.text_content(root).contains("Digging into the bug")}),
        "board_row" => {
            let row = dom
                .descendants(root)
                .into_iter()
                .find(|&n| dom.attr(n, "id") == Some("board_row_channel_thread_1901300001"));
            row.map_or(json!({"title":false,"owner":false,"badge":""}),|row|json!({"title":dom.text_content(row).contains("Ship the launch"),"owner":text(&dom,nodes(&dom,row,"board-row__owner")).contains("Bender Bot"),"badge":text(&dom,nodes(&dom,row,"agent-badge"))}))
        }
        "post" => {
            let runs = nodes(&dom, root, "board-post__run")
                .into_iter()
                .flat_map(|n| dom.descendants(n))
                .filter(|&n| dom.local_name(n) == Some("a"))
                .map(|n| {
                    json!([
                        dom.attr(n, "href"),
                        dom.text_content(n)
                            .split_whitespace()
                            .collect::<Vec<_>>()
                            .join(" ")
                    ])
                })
                .collect::<Vec<_>>();
            json!({"brief":text(&dom,nodes(&dom,root,"board-post__messages")).contains("Everything goes out Friday."),"run":runs})
        }
        "result" => {
            json!({"reply":text(&dom,nodes(&dom,root,"board-post__messages")).contains("Halfway there."),"result":text(&dom,nodes(&dom,root,"board-post__result-body")),"history":text(&dom,nodes(&dom,root,"board-post__history")).contains("Bender Bot updated the result")})
        }
        other => panic!("unknown projection {other}"),
    }
}
async fn extra_state(app: &TestApp) -> Value {
    app.db().read(|conn| {
        let rows=|sql:&str|->campfire_db::Result<Vec<Value>> {let mut q=conn.prepare(sql)?;let v=q.query_map([],|r|r.get::<_,String>(0))?.collect::<std::result::Result<Vec<_>,_>>()?;Ok(v.into_iter().map(|v|serde_json::from_str(&v).unwrap()).collect())};
        Ok(json!({
            "threads":rows("SELECT json_object('id',id,'title',name,'room_id',room_id,'owner',work_owner_id,'status',work_status,'result',result_markdown,'count',messages_count,'updated_at',strftime('%Y-%m-%dT%H:%M:%fZ',updated_at),'activity_at',strftime('%Y-%m-%dT%H:%M:%fZ',last_activity_at)) FROM channel_threads WHERE id>=1900700020 ORDER BY id")?,
            "messages":rows("SELECT json_object('id',id,'thread_id',thread_id,'creator_id',creator_id,'markdown',markdown_source,'opener',json(CASE board_post_opener WHEN 1 THEN 'true' ELSE 'false' END)) FROM messages WHERE id>1900700003 ORDER BY id")?,
            "history":rows("SELECT json_object('id',id,'thread_id',channel_thread_id,'note',json_extract(metadata,'$.note')) FROM work_thread_events WHERE id>1901301000 ORDER BY id")?,
            "handoffs":rows("SELECT json_object('id',id,'thread_id',channel_thread_id,'summary',summary) FROM work_handoffs WHERE id>1901302000 ORDER BY id")?,
            "ledger":rows("SELECT json_object('id',id,'status',webhook_status,'attempts',webhook_attempts,'outcome',outcome) FROM agent_events WHERE id>1901303000 ORDER BY id")?,
            "inbox":rows("SELECT json_object('id',id,'user_id',user_id,'source_id',source_id,'event_type',event_type) FROM activity_items WHERE source_type='WorkThreadEvent' AND source_id>1901301000 ORDER BY id")?
        }))
    }).await.unwrap()
}
async fn run(key: &str) {
    let vector: Value =
        serde_json::from_str(include_str!("../../../../vectors/agent_next6_named.json")).unwrap();
    let case = vector["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["key"] == key)
        .unwrap();
    run_case(case).await;
}
async fn run_case(case: &Value) -> Vec<usize> {
    let key = case["key"].as_str().unwrap();
    let mut counts = Vec::new();
    let app = super::agent_reads_tests::prepare(case).await;
    let hidden = case["setup"]["hide_base"] == true;
    let planned = case["setup"]["initial_planned"] == true;
    app.db().write(move|tx| {
        tx.conn().execute("DELETE FROM agent_events WHERE agent_id=773018776",[])?;
        tx.conn().execute("UPDATE channel_threads SET work_owner_id=NULL WHERE id<1900700020 AND work_owner_id=394959859",[])?;
        if planned {tx.conn().execute("UPDATE channel_threads SET work_status='planned' WHERE id=1900700020",[])?;}
        if hidden {tx.conn().execute("UPDATE channel_threads SET work_owner_id=127326141,work_status='done',room_id=201306877 WHERE id=1900700020",[])?;tx.conn().execute("UPDATE messages SET room_id=201306877 WHERE thread_id=1900700020",[])?;}
        AgentCredential::create(tx,NewCredential{agent_id:1901100002,created_by_id:DAVID,name:"Receiver contract".into(),token_digest:{ use sha2::{Digest,Sha256}; format!("{:x}",Sha256::digest("ws11api-receiver-credential")) },token_last_four:"test".into(),..Default::default()})?;
        Ok(())
    }).await.unwrap();
    let delivery = if case["steps"]
        .as_array()
        .unwrap()
        .iter()
        .any(|s| s["action"] == "deliver")
    {
        Some(crate::integrations::Next6Delivery::start(&app).await)
    } else {
        None
    };
    let mut observed = 0;
    for step in case["steps"].as_array().unwrap() {
        if step.get("action").is_some() {
            if let Some(output) = action(&app, step, delivery.as_ref()).await {
                let gold = &case["observations"][observed];
                assert_eq!(
                    output, gold["output"],
                    "{key}: {} producer output",
                    step["action"]
                );
                observed += 1;
            }
            continue;
        }
        let gold = &case["observations"][observed];
        let mut req = Req::new(
            Method::from_bytes(step["method"].as_str().unwrap().to_uppercase().as_bytes()).unwrap(),
            step["path"].as_str().unwrap(),
        )
        .header(
            "accept",
            if step.get("projection").is_some() {
                "text/html"
            } else {
                "application/json"
            },
        )
        .header("content-type", "application/json")
        .header("user-agent", "ws11api-work-contract")
        .header("x-forwarded-for", "203.0.113.31");
        if step["auth"] != "session" {
            req = req.header(
                "authorization",
                &format!(
                    "Bearer {}",
                    if step["auth"] == "receiver" {
                        "ws11api-receiver-credential"
                    } else {
                        SECRET
                    }
                ),
            );
        }
        if let Some(body) = gold["request_body"].as_str() {
            req = req.body(body);
        }
        let log = app.db().capture_queries();
        let reply = if step["auth"] == "session" {
            app.david().write(req).await
        } else {
            app.anonymous().send(req).await
        };
        app.db().stop_capturing_queries();
        let count = log
            .lock()
            .unwrap()
            .iter()
            .filter(|sql| sql.trim_start().to_ascii_uppercase().starts_with("SELECT"))
            .count();
        counts.push(count);
        if case.get("cost_path").is_some() {
            println!(
                "WS11_NEXT6_READS {key} response={observed} Rust={count} Rails={}",
                gold["selects"]
            );
        }
        assert_eq!(
            reply.status.as_u16(),
            gold["status"].as_u64().unwrap() as u16,
            "{key}: response {observed} status"
        );
        if let Some(p) = step["projection"].as_str() {
            assert_eq!(
                projection(&reply.text(), p),
                serde_json::from_str::<Value>(gold["response_body"].as_str().unwrap()).unwrap(),
                "{key}: response {observed} HTML projection"
            );
        } else {
            assert_eq!(
                reply.text(),
                gold["response_body"].as_str().unwrap(),
                "{key}: response {observed} bytes"
            );
        }
        for (name, value) in gold["response_headers"].as_object().unwrap() {
            assert_eq!(
                json!(reply.header(name)),
                *value,
                "{key}: response {observed} header {name}"
            );
        }
        observed += 1;
    }
    assert_eq!(observed, case["observations"].as_array().unwrap().len());
    super::agent_work_writes_tests::assert_state_with_job_facts(
        &app,
        &case["state"],
        key,
        key == "work_message_rate",
    )
    .await;
    assert_eq!(
        extra_state(&app).await,
        case["extra_state"],
        "{key}: all committed targets, notes, acknowledgments and inbox"
    );
    println!("WS11_NEXT6 {key}: {observed} real responses/projections; committed state/jobs");
    counts
}
macro_rules! cases { ($($name:ident => $key:literal),* $(,)?)=>{$(#[tokio::test]async fn $name(){run($key).await;})*}; }
cases! {
    ws11_next6_posts_order => "posts_order",
    ws11_next6_posts_reply => "posts_reply",
    ws11_next6_posts_status => "posts_status",
    ws11_next6_posts_owner => "posts_owner",
    ws11_next6_posts_cap => "posts_cap",
    ws11_next6_posts_legacy => "posts_legacy",
    ws11_next6_posts_left => "posts_left",
    ws11_next6_shared_payload => "shared_payload",
    ws11_next6_work_order => "work_order",
    ws11_next6_work_empty => "work_empty",
    ws11_next6_work_links_show => "work_links_show",
    ws11_next6_work_links_list => "work_links_list",
    ws11_next6_work_note_history => "work_note_history",
    ws11_next6_work_orphaned => "work_orphaned",
    ws11_next6_result_permissions => "result_permissions",
    ws11_next6_result_precedence => "result_precedence",
    ws11_next6_assignment_poll => "assignment_poll",
    ws11_next6_assignment_links => "assignment_links",
    ws11_next6_unassignment_poll => "unassignment_poll",
    ws11_next6_assignment_ack => "assignment_ack",
    ws11_next6_assignment_webhook => "assignment_webhook",
    ws11_next6_assignment_no_read => "assignment_no_read",
    ws11_next6_assignment_revoked => "assignment_revoked",
    ws11_next6_assignment_left => "assignment_left",
    ws11_next6_work_message_rate => "work_message_rate",
    ws11_next6_handoff_poll => "handoff_poll",
    ws11_next6_handoff_ack => "handoff_ack",
    ws11_next6_handoff_webhook => "handoff_webhook",
    ws11_next6_handoff_throttle => "handoff_throttle",
    ws11_next6_handoff_shared_throttle => "handoff_shared_throttle",
    ws11_next6_board_flow => "board_flow",
}

#[tokio::test]
async fn ws11_next6_request_reads_stay_flat_at_two_sizes() {
    let vector: Value =
        serde_json::from_str(include_str!("../../../../vectors/agent_next6_queries.json")).unwrap();
    let mut counts = std::collections::HashMap::new();
    for case in vector["cases"].as_array().unwrap() {
        let measured = run_case(case).await;
        let key = case["cost_path"].as_str().unwrap();
        if let Some(first) = counts.insert(key, measured.clone()) {
            assert_eq!(
                first, measured,
                "{key}: request reads must remain flat at 5/50 rows"
            );
        }
        assert!(
            case["observations"]
                .as_array()
                .unwrap()
                .iter()
                .all(|o| o["cache_hits"] == 0)
        );
    }
}
