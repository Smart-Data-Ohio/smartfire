use crate::controllers::presenters::test_support::{Req, TestApp};
use crate::integrations::agent_repositories::{RepositoryReader, RepositoryRequest};
use crate::net::BoxFuture;
use campfire_db::{AgentCredential, NewCredential};
use campfire_kit::Method;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

const AGENT: i64 = 773018776;
const BOT: i64 = 394959859;
const DAVID: i64 = 127326141;
const ROOM: i64 = 486777696;
const ACCOUNT: i64 = 1996100000;
const SECRET: &str = "pr192-review-private-query-credential";

// Replaces the complete repository-access seam: no request can reach the network.
struct AllowRepository {
    calls: AtomicUsize,
}
impl RepositoryReader for AllowRepository {
    fn readable(&self, request: RepositoryRequest) -> BoxFuture<'_, campfire_db::Result<bool>> {
        assert_eq!(request.account_id, ACCOUNT);
        assert_eq!(request.user_id, DAVID);
        assert_eq!((&*request.owner, &*request.repo), ("review", "private"));
        self.calls.fetch_add(1, Ordering::Relaxed);
        Box::pin(async { Ok(true) })
    }
}

async fn setup() -> (TestApp, Arc<AllowRepository>) {
    let app = TestApp::boot_frozen()
        .await
        .expect("default seed required")
        .without_job_runner()
        .await;
    let crypto = app.booted.app.ar_encryption.clone();
    app.db().write(move |tx| {
        tx.conn().execute("UPDATE agents SET owner_id=?,status='idle',status_note=NULL,status_changed_at=NULL,working_presence=NULL,working_presence_expires_at=NULL,last_seen_at=NULL WHERE id=?", rusqlite::params![DAVID, AGENT])?;
        tx.conn().execute("DELETE FROM agent_events WHERE agent_id=?", [AGENT])?;
        tx.conn().execute("DELETE FROM agent_grants WHERE agent_id=?", [AGENT])?;
        tx.conn().execute("DELETE FROM agent_credentials WHERE agent_id=?", [AGENT])?;
        tx.conn().execute("DELETE FROM github_connected_accounts WHERE user_id=?", [DAVID])?;
        let digest = format!("{:x}", Sha256::digest(SECRET));
        AgentCredential::create(tx, NewCredential { agent_id:AGENT, created_by_id:DAVID,
            name:"PR192 private query probe".into(), token_last_four:digest[..4].into(),
            token_digest:digest, ..Default::default() })?;
        tx.conn().execute("INSERT INTO github_connected_accounts(id,user_id,github_login,access_token,created_at,updated_at) VALUES(?,?,'review-owner',?,?,?)", rusqlite::params![ACCOUNT, DAVID, crypto.encrypt("obviously-fake-review-token"), tx.now(), tx.now()])?;
        tx.conn().execute("UPDATE channel_threads SET work_owner_id=NULL WHERE work_owner_id=?", [BOT])?;
        Ok(())
    }).await.unwrap();
    let reader = Arc::new(AllowRepository {
        calls: AtomicUsize::new(0),
    });
    app.booted.app.agent_repositories.install(reader.clone());
    (app, reader)
}

async fn add_work(app: &TestApp, start: i64, end: i64) {
    app.db().write(move |tx| {
        for n in start..end {
            let thread = 1996101000 + n;
            let pr = 1996102000 + n;
            let link = 1996103000 + n;
            tx.conn().execute("INSERT INTO channel_threads(id,name,room_id,creator_id,work_owner_id,work_status,last_activity_at,created_at,updated_at) VALUES(?,'Private work query probe',?,?,?,'in_progress',?,?,?)", rusqlite::params![thread, ROOM, DAVID, BOT, tx.now(), tx.now(), tx.now()])?;
            tx.conn().execute("INSERT INTO github_pull_requests(id,owner,repo,number,title,state,head_branch,base_branch,private,created_at,updated_at) VALUES(?,'review','private',?,'Private title','open','secret-branch','main',1,?,?)", rusqlite::params![pr, n+1, tx.now(), tx.now()])?;
            tx.conn().execute("INSERT INTO work_thread_links(id,channel_thread_id,kind,github_pull_request_id,created_by_id,created_at,updated_at) VALUES(?,?,'pull_request',?,?,?,?)", rusqlite::params![link, thread, pr, DAVID, tx.now(), tx.now()])?;
        }
        Ok(())
    }).await.unwrap();
}

fn req(mcp: bool) -> Req {
    let mut req = Req::new(
        if mcp { Method::POST } else { Method::GET },
        if mcp { "/agents/mcp" } else { "/agents/work" },
    )
    .header("accept", "application/json")
    .header("content-type", "application/json")
    .header("authorization", &format!("Bearer {SECRET}"));
    if mcp {
        req = req.body(
            json!({"jsonrpc":"2.0","id":192,"method":"tools/call",
        "params":{"name":"list_work","arguments":{}}})
            .to_string(),
        );
    }
    req
}

async fn measure(app: &TestApp, reader: &AllowRepository, mcp: bool, size: usize) -> Vec<String> {
    let label = if mcp {
        "mcp_private_work"
    } else {
        "rest_private_work"
    };
    let warm = app.anonymous().send(req(mcp)).await;
    assert_eq!(warm.status.as_u16(), 200, "{label}: {}", warm.text());
    let start_calls = reader.calls.load(Ordering::Relaxed);
    let log = app.db().capture_read_queries();
    let response = app.anonymous().send(req(mcp)).await;
    app.db().stop_capturing_read_queries();
    assert_eq!(
        response.status.as_u16(),
        200,
        "{label}: {}",
        response.text()
    );
    assert_eq!(
        response.body, warm.body,
        "{label}: warm/current byte equality"
    );
    let wire = response.json();
    assert!(wire.get("error").is_none(), "{label}: {wire}");
    assert_ne!(wire["result"]["isError"], json!(true), "{label}: {wire}");
    let payload: &Value = if mcp {
        &wire["result"]["structuredContent"]
    } else {
        &wire
    };
    let rows = payload.as_array().expect("work array");
    assert_eq!(rows.len(), size);
    for row in rows {
        let links = row["links"].as_array().expect("work links");
        assert_eq!(links.len(), 1, "{label}: {row}");
        assert_eq!(links[0]["title"], "Private title");
        assert_eq!(links[0]["pull_request"]["head_branch"], "secret-branch");
    }
    let statements = log
        .lock()
        .unwrap()
        .iter()
        .filter(|sql| sql.trim_start().to_ascii_uppercase().starts_with("SELECT"))
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(
        reader.calls.load(Ordering::Relaxed) - start_calls,
        size,
        "permission service must still run for every occurrence"
    );
    println!(
        "PR192_R2_PRIVATE interface={label} size={size} SELECTs={}",
        statements.len()
    );
    statements
}

fn print_query_difference(measurements: &[Vec<String>]) {
    let mut frequencies = BTreeMap::<&str, [usize; 2]>::new();
    for (index, (size, statements)) in [5, 50].into_iter().zip(measurements).enumerate() {
        eprintln!("PR192_R2_PRIVATE SQL size={size} SELECTs={}", statements.len());
        // SQLITE_TRACE_STMT supplies statement text, not expanded bind values.
        for (number, sql) in statements.iter().enumerate() {
            eprintln!("  {}: {sql}", number + 1);
            frequencies.entry(sql).or_default()[index] += 1;
        }
    }
    eprintln!("PR192_R2_PRIVATE SQL diff (occurrences at size=5 -> size=50):");
    for (sql, [small, large]) in frequencies {
        if small != large {
            eprintln!("  {small} -> {large}: {sql}");
        }
    }
}

async fn private_queries(mcp: bool) {
    let (app, reader) = setup().await;
    let mut measurements = Vec::new();
    for (start, end) in [(0, 5), (5, 50)] {
        add_work(&app, start, end).await;
        measurements.push(measure(&app, &reader, mcp, end as usize).await);
    }
    let counts = measurements.iter().map(Vec::len).collect::<Vec<_>>();
    if counts[0] != counts[1] {
        print_query_difference(&measurements);
    }
    assert_eq!(
        counts[0], counts[1],
        "private-linked work reads must be flat: {counts:?}"
    );
}
#[tokio::test]
async fn pr192_r2_private_rest_query_count_is_flat() {
    private_queries(false).await;
}
#[tokio::test]
async fn pr192_r2_private_mcp_query_count_is_flat() {
    private_queries(true).await;
}

fn payload_req(path: &str) -> Req {
    Req::new(Method::GET, path)
        .header("accept", "application/json")
        .header(
            "authorization",
            &format!("Bearer {}", super::agent_http_tests::SECRET),
        )
}
async fn payload_queries(context: bool) -> Vec<String> {
    let app = super::agent_review_tests::readers().await;
    let request = || {
        if context {
            payload_req("/agents/context?thread_id=1996000000&limit=50")
        } else {
            Req::new(Method::POST,"/agents/mcp").header("accept","application/json").header("content-type","application/json")
            .header("authorization",&format!("Bearer {}",super::agent_http_tests::SECRET))
            .body(json!({"jsonrpc":"2.0","id":192,"method":"tools/call","params":{"name":"read_messages","arguments":{"room_id":486777696,"limit":50}}}).to_string())
        }
    };
    let warm = app.anonymous().send(request()).await;
    assert_eq!(warm.status.as_u16(), 200, "{}", warm.text());
    let log = app.db().capture_read_queries();
    let reply = app.anonymous().send(request()).await;
    app.db().stop_capturing_read_queries();
    assert_eq!(reply.status.as_u16(), 200, "{}", reply.text());
    assert_eq!(reply.body, warm.body);
    let statements = log
        .lock()
        .unwrap()
        .iter()
        .filter(|sql| sql.trim_start().to_ascii_uppercase().starts_with("SELECT"))
        .cloned()
        .collect::<Vec<_>>();
    println!(
        "PR192_R2_PAYLOAD context={context} SELECTs={}",
        statements.len()
    );
    statements
}
#[tokio::test]
async fn pr192_r2_plain_payload_reads_only_rendered_facts() {
    let queries = payload_queries(false).await;
    let unused = [
        "boosts",
        "message_pins",
        "agent_steps",
        "polls",
        "poll_options",
        "poll_votes",
        "message_references",
        "fizzy_card_references",
        "link_embed_references",
        "event_references",
        "github_pull_request_references",
    ];
    let pattern = regex::Regex::new(&format!(
        r"(?i)\b(?:FROM|JOIN)\s+(?:{})\b",
        unused.join("|")
    ))
    .unwrap();
    let unused = queries
        .iter()
        .filter(|q| pattern.is_match(q))
        .collect::<Vec<_>>();
    assert!(
        unused.is_empty(),
        "unrendered facts were loaded: {unused:?}"
    );
}
#[tokio::test]
async fn pr192_r2_context_reuses_preloaded_authors() {
    let queries = payload_queries(true).await;
    let count = queries
        .iter()
        .filter(|q| q.replace('"', "").contains("FROM users WHERE") && q.contains(" IN ("))
        .count();
    println!("PR192_R2_CONTEXT author_batch_SELECTs={count}");
    assert_eq!(count, 1, "context must share its payload author's batch");
}
pub(super) fn row_state(
    conn: &campfire_db::Connection,
    sql: &str,
    expected: &Value,
) -> campfire_db::Result<Value> {
    Ok(conn.query_row(sql, [], |row| {
        let mut out = serde_json::Map::new();
        for (key, want) in expected.as_object().unwrap() {
            use rusqlite::types::ValueRef;
            let value = match row.get_ref(key.as_str())? {
                ValueRef::Null => Value::Null,
                ValueRef::Integer(n) => {
                    if want.is_boolean() {
                        json!(n != 0)
                    } else {
                        json!(n)
                    }
                }
                ValueRef::Real(n) => json!(n),
                ValueRef::Text(raw) => {
                    let text = std::str::from_utf8(raw).unwrap();
                    if want.is_object() {
                        serde_json::from_str(text).unwrap()
                    } else if want.as_str().is_some_and(|s| s.ends_with(" UTC")) {
                        let time = row.get::<_, campfire_db::Timestamp>(key.as_str())?;
                        json!(time.jiff().strftime("%Y-%m-%d %H:%M:%S UTC").to_string())
                    } else {
                        json!(text)
                    }
                }
                ValueRef::Blob(_) => panic!("unexpected binary field"),
            };
            out.insert(key.clone(), value);
        }
        Ok(Value::Object(out))
    })?)
}
#[tokio::test]
async fn pr192_r2_fresh_jpeg_approved_status_and_committed_state() {
    let app = super::agent_http_tests::setup()
        .await
        .without_job_runner()
        .await;
    app.db().write(|tx| {
        tx.conn().execute_batch("UPDATE agents SET owner_id=127326141,daily_message_cap=NULL WHERE id=773018776; INSERT INTO channel_threads(id,name,room_id,creator_id,work_owner_id,work_status,closed_at,last_activity_at,created_at,updated_at) VALUES(1900700020,'Review',486777696,394959859,394959859,'in_progress','2026-03-01 16:00:00','2026-03-01 16:00:00','2026-03-01 16:00:00','2026-03-01 16:00:00'); DELETE FROM background_jobs;")?;
        Ok(())
    }).await.unwrap();
    let vector: Value = serde_json::from_str(include_str!(
        "../../../../vectors/agent_review192r2_attachment.json"
    ))
    .unwrap();
    super::messages::attachment_processing_tests::fixture_upload(&app, 1, BOT).await;
    let mut case = vector["cases"][0].clone();
    case["state"]["source_metadata"]["uploader_id"] = json!(BOT);
    let reply = app
        .anonymous()
        .send(
            Req::new(Method::POST, case["path"].as_str().unwrap())
                .header("accept", "application/json")
                .header(
                    "authorization",
                    &format!("Bearer {}", super::agent_http_tests::SECRET),
                )
                .header("content-type", "application/json")
                .body(case["body"].as_str().unwrap()),
        )
        .await;
    assert_eq!(
        reply.status.as_u16(),
        case["approved"]["status"].as_u64().unwrap() as u16
    );
    assert_eq!(reply.text(), case["approved"]["response"].as_str().unwrap());
    for (key, value) in case["approved"]["headers"].as_object().unwrap() {
        assert_eq!(reply.header(key), value.as_str());
    }
    super::messages::attachment_processing_tests::perform_queued(&app, 1).await.unwrap();
    let state = case["state"].clone();
    let storage = app.booted.app.storage.clone();
    let actual=app.db().read(move |conn| {
        let message=row_state(conn,"SELECT * FROM messages WHERE client_message_id='pr192-r2-fresh-jpeg'",&state["message"])?;
        let thread=row_state(conn,"SELECT * FROM channel_threads WHERE id=1900700020",&state["thread"])?;
        let variant=row_state(conn,"SELECT * FROM active_storage_variant_records WHERE blob_id=1 ORDER BY id DESC LIMIT 1",&state["variant"])?;
        let image=row_state(conn,"SELECT b.* FROM active_storage_blobs b JOIN active_storage_attachments a ON a.blob_id=b.id WHERE a.record_type='ActiveStorage::VariantRecord' AND a.record_id=(SELECT id FROM active_storage_variant_records WHERE blob_id=1 ORDER BY id DESC LIMIT 1)",&state["image"])?;
        let blob=campfire_storage::Blob::find(conn,image["id"].as_i64().unwrap()).map_err(|e|campfire_db::Error::Other(e.to_string()))?.unwrap();
        let exists=storage.service.exist(&blob.key);
        let attachments=state["attachments"].as_array().unwrap().iter().map(|expected| {
            let sql=format!("SELECT * FROM active_storage_attachments WHERE record_type='{}' AND record_id={}",expected["record_type"].as_str().unwrap(),if expected["name"]=="image" {"(SELECT id FROM active_storage_variant_records WHERE blob_id=1 ORDER BY id DESC LIMIT 1)"} else {"935962058"});
            row_state(conn,&sql,expected)
        }).collect::<campfire_db::Result<Vec<_>>>()?;
        let metadata:String=conn.query_row("SELECT metadata FROM active_storage_blobs WHERE id=1",[],|r|r.get(0))?;
        let jobs=conn.prepare("SELECT job_class,arguments FROM background_jobs ORDER BY id")?.query_map([],|r|Ok(json!({"class":r.get::<_,String>(0)?,"args":serde_json::from_str::<Value>(&r.get::<_,String>(1)?).unwrap()})))?.collect::<std::result::Result<Vec<_>,_>>()?;
        Ok(json!({"message":message,"thread":thread,"variant":variant,"image":image,"attachments":attachments,"source_metadata":serde_json::from_str::<Value>(&metadata).unwrap(),"variant_file_exists":exists,"variant_file_size":if exists {Some(std::fs::metadata(storage.service.path_for(&blob.key)).unwrap().len())} else {None},"jobs":jobs,"variant_count":conn.query_row("SELECT count(*) FROM active_storage_variant_records WHERE blob_id=1",[],|r|r.get::<_,i64>(0))?,"attachment_count":conn.query_row("SELECT count(*) FROM active_storage_attachments WHERE (record_type='Message' AND record_id=935962058) OR (record_type='ActiveStorage::VariantRecord' AND record_id=(SELECT id FROM active_storage_variant_records WHERE blob_id=1 ORDER BY id DESC LIMIT 1))",[],|r|r.get::<_,i64>(0))?}))
    }).await.unwrap();
    println!(
        "PR192_R2_JPEG Rust_status={} Rails_status=201 jobs={} variant_file_exists={}",
        reply.status.as_u16(),
        actual["jobs"].as_array().unwrap().len(),
        actual["variant_file_exists"]
    );
    assert_eq!(
        actual, case["state"],
        "post-job state must match fresh #226 Rails output"
    );
}
