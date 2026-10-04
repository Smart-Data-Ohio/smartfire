//! Rails wire bytes for room/history/board/work reads through the production router.
use super::agent_http_tests::{AGENT, SECRET, setup};
use super::presenters::test_support::{Req, TestApp};
use campfire_db::{Message, NewMessage};
use campfire_kit::Method;
use serde_json::Value;

pub(super) async fn prepare(case: &Value) -> TestApp {
    let app = setup().await.without_job_runner().await;
    let config = case["setup"].clone();
    app.db().write(move |tx| {
        for cap in config["grant"].as_array().into_iter().flatten() {
            let scope = match cap.as_str() {
                Some("manage_threads") => config.get("manage_room").unwrap_or(&config["grant_room"]),
                Some("read_messages") => config.get("read_room").unwrap_or(&config["grant_room"]),
                _ => &config["grant_room"],
            };
            tx.conn().execute("INSERT INTO agent_grants(agent_id,capability,room_id,granted_by_id,revoked_at,created_at,updated_at) VALUES(?,?,?,127326141,?,?,?)",rusqlite::params![AGENT,cap.as_str(),scope.as_i64(),if config["revoked"]==true {Some(tx.now())} else {None},tx.now(),tx.now()])?;
        }
        tx.conn().execute("UPDATE agents SET owner_id=127326141 WHERE id=?",[AGENT])?;
        if config["bot_key"]==true {
            use sha2::{Digest,Sha256};
            tx.conn().execute("UPDATE users SET bot_token_digest=? WHERE id=394959859",[format!("{:x}",Sha256::digest("BenderToken1"))])?;
        }
        tx.conn().execute("UPDATE sqlite_sequence SET seq=1900700000 WHERE name='messages'",[])?;
        for (creator,source,client) in [(127326141,"Source α & β","read-source"),(394959859,"Agent","read-agent")] {
            Message::create(tx,NewMessage{room_id:486777696,creator_id:creator,markdown_source:Some(source.into()),client_message_id:Some(client.into()),..Default::default()})?;
        }
        tx.conn().execute("INSERT INTO channel_threads(id,name,room_id,creator_id,work_owner_id,work_status,locked_at,last_activity_at,created_at,updated_at) VALUES(1900700020,'Owned α & β',486777696,394959859,?,'in_progress',?,?,?,?)",rusqlite::params![if config["other_owner"]==true {127326141} else {394959859},if config["locked"]==true {Some(tx.now())} else {None},tx.now(),tx.now(),tx.now()])?;
        campfire_db::ThreadTag::create(tx,1900700020,"api")?;
        if config["work_pr"]==true {
            tx.conn().execute("INSERT INTO github_pull_requests(id,owner,repo,number,title,state,head_branch,base_branch,review_decision,check_status,private,created_at,updated_at) VALUES(1900700030,'acme','secret',3,'Secret acquisition α & β','open','secret-branch','main','approved','passing',?,?,?)",rusqlite::params![config["pr_private"].as_bool(),tx.now(),tx.now()])?;
            tx.conn().execute("INSERT INTO work_thread_links(id,channel_thread_id,kind,github_pull_request_id,created_by_id,created_at,updated_at) VALUES(1900700031,1900700020,'pull_request',1900700030,127326141,?,?)",[tx.now(),tx.now()])?;
        }
        Message::create(tx,NewMessage{room_id:486777696,creator_id:394959859,thread_id:Some(1900700020),markdown_source:Some("Thread".into()),client_message_id:Some("read-thread".into()),..Default::default()})?;
        if config["board"]==true {tx.conn().execute("UPDATE rooms SET type='Rooms::Board' WHERE id=486777696",[])?;}
        if config["remove_member"]==true {tx.conn().execute("DELETE FROM memberships WHERE room_id=486777696 AND user_id=394959859",[])?;}
        if config["pin"]==true {
            campfire_db::MessagePin::pin(tx,&Message::find(tx.conn(),1900700001)?,394959859)?.unwrap();
        }
        if config["cap"]==true {
            for i in 0..50 {
                let message=if i==0 {Message::find(tx.conn(),1900700001)?} else {Message::create(tx,NewMessage{room_id:486777696,creator_id:127326141,markdown_source:Some(format!("Cap {i}")),client_message_id:Some(format!("cap-{i}")),..Default::default()})?};
                campfire_db::MessagePin::create(tx,&message,486777696,394959859)?;
            }
        }
        tx.conn().execute("UPDATE agents SET daily_message_cap=? WHERE id=?",rusqlite::params![config["message_cap"].as_i64(),AGENT])?;
        tx.conn().execute("UPDATE sqlite_sequence SET seq=1900800000 WHERE name='polls'",[])?;
        tx.conn().execute("UPDATE sqlite_sequence SET seq=1900810000 WHERE name='poll_options'",[])?;
        if config["poll"]==true {
            let poll=campfire_db::Poll::create_for_message(tx,&Message::find(tx.conn(),1900700001)?,campfire_db::NewPoll{labels:vec!["Small".into(),"Large".into()],multiple:true,anonymous:config["anonymous"]==true,..Default::default()})?;
            if config["votes"]==true {
                let options=poll.options(tx.conn())?;
                for (user,option) in [(394959859,0),(127326141,0),(149087659,1)]{campfire_db::PollVote::create(tx,poll.id,options[option].id,user)?;}
            }
        if config["closed"]==true {tx.conn().execute("UPDATE polls SET closed_at=? WHERE id=?",rusqlite::params![tx.now(),poll.id])?;}
        }
        tx.conn().execute("UPDATE sqlite_sequence SET seq=1900900000 WHERE name='boosts'",[])?;
        if let Some(content)=config["prior"].as_str() {
            campfire_db::Boost::create(tx,1900700001,394959859,content)?;
        }
        if config["polling"]==true {
            tx.conn().execute("DELETE FROM agent_events WHERE agent_id=?",[AGENT])?;
            tx.conn().execute("UPDATE sqlite_sequence SET seq=1900950000 WHERE name='agent_events'",[])?;
            if config["drive"]==true {
                for file in ["1AbcDefGhIjKlMnOpQrSt","2BcdEfgHiJkLmNoPqRsTu"] {
                    tx.conn().execute("INSERT INTO drive_attachments(message_id,file_id,created_at) VALUES(1900700001,?,?)",rusqlite::params![file,tx.now()])?;
                }
            }
            campfire_db::models::agent_delivery::AgentEvent::create(tx,campfire_db::models::agent_delivery::NewEvent {
                agent_id:AGENT, room_id:Some(486777696), message_id:Some(1900700001), actor_id:Some(127326141),
                event_type:"mention".into(), outcome:Some("delivered".into()), metadata:serde_json::json!({"hop":0}), ..Default::default()
            })?;
        }
        if config["work_write"]==true {super::agent_work_writes_tests::fixture(tx,&config)?;}
        Ok(())
    }).await.unwrap();
    app
}
pub(super) fn request(case: &Value) -> Req {
        let mut req = Req::new(
            Method::from_bytes(case["method"].as_str().unwrap().to_uppercase().as_bytes()).unwrap(),
            case["path"].as_str().unwrap(),
        )
        .header("accept", "application/json")
        .header("user-agent", "ws11api-work-contract")
        .header("content-type",case["setup"]["content_type"].as_str().unwrap_or("application/json"))
        ;
        if case["setup"]["work_write"]==true {req=req.header("x-forwarded-for","203.0.113.31");}
        if case["setup"]["bot_key"]!=true {req=req.header("authorization", &["Bearer", SECRET].join(" "));}
        if let Some(body) = case["body"].as_str() {
            req = req.body(body);
        }
        if let Some(filename)=case["setup"]["upload_filename"].as_str() {
            let mime=case["setup"]["upload_mime"].as_str().unwrap();
            let field=if case["setup"]["bot_key"]==true {"attachment"} else {"message[attachment]"};
            let mut body=format!("--ws11api-attachment\r\nContent-Disposition: form-data; name=\"{field}\"; filename=\"{filename}\"\r\nContent-Type: {mime}\r\n\r\n").into_bytes();
            let bytes=case["setup"]["upload_bytes"].as_str().map(str::as_bytes).unwrap_or_else(|| {
                assert_eq!(filename,"moon.jpg");
                include_bytes!("../../../../vectors/users_logos/moon.jpg")
            });
            body.extend_from_slice(bytes);
            body.extend_from_slice(format!("\r\n--ws11api-attachment\r\nContent-Disposition: form-data; name=\"message[client_message_id]\"\r\n\r\nmedia-{}\r\n--ws11api-attachment--\r\n",case["name"].as_str().unwrap()).as_bytes());
            req=req.body(body);
        }
        req
}
pub(super) async fn check(case: &Value) {
    let app=prepare(case).await;
    let request=||request(case);
    for _ in 0..case["setup"]["repeat"].as_u64().unwrap_or(0) {
        let warm = app.anonymous().send(request()).await;
        if case["setup"]["transition"].is_string() {
            assert_eq!(warm.status.as_u16(), case["warm_status"].as_u64().unwrap() as u16);
            assert_eq!(warm.text(), case["warm_body"].as_str().unwrap(), "successful request before permission change");
        }
    }
    let transition = case["setup"]["transition"].as_str().map(str::to_owned);
    if let Some(transition) = transition {
        app.db().write(move |tx| {
            if transition.starts_with("owner_") { tx.conn().execute("UPDATE sessions SET ip_address='203.0.113.31' WHERE user_id=127326141", [])?; }
            match transition.as_str() {
                "grant" => { tx.conn().execute("UPDATE agent_grants SET revoked_at=? WHERE agent_id=?", rusqlite::params![tx.now(),AGENT])?; }
                "membership" => { tx.conn().execute("DELETE FROM memberships WHERE room_id=486777696 AND user_id=394959859", [])?; }
                "credential_revoked" => { tx.conn().execute("UPDATE agent_credentials SET revoked_at=? WHERE agent_id=?", rusqlite::params![tx.now(),AGENT])?; }
                "credential_expired" => { tx.conn().execute("UPDATE agent_credentials SET expires_at='2026-03-02 15:59:59' WHERE agent_id=?", [AGENT])?; }
                "agent_suspended" => { tx.conn().execute("UPDATE agents SET suspended_at=? WHERE id=?",rusqlite::params![tx.now(),AGENT])?; }
                "owner_deactivated" => { let mut owner=campfire_db::User::find(tx.conn(),127326141)?; owner.deactivate(tx)?; }
                "owner_banned" => { let mut owner=campfire_db::User::find(tx.conn(),127326141)?; owner.ban(tx)?; }
                other => panic!("unknown transition {other}"),
            }
            Ok(())
        }).await.unwrap();
    }
    let mut cable=if case["broadcasts"].is_array() {Some(super::agent_reactions_tests::subscribe(&app).await)} else {None};
    let reply = app.anonymous().send(request()).await;
    let name = case["name"].as_str().unwrap();
    assert_eq!(
        reply.status.as_u16(),
        case["status"].as_u64().unwrap() as u16,
        "{name}: {}",
        reply.text()
    );
    assert_eq!(
        reply.text(),
        case["response_body"].as_str().unwrap(),
        "{name}"
    );
    for (key, expected) in case["response_headers"].as_object().unwrap() {
        assert_eq!(reply.header(key), expected.as_str(), "{name}: {key}");
    }
    if let Some(expected)=case.get("state") {
        super::agent_work_writes_tests::assert_state(&app,expected,name).await;
    }
    if let Some(expected)=case.get("attachment_state") {
        let actual=app.db().read(|conn| {
            let count:i64=conn.query_row("SELECT COUNT(*) FROM messages WHERE id>=1900700004",[],|row|row.get(0))?;
            let id:Option<i64>=conn.query_row("SELECT MAX(id) FROM messages WHERE id>=1900700004",[],|row|row.get(0))?;
            let message=id.map(|id|Message::find(conn,id)).transpose()?;
            let blob=if let Some(message)=&message {campfire_storage::Blob::attached(conn,"Message",message.id,"attachment").map_err(|error|campfire_db::Error::Other(error.to_string()))?} else {None};
            Ok(serde_json::json!({"messages":count,"markdown_source":message.and_then(|message|message.markdown_source),"attachment":blob.map(|blob|serde_json::json!({"filename":blob.filename.to_string(),"content_type":blob.content_type,"byte_size":blob.byte_size,"checksum":blob.checksum,"metadata":serde_json::from_str::<Value>(&blob.metadata.encode()).unwrap()}))}))
        }).await.unwrap();
        assert_eq!(&actual,expected,"{name}: persisted attachment state");
    }
    if let Some((_,client))=&mut cable {
        for expected in case["broadcasts"].as_array().unwrap() {
            let frame:Value=serde_json::from_str(&client.next_text().await).unwrap();
            assert_eq!(frame["message"],*expected,"{name}: reaction broadcast bytes");
        }
        client.assert_silent().await;
    }
    eprintln!("WS11-api read wire case {name}: 1 passed; 0 failed");
}
async fn group(prefixes: &[&str]) {
    let vectors: Value =
        serde_json::from_str(include_str!("../../../../vectors/agent_reads_http.json")).unwrap();
    for case in vectors["cases"].as_array().unwrap().iter().filter(|case| {
        prefixes
            .iter()
            .any(|prefix| case["name"].as_str().unwrap().starts_with(prefix))
    }) {
        check(case).await;
    }
}
#[tokio::test]
async fn agent_reads_rooms_bytes() {
    group(&["mcp_list_rooms", "mcp_rooms_"]).await;
}
#[tokio::test]
async fn agent_reads_history_bytes() {
    group(&["mcp_history_"]).await;
}
#[tokio::test]
async fn agent_reads_work_bytes() {
    group(&["work_", "mcp_list_work", "mcp_work_"]).await;
}
#[tokio::test]
async fn agent_reads_board_bytes() {
    group(&["posts_", "mcp_posts_"]).await;
}
