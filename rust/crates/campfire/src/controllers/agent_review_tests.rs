//! PR192 regressions exercise the production router, media processor and reader pool.
use super::agent_http_tests::{AGENT, SECRET, setup};
use super::presenters::test_support::{Reply, Req, TestApp};
use campfire_kit::Method;
use serde_json::{Value, json};

const ROOM: i64 = 486777696;
const THREAD: i64 = 1996000000;
const BOT: i64 = 394959859;
const DAVID: i64 = 127326141;

fn request(method: Method, path: &str, body: Option<Value>) -> Req {
    let mut req = Req::new(method, path)
        .header("accept", "application/json")
        .header("content-type", "application/json")
        .header("authorization", &["Bearer", SECRET].join(" "));
    if let Some(body) = body {
        req = req.body(body.to_string());
    }
    req
}
fn tool(name: &str, arguments: Value) -> Value {
    json!({"jsonrpc":"2.0","id":192,"method":"tools/call","params":{"name":name,"arguments":arguments}})
}
async fn readers() -> TestApp {
    let app = setup().await.without_job_runner().await;
    app.db().write(|tx| {
        tx.conn().execute("INSERT INTO channel_threads(id,name,room_id,creator_id,work_owner_id,work_status,last_activity_at,created_at,updated_at,messages_count) VALUES(?,'Query probe',?,?,?,'in_progress',?,?,?,50)",rusqlite::params![THREAD,ROOM,DAVID,BOT,tx.now(),tx.now(),tx.now()])?;
        for n in 0..50_i64 {
            for (id, thread) in [(1996001000+n,None),(1996002000+n,Some(THREAD))] {
                tx.conn().execute("INSERT INTO messages(id,room_id,creator_id,thread_id,client_message_id,markdown_source,created_at,updated_at) VALUES(?,?,?,?,?,'Query probe',?,?)",rusqlite::params![id,ROOM,DAVID,thread,format!("pr192-read-{id}"),tx.now(),tx.now()])?;
                tx.conn().execute("INSERT INTO action_text_rich_texts(name,record_type,record_id,body,created_at,updated_at) VALUES('body','Message',?,'<p>Query probe</p>',?,?)",rusqlite::params![id,tx.now(),tx.now()])?;
            }
        }
        Ok(())
    }).await.unwrap();
    app
}
async fn measure(
    app: &TestApp,
    label: &str,
    size: usize,
    method: Method,
    path: &str,
    body: Option<Value>,
) -> usize {
    let warm = app
        .anonymous()
        .send(request(method.clone(), path, body.clone()))
        .await;
    assert_eq!(warm.status.as_u16(), 200, "{label}: {}", warm.text());
    let queries = app.db().capture_read_queries();
    let reply = app.anonymous().send(request(method, path, body)).await;
    app.db().stop_capturing_read_queries();
    assert_eq!(reply.status.as_u16(), 200, "{label}: {}", reply.text());
    assert_eq!(
        reply.body, warm.body,
        "{label}: request-scoped preload changed response bytes"
    );
    let wire = reply.json();
    assert!(wire.get("error").is_none(), "{label}: {wire}");
    assert_ne!(wire["result"]["isError"], json!(true));
    let payload = wire
        .get("result")
        .map(|r| &r["structuredContent"])
        .unwrap_or(&wire);
    let returned = payload
        .as_array()
        .or_else(|| payload["messages"].as_array())
        .unwrap()
        .len();
    assert_eq!(returned, size, "{label}: page size");
    let count = queries
        .lock()
        .unwrap()
        .iter()
        .filter(|sql| sql.trim_start().to_ascii_uppercase().starts_with("SELECT"))
        .count();
    println!("WS11-api review reader: {label}; size={size}; returned={returned}; SELECTs={count}");
    count
}
async fn history_queries(context: bool, thread: bool) {
    let app = readers().await;
    let mut counts = Vec::new();
    for size in [5, 50] {
        let label = if context {
            "context_thread"
        } else if thread {
            "history_thread"
        } else {
            "history_root"
        };
        let count = if context {
            measure(
                &app,
                label,
                size,
                Method::GET,
                &format!("/agents/context?thread_id={THREAD}&limit={size}"),
                None,
            )
            .await
        } else {
            let key = if thread { "thread_id" } else { "room_id" };
            let id = if thread { THREAD } else { ROOM };
            measure(
                &app,
                label,
                size,
                Method::POST,
                "/agents/mcp",
                Some(tool("read_messages", json!({key:id,"limit":size}))),
            )
            .await
        };
        counts.push(count);
    }
    assert_eq!(
        counts[0], counts[1],
        "reader SELECT count must stay flat from 5 to 50 rows: {counts:?}"
    );
}
#[tokio::test]
async fn pr192_history_root_query_count_is_flat() {
    history_queries(false, false).await;
}
#[tokio::test]
async fn pr192_history_thread_query_count_is_flat() {
    history_queries(false, true).await;
}
#[tokio::test]
async fn pr192_context_thread_query_count_is_flat() {
    history_queries(true, true).await;
}

async fn work_queries(board: bool) {
    let app = readers().await;
    app.db()
        .write(move |tx| {
            tx.conn()
                .execute("UPDATE agents SET owner_id=NULL WHERE id=?", [AGENT])?;
            tx.conn().execute(
                "UPDATE channel_threads SET work_owner_id=NULL WHERE work_owner_id=?",
                [BOT],
            )?;
            if board {
                tx.conn()
                    .execute("UPDATE rooms SET type='Rooms::Board' WHERE id=?", [ROOM])?;
            }
            Ok(())
        })
        .await
        .unwrap();
    let mut counts = Vec::new();
    for (start, end) in [(0, 5), (5, 50)] {
        app.db().write(move |tx| {
            for n in start..end {
                let id=1996003000+n;
                tx.conn().execute("INSERT INTO channel_threads(id,name,room_id,creator_id,work_owner_id,work_status,last_activity_at,created_at,updated_at) VALUES(?,'Work query probe',?,?,?,'in_progress',?,?,?)",rusqlite::params![id,ROOM,DAVID,BOT,tx.now(),tx.now(),tx.now()])?;
                tx.conn().execute("INSERT INTO thread_tags(channel_thread_id,name,created_at,updated_at) VALUES(?,'probe',?,?)",rusqlite::params![id,tx.now(),tx.now()])?;
            }
            Ok(())
        }).await.unwrap();
        let path = if board {
            format!("/rooms/{ROOM}/agents/posts?owner=agents&tag=probe")
        } else {
            "/agents/work".into()
        };
        let name = if board {
            "list_board_posts"
        } else {
            "list_work"
        };
        let args = if board {
            json!({"room_id":ROOM,"owner":"agents","tag":"probe"})
        } else {
            json!({})
        };
        let rest = measure(
            &app,
            if board { "board_rest" } else { "work_rest" },
            end as usize,
            Method::GET,
            &path,
            None,
        )
        .await;
        let mcp = measure(
            &app,
            if board { "board_mcp" } else { "work_mcp" },
            end as usize,
            Method::POST,
            "/agents/mcp",
            Some(tool(name, args)),
        )
        .await;
        counts.push((rest, mcp));
    }
    assert_eq!(
        counts[0], counts[1],
        "REST and MCP reader SELECTs must stay flat from 5 to 50 rows: {counts:?}"
    );
}
#[tokio::test]
async fn pr192_owned_work_query_count_is_flat() {
    work_queries(false).await;
}
#[tokio::test]
async fn pr192_board_filters_query_count_is_flat() {
    work_queries(true).await;
}

async fn snapshot(app: &TestApp) -> String {
    use sha2::{Digest, Sha256};
    let rows = app
        .db()
        .read(|conn| {
            let mut rows = Vec::new();
            for table in [
                "rooms",
                "action_text_rich_texts",
                "drive_attachments",
                "messages",
                "channel_threads",
                "thread_memberships",
                "work_thread_events",
                "agent_events",
                "background_jobs",
                "active_storage_blobs",
                "active_storage_attachments",
                "active_storage_variant_records",
            ] {
                let mut query = conn.prepare(&format!("SELECT * FROM {table} ORDER BY id"))?;
                let columns = query.column_count();
                let mut cursor = query.query([])?;
                while let Some(row) = cursor.next()? {
                    rows.push((
                        table,
                        (0..columns)
                            .map(|column| format!("{:?}", row.get_ref(column).unwrap()))
                            .collect::<Vec<_>>(),
                    ));
                }
            }
            Ok(rows)
        })
        .await
        .unwrap();
    format!("{:x}", Sha256::digest(serde_json::to_vec(&rows).unwrap()))
}
fn stored_files(app: &TestApp) -> Vec<std::path::PathBuf> {
    fn visit(root: &std::path::Path, path: &std::path::Path, files: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                visit(root, &path, files)
            } else {
                files.push(path.strip_prefix(root).unwrap().to_owned())
            }
        }
    }
    let root = app.booted.app.storage.service.root();
    let mut files = Vec::new();
    visit(root, root, &mut files);
    files.sort();
    files
}
#[tokio::test]
async fn pr192_failed_thread_attachment_rolls_back_post_reopen_and_queue() {
    let app = setup().await.without_job_runner().await;
    app.db().write(|tx| {
        tx.conn().execute_batch("INSERT INTO channel_threads(id,name,room_id,creator_id,work_owner_id,work_status,closed_at,last_activity_at,created_at,updated_at) VALUES(1900700020,'Review',486777696,394959859,394959859,'in_progress','2026-03-01 16:00:00','2026-03-01 16:00:00','2026-03-01 16:00:00','2026-03-01 16:00:00'); CREATE TRIGGER review_reject_variant BEFORE INSERT ON active_storage_variant_records BEGIN SELECT RAISE(ABORT,'review variant failure'); END")?;
        Ok(())
    }).await.unwrap();
    let vectors: Value = serde_json::from_str(include_str!(
        "../../../../vectors/agent_attachments_http.json"
    ))
    .unwrap();
    let case = vectors["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["name"] == "attachment_rest_direct_1_0")
        .unwrap();
    let mut body: Value = serde_json::from_str(case["body"].as_str().unwrap()).unwrap();
    body["thread_id"] = json!(1900700020);
    body["message"]["client_message_id"] = json!("pr192-media-failure");
    let before = snapshot(&app).await;
    let files = stored_files(&app);
    let reply = app
        .anonymous()
        .send(request(
            Method::POST,
            "/rooms/486777696/agents/messages",
            Some(body),
        ))
        .await;
    assert_eq!(
        reply.status.as_u16(),
        500,
        "fault must reach media processing"
    );
    let (posts, closed): (i64, Option<String>) = app
        .db()
        .read(|conn| {
            Ok((
                conn.query_row(
                    "SELECT count(*) FROM messages WHERE client_message_id='pr192-media-failure'",
                    [],
                    |row| row.get(0),
                )?,
                conn.query_row(
                    "SELECT closed_at FROM channel_threads WHERE id=1900700020",
                    [],
                    |row| row.get(0),
                )?,
            ))
        })
        .await
        .unwrap();
    println!(
        "WS11-api review thread attachment: status=500; posted_rows={posts}; closed_at={closed:?}"
    );
    assert_eq!(posts, 0, "failed media must roll back the message");
    assert_eq!(
        closed.as_deref(),
        Some("2026-03-01 16:00:00"),
        "failed media must keep the thread closed"
    );
    assert_eq!(
        snapshot(&app).await,
        before,
        "failed media must roll back domain, media and durable queue rows"
    );
    assert_eq!(
        stored_files(&app),
        files,
        "failed media must discard staged variant files"
    );
}

async fn differential(approval: bool) {
    let vectors: Value = serde_json::from_str(include_str!(
        "../../../../vectors/agent_review192_http.json"
    ))
    .unwrap();
    let mut failures = Vec::new();
    let mut passed = 0;
    for case in vectors["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|case| case["name"].as_str().unwrap().starts_with("approval_") == approval)
    {
        let name = case["name"].as_str().unwrap();
        let app = readers().await;
        let zone = case["setup"]["zone"].as_str().map(str::to_owned);
        let board = case["setup"]["board"].as_bool().unwrap_or(false);
        app.db().write(move |tx| {
            if let Some(zone)=zone {
                tx.conn().execute("UPDATE users SET time_zone=? WHERE id=?",rusqlite::params![zone,BOT])?;
                tx.conn().execute("INSERT INTO agent_grants(agent_id,capability,granted_by_id,created_at,updated_at) VALUES(?,'external_action',?,?,?)",rusqlite::params![AGENT,DAVID,tx.now(),tx.now()])?;
            }
            if board {tx.conn().execute("UPDATE rooms SET type='Rooms::Board' WHERE id=?",[ROOM])?;}
            Ok(())
        }).await.unwrap();
        let mut req = request(
            Method::from_bytes(case["method"].as_str().unwrap().as_bytes()).unwrap(),
            case["path"].as_str().unwrap(),
            None,
        );
        if let Some(body) = case["body"].as_str() {
            req = req.body(body);
        }
        let reply: Reply = app.anonymous().send(req).await;
        let mut matches = reply.status.as_u16() == case["status"].as_u64().unwrap() as u16
            && reply.text() == case["response"].as_str().unwrap();
        for (key, value) in case["headers"].as_object().unwrap() {
            matches &= reply.header(key) == value.as_str();
        }
        if approval {
            let stored:Option<String>=app.db().read(move |conn| {
                let mut query=conn.prepare("SELECT expires_at FROM agent_approvals WHERE agent_id=? ORDER BY id DESC LIMIT 1")?;
                let mut rows=query.query([AGENT])?;
                Ok(rows.next()?.map(|row|row.get::<_,campfire_db::Timestamp>(0)).transpose()?.map(|time|time.jiff().strftime("%Y-%m-%dT%H:%M:%S.%6fZ").to_string()))
            }).await.unwrap();
            matches &= stored.as_deref() == case["stored_deadline"].as_str();
        }
        if matches {
            passed += 1;
        } else {
            failures.push(format!(
                "{name}: expected {} {}, received {} {}",
                case["status"],
                case["response"],
                reply.status,
                reply.text()
            ));
        }
    }
    println!(
        "WS11-api review {} differential: {passed} passed; {} failed",
        if approval {
            "approval zone"
        } else {
            "ID coercion"
        },
        failures.len()
    );
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
#[tokio::test]
async fn pr192_approval_deadlines_match_rails_zones_dates_and_dst() {
    differential(true).await;
}
#[tokio::test]
async fn pr192_context_and_reader_id_shapes_match_rails() {
    differential(false).await;
}
