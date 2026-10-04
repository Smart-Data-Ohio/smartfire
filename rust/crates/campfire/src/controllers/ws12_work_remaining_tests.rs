//! Complete named board/link request sequences; no bodies or saved facts are masked.
use super::presenters::test_support::{Req, TestApp, with_fixed_render_secrets};
use crate::channels::tests::support::{Client, bind_listener, identifier};
use campfire_kit::Method;
use serde_json::{Value, json};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
struct Server(tokio::task::JoinHandle<()>);
impl Drop for Server {
    fn drop(&mut self) {
        self.0.abort();
    }
}
async fn subscribe(app: &TestApp, room: i64) -> (Server, Client) {
    let listener = bind_listener().await;
    let address = listener.local_addr().unwrap();
    let router = app.booted.router.clone();
    let server = Server(tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    }));
    let mut request = format!("ws://{address}/cable")
        .into_client_request()
        .unwrap();
    request
        .headers_mut()
        .insert("origin", format!("http://{address}").parse().unwrap());
    let sessions: Value =
        serde_json::from_str(include_str!("../../../../vectors/campfire_sessions.json")).unwrap();
    request.headers_mut().insert(
        "cookie",
        sessions["sessions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["user_name"] == "David")
            .unwrap()["cookie_header"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap(),
    );
    request.headers_mut().insert(
        "sec-websocket-protocol",
        "actioncable-v1-json".parse().unwrap(),
    );
    let (socket, _) = tokio_tungstenite::connect_async(request).await.unwrap();
    let mut client = Client { socket };
    assert_eq!(client.next_text().await, r#"{"type":"welcome"}"#);
    let gid = crate::channels::room_gid(
        &app.db()
            .read(move |c| campfire_db::Room::find(c, room))
            .await
            .unwrap(),
    )
    .to_param();
    let signed =
        rails_compat::turbo::signed_stream_name(&app.booted.app.secrets, &[&gid, "messages"]);
    client
        .confirm(&identifier(
            json!({"channel":"RoomMessagesChannel","signed_stream_name":signed}),
        ))
        .await;
    (server, client)
}
fn all(
    conn: &campfire_db::Connection,
    sql: &str,
    f: impl FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<Value>,
) -> campfire_db::Result<Vec<Value>> {
    Ok(conn
        .prepare(sql)?
        .query_map([], f)?
        .collect::<rusqlite::Result<_>>()?)
}
fn facts(conn: &campfire_db::Connection) -> campfire_db::Result<Value> {
    let threads = all(
        conn,
        "SELECT id,name,work_status,work_owner_id FROM channel_threads ORDER BY id",
        |r| {
            Ok(json!([
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Option<String>>(2)?,
                r.get::<_, Option<i64>>(3)?
            ]))
        },
    )?;
    let items = all(
        conn,
        "SELECT user_id,source_type,source_id,event_type,read_at,handled_at FROM activity_items ORDER BY id",
        |r| {
            let time =
                |t: campfire_db::Timestamp| t.jiff().strftime("%Y-%m-%dT%H:%M:%S.000Z").to_string();
            Ok(json!([
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, Option<campfire_db::Timestamp>>(4)?.map(time),
                r.get::<_, Option<campfire_db::Timestamp>>(5)?.map(time)
            ]))
        },
    )?;
    let links = all(
        conn,
        "SELECT id,channel_thread_id,kind,url,title,event_id,github_pull_request_id FROM work_thread_links ORDER BY id",
        |r| {
            Ok(json!([
                r.get::<_, i64>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, Option<String>>(3)?,
                r.get::<_, Option<String>>(4)?,
                r.get::<_, Option<i64>>(5)?,
                r.get::<_, Option<i64>>(6)?
            ]))
        },
    )?;
    let memberships = campfire_db::Membership::for_room(conn, 699448332)?
        .into_iter()
        .filter(|m| m.user_id < 901881000)
        .map(|m| json!([m.user_id, m.unread()]))
        .collect::<Vec<_>>();
    let mut memberships = memberships;
    memberships.sort_by_key(|r| r[0].as_i64());
    let followers = all(
        conn,
        "SELECT thread_id,user_id,involvement FROM thread_memberships ORDER BY thread_id,user_id",
        |r| {
            Ok(json!([
                r.get::<_, i64>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, String>(2)?
            ]))
        },
    )?;
    let messages = all(
        conn,
        "SELECT id,creator_id,thread_id,markdown_source FROM messages ORDER BY id",
        |r| {
            Ok(json!([
                r.get::<_, i64>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, Option<i64>>(2)?,
                r.get::<_, Option<String>>(3)?
            ]))
        },
    )?;
    Ok(
        json!({"threads":threads,"items":items,"links":links,"memberships":memberships,"followers":followers,"messages":messages}),
    )
}
async fn compare(key: &str) -> Vec<usize> {
    let oracle: Value =
        serde_json::from_str(include_str!("../../../../vectors/ws12_work_remaining.json")).unwrap();
    let row = oracle["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == key)
        .unwrap();
    let mut measured = Vec::new();
    for size in [10, 100] {
        let env = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../parity/.env.reference"),
        )
        .unwrap();
        let vapid = env
            .lines()
            .filter_map(|l| l.split_once('='))
            .filter(|(k, _)| matches!(*k, "VAPID_PUBLIC_KEY" | "VAPID_PRIVATE_KEY"))
            .collect::<Vec<_>>();
        let app = TestApp::boot_frozen_with_env(&vapid)
            .await
            .unwrap()
            .without_job_runner()
            .await;
        let setup = row["setup"].clone();
        app.db().write(move|tx|{
            tx.conn().execute_batch("PRAGMA defer_foreign_keys=ON")?;
            for deletes in [true,false] {for q in setup.as_array().unwrap(){let sql=q.as_str().unwrap();if sql.starts_with("DELETE FROM ")==deletes {tx.conn().execute_batch(sql)?;}}}
            // Inaccessible unrelated rooms grow without adding legitimate recipients
            // or changing the response. Their policies must never add per-row reads.
            for i in 0..size {tx.conn().execute("INSERT INTO users(id,name,role,status,created_at,updated_at) VALUES(?,'Unrelated user',0,0,?,?)",rusqlite::params![901881000+i,tx.now(),tx.now()])?;}
            Ok(())
        }).await.unwrap();
        let mut anonymous = app.anonymous();
        let mut david = app.david();
        let mut kevin = app.sign_in(super::presenters::test_support::KEVIN).await;
        let mut jz = app.sign_in(773523953).await;
        let mut jason = app.sign_in(super::presenters::test_support::JASON).await;
        let mut cable = if let Some(room) = row["broadcast_room"].as_i64() {
            Some(subscribe(&app, room).await)
        } else {
            None
        };
        let mut counts = Vec::new();
        for (index, step) in row["steps"].as_array().unwrap().iter().enumerate() {
            if let Some(sql) = step["sql"].as_array() {
                let sql = sql.clone();
                app.db()
                    .write(move |tx| {
                        for q in sql {
                            tx.conn().execute_batch(q.as_str().unwrap())?;
                        }
                        Ok(())
                    })
                    .await
                    .unwrap();
            }
            let method =
                Method::from_bytes(step["method"].as_str().unwrap().to_uppercase().as_bytes())
                    .unwrap();
            let mut request = Req::new(method, step["path"].as_str().unwrap())
                .header(
                    "accept",
                    step["accept"].as_str().unwrap_or("application/json"),
                )
                .header(
                    "content-type",
                    if step["raw"].is_string() {
                        "text/plain"
                    } else {
                        "application/json"
                    },
                )
                .body(if let Some(raw) = step["raw"].as_str() {
                    raw.as_bytes().to_vec()
                } else {
                    serde_json::to_vec(&step["params"]).unwrap()
                });
            if step["agent"] == true {
                request = request.header(
                    "authorization",
                    &["Bearer", "bender-test-secret-1234"].join(" "),
                );
            }
            let browser = if step["agent"] == true {
                &mut anonymous
            } else {
                match step["viewer"].as_str() {
                    Some("kevin") => &mut kevin,
                    Some("jz") => &mut jz,
                    Some("jason") => &mut jason,
                    _ => &mut david,
                }
            };
            let queries = app.db().capture_queries();
            let response = with_fixed_render_secrets(browser.write(request)).await;
            app.db().stop_capturing_queries();
            counts.push(queries.lock().unwrap().len());
            assert_eq!(
                response.status.as_u16() as u64,
                step["status"].as_u64().unwrap(),
                "{key} step {index}: status {}",
                response.text()
            );
            if response.text() != step["body"].as_str().unwrap() {
                super::presenters::test_support::rails_mismatch(
                    &response.text(),
                    step["body"].as_str().unwrap(),
                    &format!("{key} step {index}: complete Rails response bytes"),
                );
            }
            for header in ["content-type", "cache-control", "location"] {
                assert_eq!(
                    response.header(header),
                    step["headers"][header].as_str(),
                    "{key} step {index}: {header}"
                );
            }

            assert_eq!(
                app.db().read(facts).await.unwrap(),
                step["facts"],
                "{key} step {index}: complete persisted original facts"
            );
            if let Some((_, client)) = &mut cable {
                for frame in step["frames"].as_array().unwrap() {
                    let actual: Value = serde_json::from_str(&client.next_text().await).unwrap();
                    assert_eq!(
                        actual["message"], frame["message"],
                        "{key}: complete Rails room broadcast"
                    );
                }
                client.assert_silent().await;
            }
        }
        println!("WS12_WORK_NAMED {key} unrelated_users={size} SELECTs={counts:?}");
        measured.push(counts);
    }
    assert_eq!(
        measured[0], measured[1],
        "{key}: request reads stay flat across unrelated users"
    );
    measured.remove(0)
}

#[tokio::test]
async fn ws12_work_c111_complete_named_rails_http_sequence() {
    compare("c111").await;
}

#[tokio::test]
async fn ws12_work_c112_complete_named_rails_http_sequence() {
    compare("c112").await;
}

#[tokio::test]
async fn ws12_work_c113_complete_named_rails_http_sequence() {
    compare("c113").await;
}

#[tokio::test]
async fn ws12_work_c114_complete_named_rails_http_sequence() {
    compare("c114").await;
}

#[tokio::test]
async fn ws12_work_c115_complete_named_rails_http_sequence() {
    compare("c115").await;
}

#[tokio::test]
async fn ws12_work_c120_complete_named_rails_http_sequence() {
    compare("c120").await;
}

#[tokio::test]
async fn ws12_work_c121_complete_named_rails_http_sequence() {
    compare("c121").await;
}

#[tokio::test]
async fn ws12_work_c125_complete_named_rails_http_sequence() {
    compare("c125").await;
}

#[tokio::test]
async fn ws12_work_c126_complete_named_rails_http_sequence() {
    compare("c126").await;
}

#[tokio::test]
async fn ws12_work_c134_complete_named_rails_http_sequence() {
    compare("c134").await;
}

#[tokio::test]
async fn ws12_work_c135_complete_named_rails_http_sequence() {
    compare("c135").await;
}

#[tokio::test]
async fn ws12_work_c136_complete_named_rails_http_sequence() {
    compare("c136").await;
}

#[tokio::test]
async fn ws12_work_c138_complete_named_rails_http_sequence() {
    compare("c138").await;
}

#[tokio::test]
async fn ws12_work_c140_complete_named_rails_http_sequence() {
    compare("c140").await;
}

#[tokio::test]
async fn ws12_work_c229_complete_named_rails_http_sequence() {
    compare("c229").await;
}

#[tokio::test]
async fn ws12_work_c230_complete_named_rails_http_sequence() {
    compare("c230").await;
}

#[tokio::test]
async fn ws12_work_c119_board_index_reads_are_flat_at_ten_and_a_hundred_posts() {
    let small = compare("c119-10").await;
    let large = compare("c119-100").await;
    println!(
        "WS12_BOARD_INDEX_READS posts=10/100 SELECTs={}/{}",
        small[0], large[0]
    );
    assert_eq!(
        small, large,
        "c119 board render is O(1) in SELECTs per post"
    );
}

#[tokio::test]
async fn ws12_ordinary_work_owner_options_batch_agent_profiles_at_two_sizes() {
    let oracle: Value =
        serde_json::from_str(include_str!("../../../../vectors/ws12_work_remaining.json")).unwrap();
    let profile_oracle: Value = serde_json::from_str(include_str!(
        "../../../../vectors/ws12_owner_profile_reads.json"
    ))
    .unwrap();
    let mut counts = Vec::new();
    for size in [10, 100] {
        let app = TestApp::boot_frozen()
            .await
            .unwrap()
            .without_job_runner()
            .await;
        let setup = oracle["rows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["id"] == "c229")
            .unwrap()["setup"]
            .clone();
        app.db().write(move|tx|{
            for q in setup.as_array().unwrap(){tx.conn().execute_batch(q.as_str().unwrap())?;}
            for i in 0..size {let uid=901896000+i;
                tx.conn().execute("INSERT INTO users(id,name,role,status,created_at,updated_at) VALUES(?,? ,2,0,?,?)",rusqlite::params![uid,format!("Extra eligible {i}"),tx.now(),tx.now()])?;
                tx.conn().execute("INSERT INTO agents(id,user_id,owner_id,kind,provider,description,status,created_at,updated_at) VALUES(?,?,127326141,'workspace','TestLab','Does the work','idle',?,?)",rusqlite::params![uid,uid,tx.now(),tx.now()])?;
                tx.conn().execute("INSERT INTO memberships(room_id,user_id,involvement,created_at,updated_at) VALUES(486777696,?,'mentions',?,?)",rusqlite::params![uid,tx.now(),tx.now()])?;
            }Ok(())
        }).await.unwrap();
        let mut browser = app.sign_in(773523953).await;
        let request = || Req::new(Method::GET, "/rooms/486777696/threads/91.json");
        browser.send(request()).await;
        let queries = app.db().capture_queries();
        let response = browser.send(request()).await;
        app.db().stop_capturing_queries();
        assert_eq!(response.status.as_u16(), 200);
        let golden = profile_oracle["rows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["size"] == size)
            .unwrap();
        assert_eq!(
            response.text(),
            golden["body"].as_str().unwrap(),
            "complete owner-picker JSON bytes at {size} agents"
        );
        assert_eq!(response.header("pragma"), Some("no-cache"));
        let payload: Value = serde_json::from_str(&response.text()).unwrap();
        let options = payload["thread"]["work_owner_options"].as_array().unwrap();
        let extra = options
            .iter()
            .filter(|o| {
                o["id"]
                    .as_i64()
                    .is_some_and(|id| (901896000..901896000 + size).contains(&id))
            })
            .collect::<Vec<_>>();
        assert_eq!(extra.len(), size as usize);
        assert!(extra.iter().all(|o| o["agent"] == true
            && o["provider"] == "TestLab"
            && o["description"] == "Does the work"));
        counts.push(queries.lock().unwrap().len());
    }
    println!(
        "WS12_OWNER_PROFILE_READS agents=10/100 SELECTs={}/{}",
        counts[0], counts[1]
    );
    assert_eq!(
        counts[0], counts[1],
        "owner-picker profiles must be read together across the room"
    );
}
