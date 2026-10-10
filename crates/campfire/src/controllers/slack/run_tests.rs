use super::{Fresh, support::response_session};
use serde_json::{Value, json};
#[tokio::test]
async fn slack_run_http_actions_sessions_csrf_rows_audits_and_jobs_match_rails() {
    use campfire_db::models::slack::{NewConnection, SlackConnection, SlackWorkspace};
    use rusqlite::params;
    let cases: Value =
        serde_json::from_str(include_str!("../../../../../vectors/slack/runs_http.json")).unwrap();
    let selected = std::env::var("WS16_RUN_HTTP_CASE").ok();
    let mut checked = 0;
    for case in cases.as_array().unwrap().iter().filter(|case| !case["method"].as_str().unwrap_or("GET").eq_ignore_ascii_case("get")) {
        if selected
            .as_ref()
            .is_some_and(|name| case["name"].as_str() != Some(name))
        {
            continue;
        }
        checked += 1;
        let f = Fresh::new(case["role"].as_i64().unwrap_or(1), vec![]).await;
        let input = case.clone();
        let crypto = rails_compat::ar_encryption::ArEncryption::new(&f.app.secrets);
        f.app.db.write(move |tx| {
            let w=SlackWorkspace::create(tx,&crypto,"fixture-client","fixture-secret",Some(811))?;
            tx.conn().execute("UPDATE slack_workspaces SET id=851,team_id=? WHERE id=?",params![if input["unconfigured"]==true{None}else{Some("TFIXTURE")},w.id])?;
            if input["unconnected"]!=true {
                let grant=SlackConnection::create(tx,&crypto,NewConnection{workspace_id:851,user_id:811,slack_user_id:"UFIXTURE",access_token:Some("fixture-user-grant"),scopes:None})?;
                tx.conn().execute("UPDATE slack_connections SET id=852,disconnected_reason=? WHERE id=?",params![if input["rejected"]==true{Some("Rejected")}else{None},grant.id])?;
            }
            tx.conn().execute("UPDATE users SET time_zone=? WHERE id=811",[input["zone"].as_str()])?;
            for (id,name,kind,deleted) in [(861,"Existing open","Rooms::Open",false),(862,"Existing closed","Rooms::Closed",false),(863,"Deleted","Rooms::Open",true),(864,"Direct","Rooms::Direct",false)] {
                tx.conn().execute("INSERT INTO rooms(id,name,type,creator_id,deleted_at,created_at,updated_at) VALUES(?,?,?,811,?,?,?)",params![id,name,kind,if deleted{Some(tx.now())}else{None},tx.now(),tx.now()])?;
            }
            let stats=json!({"conversations":(["C1","G2","D3","M4","X5"].map(|id|json!({"id":id,"target":{"action":"create"}})))});
            let options=input["options"].as_object().cloned().map(Value::Object).unwrap_or(json!({}));
            let state=if input["lease"]==true{json!({"step_started_at":"2026-01-01T12:00:00.000000Z","step_lease_token":"fixture-lease"})}else{json!({})};
            tx.conn().execute("INSERT INTO slack_imports(id,slack_workspace_id,slack_connection_id,user_id,kind,mode,status,options,stats,state,started_at,created_at,updated_at) VALUES(853,851,?,?,?,?,?,?,?,?,?,?,?)",params![if input["unconnected"]==true{None}else{Some(852)},input["run_user"].as_i64().unwrap_or(811),input["kind"].as_str().unwrap_or("workspace"),input["mode"].as_str().unwrap_or("import"),input["status"].as_str().unwrap_or("completed"),options.to_string(),stats.to_string(),state.to_string(),campfire_db::Timestamp::from_second(1767268740),tx.now(),tx.now()])?;
            if input["other_active"]==true || input["later"]==true {
                tx.conn().execute("INSERT INTO slack_imports(id,slack_workspace_id,user_id,kind,mode,status,stats,started_at,created_at,updated_at) VALUES(854,851,?,'personal','import',?,?,?,?,?)",params![if input["later"]==true{input["later_user"].as_i64().unwrap_or(811)}else{812},if input["later"]==true{"completed"}else{"running"},stats.to_string(),tx.now(),tx.now(),tx.now()])?;
            }
            tx.conn().execute("DELETE FROM audit_logs",[])?;tx.conn().execute("DELETE FROM background_jobs",[])?;
            Ok(())
        }).await.unwrap();
        let method = case["method"].as_str().unwrap_or("GET");
        let path = case["path"].as_str().unwrap();
        let body = case["body"]
            .as_object()
            .cloned()
            .map(Value::Object)
            .unwrap_or(json!({}));
        let (status, headers, actual) =
            wire_request(&f, method, path, body, case["bad_csrf"] == true).await;
        assert_eq!(actual, case["response_body"].as_str().unwrap(), "{} legacy write response", case["name"]);
        assert_eq!(
            status,
            case["status_code"].as_u64().unwrap() as u16,
            "{} status",
            case["name"]
        );
        assert_eq!(
            headers.get("location").map(|h| h.to_str().unwrap()),
            case["location"].as_str(),
            "{} location",
            case["name"]
        );
        let values = response_session(&f, &headers);
        let flash = values["flash"]["flashes"]
            .as_object()
            .cloned()
            .unwrap_or_default();
        assert_eq!(json!(flash), case["flash"], "{} flash", case["name"]);
        let rows=f.app.db.read(|conn| {
            let columns=["id","slack_workspace_id","slack_connection_id","user_id","kind","mode","status","options","state","stats","error","started_at","heartbeat_at","finished_at","created_at","updated_at"];
            let mut stmt=conn.prepare(&format!("SELECT {} FROM slack_imports ORDER BY id",columns.join(",")))?;
            let rows=stmt.query_map([],|r| {
                let mut obj=serde_json::Map::new();
                for (i,col) in columns.iter().enumerate() {
                    let value=match r.get_ref(i)? {
                        rusqlite::types::ValueRef::Null=>Value::Null,
                        rusqlite::types::ValueRef::Integer(n)=>json!(n),
                        rusqlite::types::ValueRef::Text(bytes)=>{
                            let s=std::str::from_utf8(bytes).unwrap();
                            if ["options","state","stats"].contains(col){serde_json::from_str(s).unwrap()}
                            else if col.ends_with("_at"){json!(campfire_db::Timestamp::parse_db(s).unwrap().jiff().strftime("%Y-%m-%d %H:%M:%S UTC").to_string())}
                            else{json!(s)}
                        },_=>panic!("unexpected column"),
                    };obj.insert(col.to_string(),value);
                }Ok(Value::Object(obj))
            })?.collect::<rusqlite::Result<Vec<_>>>()?;
            let audit=conn.prepare("SELECT action,actor_id,target_type,target_id,target_label,details FROM audit_logs ORDER BY id")?.query_map([],|r|Ok(json!({"action":r.get::<_,String>(0)?,"actor_id":r.get::<_,Option<i64>>(1)?,"target_type":r.get::<_,Option<String>>(2)?,"target_id":r.get::<_,Option<i64>>(3)?,"target_label":r.get::<_,Option<String>>(4)?,"details":serde_json::from_str::<Value>(&r.get::<_,String>(5)?).unwrap()})))?.collect::<rusqlite::Result<Vec<_>>>()?;
            let jobs=conn.prepare("SELECT job_class,arguments,queue_name FROM background_jobs ORDER BY id")?.query_map([],|r| {let args:Value=serde_json::from_str(&r.get::<_,String>(1)?).unwrap();Ok(json!({"class":r.get::<_,String>(0)?,"arguments":[args["import_id"].clone()],"queue":r.get::<_,String>(2)?}))})?.collect::<rusqlite::Result<Vec<_>>>()?;
            Ok((rows,audit,jobs))
        }).await.unwrap();
        assert_eq!(
            json!(rows.0),
            case["runs"],
            "{} persisted rows",
            case["name"]
        );
        assert_eq!(json!(rows.1), case["audit"], "{} audits", case["name"]);
        assert_eq!(json!(rows.2), case["jobs"], "{} queued jobs", case["name"]);
        assert!(
            f.server.received().is_empty(),
            "controllers made a Slack request"
        );
    }
    println!(
        "Slack run HTTP parity: {} Rails action cases matched complete response bodies, sessions, CSRF, redirects, flashes, rows, audits and durable jobs",
        checked
    );
    assert!(checked > 0, "no selected Rails HTTP cases");
}

/// Real TCP HTTP, through the actual session/forgery middleware and routed controller.
async fn wire_request(
    f: &Fresh,
    method: &str,
    path: &str,
    body: Value,
    bad_csrf: bool,
) -> (u16, axum::http::HeaderMap, String) {
    wire_request_with_session(f, method, path, body, bad_csrf, json!({})).await
}
async fn wire_request_with_session(
    f: &Fresh,
    method: &str,
    path: &str,
    body: Value,
    bad_csrf: bool,
    mut values: Value,
) -> (u16, axum::http::HeaderMap, String) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = {
        let _binding = crate::test_support::LISTENER_BINDING.lock().await;
        let mut listener = None;
        for port in 53300..=53399 {
            if let Ok(bound) =
                tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).await
            {
                listener = Some(bound);
                break;
            }
        }
        listener.expect("free WS16 HTTP port")
    };
    let address = listener.local_addr().unwrap();
    let router = f.router.clone().layer(axum::middleware::from_fn(
        |request: axum::extract::Request, next: axum::middleware::Next| async move {
            crate::controllers::presenters::test_support::with_fixed_render_secrets(
                next.run(request),
            )
            .await
        },
    ));
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let raw = base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, [7u8; 32]);
    values["_csrf_token"] = json!(raw);
    let cookie = super::support::session(f, &values);
    let csrf = if bad_csrf {
        "invalid".into()
    } else {
        campfire_kit::csrf::mask(&[7u8; 32], [9u8; 32])
    };
    let payload = if method == "GET" {
        String::new()
    } else {
        body.to_string()
    };
    let referrer = if method == "PATCH" && path == "/account/slack_import" {
        "Referer: http://example.org/account/slack_import\r\n"
    } else {
        ""
    };
    let mut stream = tokio::net::TcpStream::connect(address).await.unwrap();
    stream.write_all(format!("{method} {path} HTTP/1.1\r\nHost: example.org\r\n{referrer}Cookie: {cookie}\r\nX-CSRF-Token: {csrf}\r\nContent-Type: application/json\r\nAccept: text/html\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{payload}",payload.len()).as_bytes()).await.unwrap();
    let mut response = vec![];
    stream.read_to_end(&mut response).await.unwrap();
    server.abort();
    let _ = server.await;
    let response = String::from_utf8(response).unwrap();
    let (head, body) = response.split_once("\r\n\r\n").unwrap();
    let mut lines = head.split("\r\n");
    let status = lines
        .next()
        .unwrap()
        .split_whitespace()
        .nth(1)
        .unwrap()
        .parse()
        .unwrap();
    let mut headers = axum::http::HeaderMap::new();
    for line in lines {
        let (name, value) = line.split_once(':').unwrap();
        headers.append(
            axum::http::HeaderName::from_bytes(name.as_bytes()).unwrap(),
            value.trim().parse().unwrap(),
        );
    }
    let body = if headers
        .get("transfer-encoding")
        .is_some_and(|v| v == "chunked")
    {
        let mut rest = body;
        let mut output = String::new();
        loop {
            let (size, next) = rest.split_once("\r\n").unwrap();
            let n = usize::from_str_radix(size.split(';').next().unwrap(), 16).unwrap();
            if n == 0 {
                break;
            }
            output.push_str(&next[..n]);
            rest = &next[n + 2..];
        }
        output
    } else {
        body.into()
    };
    (status, headers, body)
}

#[tokio::test]
async fn slack_personal_opt_in_oauth_preview_import_and_undo_over_real_http() {
    let mut routes = crate::integrations::slack::store::tests::routes(true);
    let scopes = crate::integrations::slack::oauth::USER_SCOPES.join(",");
    routes.push(crate::integrations::test_support::Route::new("POST","slack.com","/api/oauth.v2.access",200)
        .body(json!({"ok":true,"team":{"id":"TFIXTURE","name":"Fixture"},"authed_user":{"id":"UADMIN","access_token":"fixture-user-grant","scope":scopes}}).to_string()));
    let f = Fresh::new(0, routes).await;
    f.workspace().await;
    f.app
        .db
        .write(|tx| {
            tx.conn()
                .execute("UPDATE slack_workspaces SET team_id='TFIXTURE'", [])?;
            let digest =
                campfire_db::models::user::password_digest("fixture-personal-password", 4)?;
            tx.conn()
                .execute("UPDATE users SET password_digest=? WHERE id=811", [digest])?;
            Ok(())
        })
        .await
        .unwrap();
    let (code, _, _) = wire_request(&f, "GET", "/slack/imports", Value::Null, false).await;
    assert_eq!(code, 302);
assert!(f.server.received().is_empty());
    let (code, headers, _) = wire_request(
        &f,
        "GET",
        "/slack/oauth/start?return_to=/slack/imports",
        Value::Null,
        false,
    )
    .await;
    assert_eq!(code, 302);
    assert_eq!(headers["location"], "http://example.org/sudo/new");
    let (code, headers, _) = wire_request_with_session(
        &f,
        "POST",
        "/sudo",
        json!({"password":"fixture-personal-password"}),
        false,
        response_session(&f, &headers),
    )
    .await;
    assert_eq!(code, 302);
    let (code, headers, _) = wire_request_with_session(
        &f,
        "GET",
        "/slack/oauth/start?return_to=/slack/imports",
        Value::Null,
        false,
        response_session(&f, &headers),
    )
    .await;
    assert_eq!(code, 302);
    let location = url::Url::parse(headers["location"].to_str().unwrap()).unwrap();
    assert_eq!(location.host_str(), Some("slack.com"));
    let signed = location
        .query_pairs()
        .find(|(key, _)| key == "state")
        .unwrap()
        .1
        .into_owned();
    let mut query = url::form_urlencoded::Serializer::new(String::new());
    query
        .append_pair("state", &signed)
        .append_pair("code", "fixture-personal-code");
    let (code, headers, _) = wire_request_with_session(
        &f,
        "GET",
        &format!("/slack/oauth/callback?{}", query.finish()),
        Value::Null,
        false,
        response_session(&f, &headers),
    )
    .await;
    assert_eq!(code, 302);
    assert_eq!(headers["location"], "http://example.org/app/settings/slack");
    let (_, _, _) = wire_request(&f, "GET", "/slack/imports", Value::Null, false).await;
let (code, headers, _) = wire_request(
        &f,
        "POST",
        "/slack/imports",
        json!({"mode":"dry_run"}),
        false,
    )
    .await;
    assert_eq!(code, 302);
    let preview = headers["location"]
        .to_str()
        .unwrap()
        .rsplit('/')
        .next()
        .unwrap()
        .parse::<i64>()
        .unwrap();
    let crypto = std::sync::Arc::new(rails_compat::ar_encryption::ArEncryption::new(
        &f.app.secrets,
    ));
    crate::integrations::slack::store::tests::import(
        &f.app.db,
        crypto.clone(),
        preview,
        f.app.slack_network.clone(),
    )
    .await;
    let (_, _, _) = wire_request(
        &f,
        "GET",
        &format!("/slack/imports/{preview}"),
        Value::Null,
        false,
    )
    .await;
let (code,headers,_)=wire_request(&f,"POST","/slack/imports",json!({"mode":"import","dry_run_id":preview,"conversation_ids":["CPRIV","GMPIM","DIM"],"preset":"full"}),false).await;
    assert_eq!(code, 302);
    let id = headers["location"]
        .to_str()
        .unwrap()
        .rsplit('/')
        .next()
        .unwrap()
        .parse::<i64>()
        .unwrap();
    let row = crate::integrations::slack::store::tests::import(
        &f.app.db,
        crypto,
        id,
        f.app.slack_network.clone(),
    )
    .await;
    assert_eq!(row.kind, "personal");
    assert!(row.stats["counts"]["messages"].as_i64().unwrap() > 0);
    let (code, _, _) = wire_request(
        &f,
        "GET",
        &format!("/slack/imports/{id}/status"),
        Value::Null,
        false,
    )
    .await;
    assert_eq!(code, 302);
let (code, _, _) = wire_request(
        &f,
        "POST",
        &format!("/slack/imports/{id}/undo"),
        json!({}),
        true,
    )
    .await;
    assert_eq!(code, 422);
    let (code, _, _) = wire_request(
        &f,
        "POST",
        &format!("/slack/imports/{id}/undo"),
        json!({}),
        false,
    )
    .await;
    assert_eq!(code, 302);
    for _ in 0..30 {
        let db = f.app.db.clone();
        crate::integrations::slack::jobs::perform_undo(
            f.app.db.clone(),
            id,
            move |_, lease| async move {
                crate::integrations::slack::undoer::Undoer { db, id, lease }
                    .step()
                    .await
            },
        )
        .await
        .unwrap();
        if crate::integrations::slack::jobs::tests::run(&f.app.db, id)
            .await
            .status
            == "undone"
        {
            break;
        }
    }
    let (_, _, _) = wire_request(
        &f,
        "GET",
        &format!("/slack/imports/{id}"),
        Value::Null,
        false,
    )
    .await;
f.app
        .db
        .read(move |c| {
            assert_eq!(
                c.query_row(
                    "SELECT COUNT(*) FROM slack_import_records WHERE slack_import_id=?",
                    [id],
                    |r| r.get::<_, i64>(0)
                )?,
                0
            );
            assert!(campfire_db::User::find(c, 811)?.password_digest.is_some());
            Ok(())
        })
        .await
        .unwrap();
    println!(
        "Slack personal HTTP interaction: opt-in -> sudo -> OAuth -> preview -> import -> CSRF-protected undo matched"
    );
}

#[tokio::test]
async fn slack_admin_credentials_preview_plan_import_progress_and_undo_over_http() {
    use campfire_db::models::slack::{NewConnection, SlackConnection, SlackWorkspace};
    let f = Fresh::new(1, vec![]).await;
    f.app
        .db
        .write(|tx| {
            let digest = campfire_db::models::user::password_digest("fixture-password", 4)?;
            tx.conn()
                .execute("UPDATE users SET password_digest=? WHERE id=811", [digest])?;
            Ok(())
        })
        .await
        .unwrap();
    let (code, headers, _) = wire_request(
        &f,
        "PATCH",
        "/account/slack_import",
        json!({"client_id":"fixture-client","client_secret":"fixture-secret"}),
        false,
    )
    .await;
    assert_eq!(code, 302);
    assert_eq!(headers["location"], "http://example.org/sudo/new");
    let values = response_session(&f, &headers);
    let (code, headers, _) = wire_request_with_session(
        &f,
        "POST",
        "/sudo",
        json!({"password":"fixture-password"}),
        false,
        values,
    )
    .await;
    assert_eq!(code, 302);
    assert_eq!(
        headers["location"],
        "http://example.org/account/slack_import"
    );
    let values = response_session(&f, &headers);
    assert!(values["sudo_verified_at"].is_number());
    let (code, headers, _) = wire_request_with_session(
        &f,
        "PATCH",
        "/account/slack_import",
        json!({"client_id":"fixture-client","client_secret":"fixture-secret"}),
        false,
        values,
    )
    .await;
    assert_eq!(code, 302);
    let values = response_session(&f, &headers);
    let crypto = rails_compat::ar_encryption::ArEncryption::new(&f.app.secrets);
    f.app
        .db
        .write(move |tx| {
            let workspace = SlackWorkspace::current(tx.conn())?.unwrap();
            tx.conn().execute(
                "UPDATE slack_workspaces SET team_id='TFIXTURE' WHERE id=?",
                [workspace.id],
            )?;
            SlackConnection::create(
                tx,
                &crypto,
                NewConnection {
                    workspace_id: workspace.id,
                    user_id: 811,
                    slack_user_id: "UADMIN",
                    access_token: Some("fixture-user-grant"),
                    scopes: None,
                },
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let (code, headers, _) = wire_request_with_session(
        &f,
        "POST",
        "/account/slack_import/runs",
        json!({}),
        false,
        values,
    )
    .await;
    assert_eq!(code, 302);
    let values = response_session(&f, &headers);
    let preview_path = headers["location"]
        .to_str()
        .unwrap()
        .strip_prefix("http://example.org")
        .unwrap()
        .to_owned();
    let preview = preview_path
        .rsplit('/')
        .next()
        .unwrap()
        .parse::<i64>()
        .unwrap();
    let (_, _, _) =
        wire_request_with_session(&f, "GET", &preview_path, Value::Null, false, values.clone())
            .await;
f.app.db.write(move |tx|{
        let stats=json!({"conversations":[{"id":"C111","name":"general","type":"public_channel","target":{"action":"create"}},{"id":"C222","name":"random","type":"private_channel","target":{"action":"create"}}],"samples":[{"conversation":"general","slack_text":"<&>","markdown":"**HTTP preview** <script>alert(1)</script>"}]});
        tx.conn().execute("UPDATE slack_imports SET status='completed',stats=? WHERE id=?",rusqlite::params![stats.to_string(),preview])?;Ok(())
    }).await.unwrap();
    let (_, _, _) = wire_request(
        &f,
        "GET",
        &format!("{preview_path}/plan"),
        Value::Null,
        false,
    )
    .await;
let (code, headers, _) = wire_request(
        &f,
        "POST",
        &format!("{preview_path}/import"),
        json!({"conversation_ids":["C111","C222"],"preset":"test"}),
        false,
    )
    .await;
    assert_eq!(code, 302);
    let import_path = headers["location"]
        .to_str()
        .unwrap()
        .strip_prefix("http://example.org")
        .unwrap()
        .to_owned();
    let id = import_path
        .rsplit('/')
        .next()
        .unwrap()
        .parse::<i64>()
        .unwrap();
    f.app.db.write(move |tx|{tx.conn().execute("UPDATE slack_imports SET status='running',stats='{\"phase\":\"messages\",\"current\":\"random\"}' WHERE id=?",[id])?;Ok(())}).await.unwrap();
    let (code, _, _) = wire_request(
        &f,
        "GET",
        &format!("{import_path}/status"),
        Value::Null,
        false,
    )
    .await;
    assert_eq!(code, 302);
f.app
        .db
        .write(move |tx| {
            tx.conn().execute(
                "UPDATE slack_imports SET status='completed' WHERE id=?",
                [id],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let (_, _, _) = wire_request(
        &f,
        "GET",
        &format!("{import_path}/status"),
        Value::Null,
        false,
    )
    .await;
let (_, _, _) = wire_request(&f, "GET", &import_path, Value::Null, false).await;
let (code, _, _) =
        wire_request(&f, "POST", &format!("{import_path}/undo"), json!({}), false).await;
    assert_eq!(code, 302);
    let run = f
        .app
        .db
        .read(move |conn| campfire_db::models::slack_import::SlackImport::find(conn, id))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(run.status, "undoing");
    assert!(f.server.received().is_empty());
}

#[tokio::test]
async fn slack_personal_show_reads_later_stats_once_for_both_undo_controls() {
    use rusqlite::trace::{TraceEvent, TraceEventCodes};
    use std::cell::Cell;
    thread_local! {static SCANS:Cell<usize>=const {Cell::new(0)};}
    fn trace(event: TraceEvent<'_>) {
        if let TraceEvent::Stmt(_, sql) = event
            && sql.contains("slack_imports")
            && sql.contains("ORDER BY started_at DESC, id DESC")
        {
            SCANS.with(|count| count.set(count.get() + 1));
        }
    }
    let f = Fresh::new(0, vec![]).await;
    f.workspace().await;
    f.app.db.write(|tx| {
        let workspace=campfire_db::models::slack::SlackWorkspace::current(tx.conn())?.unwrap();
        for id in 853..=856 {
            let stats=json!({"conversations":[{"id":format!("D{id}"),"target":{"action":"create"}}]});
            let at=if id==853 {tx.now().ago(jiff::SignedDuration::from_secs(7200))} else {tx.now().ago(jiff::SignedDuration::from_secs(3600))};
            tx.conn().execute("INSERT INTO slack_imports(id,slack_workspace_id,user_id,kind,mode,status,stats,started_at,created_at,updated_at) VALUES(?, ?,811,'personal','import','completed',?,?,?,?)",rusqlite::params![id,workspace.id,stats.to_string(),at,at,at])?;
        }
        Ok(())
    }).await.unwrap();
    let (data, count) = f
        .app
        .db
        .read(|conn| {
            let run = campfire_db::models::slack_import::SlackImport::find(conn, 853)?.unwrap();
            conn.trace_v2(TraceEventCodes::SQLITE_TRACE_STMT, Some(trace));
            SCANS.with(|count| count.set(0));
            let result = super::super::runs::data(
                conn,
                run,
                campfire_db::Timestamp::parse_db("2026-01-01 12:00:00").unwrap(),
            );
            conn.trace_v2(TraceEventCodes::empty(), None);
            result.map(|data| (data, SCANS.with(Cell::get)))
        })
        .await
        .unwrap();
    assert_eq!(count, 1);
    assert!(data.undo_reason.is_none());
}
