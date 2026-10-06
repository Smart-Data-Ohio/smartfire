//! Committed Rails work-write requests, permission precedence and durable effects.
use super::agent_http_tests::AGENT;
use super::presenters::test_support::TestApp;
use campfire_db::{Agent, AgentKind, NewAgent, Tx, User, Webhook};
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};

thread_local! {
    static WRITER_SELECTS: std::cell::RefCell<Option<Arc<Mutex<Vec<String>>>>> = const { std::cell::RefCell::new(None) };
}
fn writer_query(event: rusqlite::trace::TraceEvent<'_>) {
    if let rusqlite::trace::TraceEvent::Stmt(_, sql) = event
        && sql.trim_start().to_ascii_uppercase().starts_with("SELECT")
    {
        WRITER_SELECTS.with(|slot| {
            if let Some(log) = slot.borrow().as_ref() {
                log.lock().unwrap().push(sql.into());
            }
        });
    }
}
async fn capture_writer(app: &TestApp) -> Arc<Mutex<Vec<String>>> {
    let log = Arc::new(Mutex::new(Vec::new()));
    let captured = log.clone();
    app.db()
        .write(move |tx| {
            WRITER_SELECTS.with(|slot| slot.replace(Some(captured)));
            tx.conn().trace_v2(
                rusqlite::trace::TraceEventCodes::SQLITE_TRACE_STMT,
                Some(writer_query),
            );
            Ok(())
        })
        .await
        .unwrap();
    log
}
async fn stop_writer(app: &TestApp) {
    app.db()
        .write(|tx| {
            tx.conn()
                .trace_v2(rusqlite::trace::TraceEventCodes::empty(), None);
            WRITER_SELECTS.with(|slot| slot.replace(None));
            Ok(())
        })
        .await
        .unwrap();
}
fn reader_selects(log: &Mutex<Vec<String>>) -> usize {
    log.lock()
        .unwrap()
        .iter()
        .filter(|sql| sql.trim_start().to_ascii_uppercase().starts_with("SELECT"))
        .count()
}

#[tokio::test]
async fn agent_work_writes_query_capture_observes_real_writer_and_reader_selects() {
    let app = TestApp::boot_frozen()
        .await
        .unwrap()
        .without_job_runner()
        .await;
    let readers = app.db().capture_read_queries();
    let writer = capture_writer(&app).await;
    assert_eq!(
        app.db()
            .write(|tx| Ok(tx
                .conn()
                .query_row("SELECT 11", [], |row| row.get::<_, i64>(0))?))
            .await
            .unwrap(),
        11
    );
    assert_eq!(
        app.db()
            .read(|conn| Ok(conn.query_row("SELECT 12", [], |row| row.get::<_, i64>(0))?))
            .await
            .unwrap(),
        12
    );
    stop_writer(&app).await;
    app.db().stop_capturing_read_queries();
    assert_eq!(&*writer.lock().unwrap(), &["SELECT 11"]);
    assert_eq!(&*readers.lock().unwrap(), &["SELECT 12"]);
}

pub(super) fn fixture(tx: &mut Tx<'_>, config: &Value) -> campfire_db::Result<()> {
    tx.conn().execute(
        "UPDATE agents SET daily_board_post_cap=?,suspended_at=? WHERE id=?",
        rusqlite::params![
            config["board_cap"].as_i64(),
            if config["inactive"] == true {
                Some(tx.now())
            } else {
                None
            },
            AGENT
        ],
    )?;
    if config["credential_revoked"] == true {
        tx.conn().execute(
            "UPDATE agent_credentials SET revoked_at=? WHERE agent_id=?",
            rusqlite::params![tx.now(), AGENT],
        )?;
    }
    if config["credential_expired"] == true {
        tx.conn().execute(
            "UPDATE agent_credentials SET expires_at=? WHERE agent_id=?",
            rusqlite::params![tx.now(), AGENT],
        )?;
    }
    let user = User::create_bot(tx, "Receiver", None)?;
    tx.conn()
        .execute("UPDATE users SET id=1901100001 WHERE id=?", [user.id])?;
    Webhook::create(tx, 1901100001, Some("https://receiver.example.test/hook"))?;
    tx.conn().execute(
        "UPDATE sqlite_sequence SET seq=1901100001 WHERE name='agents'",
        [],
    )?;
    let receiver = Agent::create(
        tx,
        NewAgent {
            user_id: 1901100001,
            owner_id: Some(127326141),
            kind: AgentKind::Workspace,
            ..Default::default()
        },
    )?;
    assert_eq!(receiver.id, 1901100002);
    tx.conn().execute("INSERT INTO memberships(room_id,user_id,created_at,updated_at) VALUES(486777696,1901100001,?,?)",[tx.now(),tx.now()])?;
    for cap in ["read_messages", "post_messages", "manage_threads"] {
        tx.conn().execute("INSERT INTO agent_grants(agent_id,capability,room_id,granted_by_id,created_at,updated_at) VALUES(1901100002,?,486777696,127326141,?,?)",rusqlite::params![cap,tx.now(),tx.now()])?;
    }
    match config["receiver"].as_str() {
        Some("inactive") => {
            tx.conn().execute(
                "UPDATE agents SET suspended_at=? WHERE id=1901100002",
                [tx.now()],
            )?;
        }
        Some("outside") => {
            tx.conn()
                .execute("DELETE FROM memberships WHERE user_id=1901100001", [])?;
        }
        Some(flag) => {
            let cap = match flag {
                "no_post" => "post_messages",
                "no_manage" => "manage_threads",
                "no_read" => "read_messages",
                _ => panic!("unknown receiver flag"),
            };
            tx.conn().execute(
                "DELETE FROM agent_grants WHERE agent_id=1901100002 AND capability=?",
                [cap],
            )?;
        }
        None => {}
    }
    if config["untracked"] == true {
        tx.conn().execute(
            "UPDATE channel_threads SET work_status=NULL,work_owner_id=NULL WHERE id=1900700020",
            [],
        )?;
    }
    if let Some(result) = config["result"].as_str() {
        tx.conn().execute(
            "UPDATE channel_threads SET result_markdown=? WHERE id=1900700020",
            [result],
        )?;
    }
    if config["rule"] == true {
        campfire_db::BoardTagAssignment::create(
            tx,
            campfire_db::NewBoardTagAssignment {
                room_id: 486777696,
                tag: "launch".into(),
                assignee_id: 1901100001,
                created_by_id: 127326141,
            },
        )?;
    }
    for i in 0..config["size"].as_i64().unwrap_or(0) {
        tx.conn().execute("INSERT INTO channel_threads(id,name,room_id,creator_id,work_owner_id,work_status,last_activity_at,created_at,updated_at) VALUES(?, ?,486777696,394959859,394959859,'planned',?,?,?)",rusqlite::params![1901200000+i,format!("Other {i}"),tx.now(),tx.now(),tx.now()])?;
    }
    for (i, table) in [
        "channel_threads",
        "work_thread_events",
        "work_handoffs",
        "agent_events",
        "audit_logs",
    ]
    .into_iter()
    .enumerate()
    {
        tx.conn()
            .execute("DELETE FROM sqlite_sequence WHERE name=?", [table])?;
        tx.conn().execute(
            "INSERT INTO sqlite_sequence(name,seq) VALUES(?,?)",
            rusqlite::params![table, 1901300000 + i as i64 * 1000],
        )?;
    }
    tx.conn().execute("DELETE FROM background_jobs", [])?;
    Ok(())
}

fn rows(conn: &campfire_db::Connection, sql: &str) -> campfire_db::Result<Vec<Value>> {
    let mut stmt = conn.prepare(sql)?;
    let strings = stmt
        .query_map([], |r| r.get::<_, String>(0))?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(strings
        .into_iter()
        .map(|s| serde_json::from_str(&s).unwrap())
        .collect())
}
pub(super) async fn assert_state(app: &TestApp, expected: &Value, name: &str) {
    assert_state_with_job_facts(app,expected,name,false).await;
}
/// Rails' test adapter and the atomic durable queue are compared as full logical
/// job facts where a named declaration does not specify job execution ordering.
pub(super) async fn assert_state_with_job_facts(app:&TestApp, expected:&Value, name:&str, unordered:bool) {
    let mut actual=app.db().read(|conn| {
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
    let mut expected = expected.clone();
    if unordered {
        for value in [&mut actual,&mut expected] {
            value["jobs"].as_array_mut().unwrap().sort_by_key(Value::to_string);
        }
    }
    if actual != expected {println!("WS11_STATE_ACTUAL {name} {actual}");}
    assert_eq!(actual, expected, "{name}: committed state");
    app.db().read(|conn| {
        let chains=rows(conn,"SELECT json_object('chain',chain_id,'hop',hop,'thread',json_extract(metadata,'$.thread_id'),'kind',event_type) FROM agent_events WHERE id>1901303000 AND event_type IN ('work_assigned','work_unassigned','work_handed_off') ORDER BY id")?;
        for (index,row) in chains.iter().enumerate() {
            assert!(uuid::Uuid::parse_str(row["chain"].as_str().unwrap()).is_ok());
            if row["kind"]=="work_handed_off" && index>0 && chains[index-1]["kind"]=="work_unassigned" {
                assert_eq!(row["thread"],chains[index-1]["thread"]);
                assert_eq!(row["chain"],chains[index-1]["chain"]);
            }
        }
        Ok(())
    }).await.unwrap();
}

async fn group(surface: &str, success_only: bool) {
    let vectors: Value = serde_json::from_str(include_str!(
        "../../../../vectors/agent_work_writes_http.json"
    ))
    .unwrap();
    for case in vectors["cases"].as_array().unwrap().iter().filter(|c| {
        c["surface"] == surface && (!success_only || c["name"] == format!("{surface}_success"))
    }) {
        super::agent_reads_tests::check(case).await;
    }
}
macro_rules! surface_tests {
    ($($name:ident => $surface:literal),* $(,)?)=>{$(
        #[tokio::test] async fn $name(){group($surface,false).await;}
    )*};
}
surface_tests! {
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

const SURFACES: [&str; 9] = [
    "rest_create",
    "rest_update",
    "rest_result",
    "rest_handoff",
    "mcp_create",
    "mcp_update",
    "mcp_board_update",
    "mcp_result",
    "mcp_handoff",
];
fn vectors() -> Value {
    serde_json::from_str(include_str!(
        "../../../../vectors/agent_work_writes_http.json"
    ))
    .unwrap()
}
#[tokio::test]
async fn agent_work_writes_reader_queries_are_flat_for_every_surface() {
    let vectors = vectors();
    let mut excessive = Vec::new();
    for surface in SURFACES {
        let mut counts = Vec::new();
        let mut rails = Vec::new();
        for size in [5, 50] {
            let case = vectors["cases"]
                .as_array()
                .unwrap()
                .iter()
                .find(|c| c["name"] == format!("{surface}_queries_{size}"))
                .unwrap();
            let app = super::agent_reads_tests::prepare(case).await;
            let log = app.db().capture_read_queries();
            let writer = capture_writer(&app).await;
            let reply = app
                .anonymous()
                .send(super::agent_reads_tests::request(case))
                .await;
            app.db().stop_capturing_read_queries();
            stop_writer(&app).await;
            assert_eq!(
                reply.status.as_u16(),
                case["status"].as_u64().unwrap() as u16
            );
            assert_eq!(reply.text(), case["response_body"].as_str().unwrap());
            counts.push(reader_selects(&log) + writer.lock().unwrap().len());
            rails.push(case["selects"].as_u64().unwrap());
        }
        assert_eq!(
            counts[0], counts[1],
            "{surface}: all request SELECTs must stay flat"
        );
        assert_eq!(rails[0], rails[1], "{surface}: pinned Rails SELECT slope");
        println!(
            "WORK_WRITE_QUERIES {surface} owned_rows=5/50 Rust_all_SELECTs={}/{} Rails_all_SELECTs={}/{}",
            counts[0], counts[1], rails[0], rails[1]
        );
        // Bounds cover current authorization, response facts and the remaining
        // WS12 service/callback cost. Repeated adapter/ledger facts stay removed.
        let limit = match surface {
            "rest_create" => 169,
            "mcp_create" => 167,
            "rest_update" | "mcp_update" | "mcp_board_update" => 83,
            "rest_result" | "mcp_result" => 33,
            "rest_handoff" | "mcp_handoff" => 94,
            _ => unreachable!(),
        };
        if counts[0] > limit {
            excessive.push(format!("{surface}: {} SELECTs exceeds {limit}", counts[0]));
        }
    }
    assert!(excessive.is_empty(), "avoidable API reads: {excessive:?}");
}

async fn domain_snapshot(app: &TestApp) -> Value {
    app.db()
        .read(|conn| {
            let mut snapshot = serde_json::Map::new();
            for table in [
                "channel_threads",
                "messages",
                "thread_memberships",
                "thread_tags",
                "work_thread_events",
                "work_handoffs",
                "agent_events",
                "audit_logs",
                "activity_items",
                "background_jobs",
            ] {
                let mut statement = conn.prepare(&format!("SELECT * FROM {table} ORDER BY id"))?;
                let names = statement
                    .column_names()
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>();
                let data = statement
                    .query_map([], |row| {
                        let mut object = serde_json::Map::new();
                        for (index, name) in names.iter().enumerate() {
                            let value = match row.get_ref(index)? {
                                rusqlite::types::ValueRef::Null => Value::Null,
                                rusqlite::types::ValueRef::Integer(n) => json!(n),
                                rusqlite::types::ValueRef::Real(n) => json!(n),
                                rusqlite::types::ValueRef::Text(s) => {
                                    json!(std::str::from_utf8(s).unwrap())
                                }
                                rusqlite::types::ValueRef::Blob(_) => {
                                    panic!("unexpected blob in work domain")
                                }
                            };
                            object.insert(name.clone(), value);
                        }
                        Ok(Value::Object(object))
                    })?
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                snapshot.insert(table.to_string(), json!(data));
            }
            Ok(Value::Object(snapshot))
        })
        .await
        .unwrap()
}
#[tokio::test]
async fn agent_work_writes_source_history_and_jobs_roll_back_on_insert_failure() {
    let vectors = vectors();
    for surface in SURFACES {
        let case = vectors["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["name"] == format!("{surface}_success"))
            .unwrap();
        // Positive control makes this regression fail against the original seams.
        super::agent_reads_tests::check(case).await;
        let app = super::agent_reads_tests::prepare(case).await;
        let table = if surface.ends_with("create") || surface.ends_with("handoff") {
            "background_jobs"
        } else {
            "work_thread_events"
        };
        app.db().write(move |tx| {
            tx.conn().execute_batch(&format!("CREATE TRIGGER reject_work_write BEFORE INSERT ON {table} BEGIN SELECT RAISE(ABORT,'injected work write failure'); END"))?;
            Ok(())
        }).await.unwrap();
        let before = domain_snapshot(&app).await;
        let reply = app
            .anonymous()
            .send(super::agent_reads_tests::request(case))
            .await;
        if surface.starts_with("rest_") {
            assert_eq!(reply.status, 500);
        } else {
            assert_eq!(reply.json()["error"]["code"], -32603);
        }
        assert_eq!(
            domain_snapshot(&app).await,
            before,
            "{surface}: reject {table} must roll back all source, history, audit, ledger and queue rows"
        );
        println!("WORK_WRITE_ATOMIC {surface} rejected_table={table} all_10_tables_unchanged=true");
    }
}
