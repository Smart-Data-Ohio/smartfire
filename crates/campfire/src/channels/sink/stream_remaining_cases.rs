//! Remaining MessageStreamingTest names: real callbacks, durable jobs and rendered socket frames.
use crate::channels::tests::support::{Client, bind_listener};
use crate::controllers::presenters::test_support::{SEED_NOW, TestApp, david_cookie};
use campfire_db::models::{agent_posting, agent_streaming};
use campfire_db::{Agent, AgentKind, Message, MessageChanges, NewAgent, NewMessage, Room, User};
use serde_json::{Value, json};
use std::sync::Arc;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
const ROOM: i64 = 486777696;
const BOT: i64 = 394959859;
const DAVID: i64 = 127326141;
const JASON: i64 = 149087659;
struct Listener(tokio::task::JoinHandle<()>);
impl Drop for Listener {
    fn drop(&mut self) {
        self.0.abort();
    }
}
fn gold(name: &str) -> Value {
    let value: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/agents_stream_remaining.json"
    ))
    .unwrap();
    value["results"][name].clone()
}
fn query_all<T, P: rusqlite::Params>(
    conn: &rusqlite::Connection,
    sql: &str,
    params: P,
    read: impl FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<T>,
) -> campfire_db::Result<Vec<T>> {
    Ok(conn
        .prepare(sql)?
        .query_map(params, read)?
        .collect::<std::result::Result<Vec<_>, _>>()?)
}
struct Observer {
    client: Client,
    other: Client,
    identifier: String,
    frames: Vec<Value>,
    badges: Vec<Value>,
    other_badges: Vec<Value>,
}
async fn snapshot(
    app: &crate::app::App,
    id: i64,
    frames: Vec<Value>,
    badges: Vec<Value>,
    other_badges: Vec<Value>,
) -> Value {
    app.db.read(move|conn| {
        let message=Message::find(conn,id)?;
        let activity=query_all(conn,"SELECT user_id,event_type FROM activity_items WHERE source_type='Message' AND source_id=? ORDER BY user_id",[id],|r|Ok(json!([r.get::<_,i64>(0)?,r.get::<_,String>(1)?])))?;
        let ledger=query_all(conn,"SELECT agent_id,event_type,outcome FROM agent_events WHERE message_id=? ORDER BY id",[id],|r|Ok(json!([r.get::<_,i64>(0)?,r.get::<_,String>(1)?,r.get::<_,Option<String>>(2)?])))?;
        let unread=query_all(conn,"SELECT user_id,unread_at FROM memberships WHERE room_id=? AND user_id IN (?,?) ORDER BY user_id",rusqlite::params![ROOM,DAVID,JASON],|r|Ok(json!([r.get::<_,i64>(0)?,r.get::<_,Option<campfire_db::Timestamp>>(1)?.map(agent_streaming::stamp)])))?;
        let jobs=query_all(conn,"SELECT job_class,arguments FROM background_jobs ORDER BY job_class,id",[],|r|{
            let class:String=r.get(0)?;let args:Value=r.get(1)?;
            let args=match class.as_str(){
                "Agent::DeliveryJob"=>json!([args["event_id"]]),
                "Bot::WebhookJob"=>json!([format!("gid://campfire/User/{}",args["bot_id"]),format!("gid://campfire/Message/{}",args["message_id"])]),
                "Room::PushMessageJob"=>json!([format!("gid://campfire/Rooms::Closed/{}",args["room_id"]),format!("gid://campfire/Message/{}",args["message_id"])]),
                "Message::StreamTrailingBroadcastJob"=>json!([args["message_id"],args["last_broadcast_at"]]),
                _=>panic!("unexpected side-effect job {class}: {args}"),
            };Ok(json!({"class":class,"args":args}))
        })?;
        Ok(json!({"streaming":message.streaming,"source":message.markdown_source,"activity":activity,"ledger":ledger,
            "indexed":conn.query_row("SELECT EXISTS(SELECT 1 FROM message_search_index WHERE rowid=? AND message_search_index MATCH 'hovercraft')",[id],|r|r.get::<_,bool>(0))?,
            "unread":unread,"jobs":jobs,"frames":frames,"badges":{DAVID.to_string():badges,JASON.to_string():other_badges}}))
    }).await.unwrap()
}
async fn compare_phase(app: &crate::app::App, id: i64, observer: &mut Observer, expected: &Value) {
    let expected_frames: Vec<Value> = Vec::new();
    let expected_badges = expected["badges"][DAVID.to_string()].as_array().unwrap();
    let new_count = expected_frames.len() - observer.frames.len() + expected_badges.len()
        - observer.badges.len();
    for _ in 0..new_count {
        let envelope: Value = serde_json::from_str(&observer.client.next_text().await).unwrap();
        if envelope["identifier"] == observer.identifier {
            observer.frames.push(envelope["message"].clone());
        } else {
            observer.badges.push(envelope["message"].clone());
        }
    }
    for _ in observer.other_badges.len()
        ..expected["badges"][JASON.to_string()]
            .as_array()
            .unwrap()
            .len()
    {
        let envelope: Value = serde_json::from_str(&observer.other.next_text().await).unwrap();
        observer.other_badges.push(envelope["message"].clone());
    }
    observer.client.assert_silent().await;
    observer.other.assert_silent().await;
    let mut expected = expected.clone();
    expected["frames"] = json!([]);
    let mut expected = expected.clone();
    expected["frames"] = json!([]);
    let observed = snapshot(
        app,
        id,
        observer.frames.clone(),
        observer.badges.clone(),
        observer.other_badges.clone(),
    )
    .await;
    assert_eq!(
        observed, expected,
        "state, jobs and complete rendered frames"
    );
}
async fn case(name: &'static str) {
    let clock = Arc::new(campfire_kit::FrozenClock::new(SEED_NOW.parse().unwrap()));
    let (app, _dir) = TestApp::boot_with_clock(clock.clone())
        .await
        .expect("default seed")
        .stop_jobs()
        .await;
    app.db
        .write(|tx| {
            tx.conn().execute("DELETE FROM agent_events", [])?;
            tx.conn().execute("DELETE FROM agent_grants", [])?;
            for (table, seq) in [
                ("users", 1901700010),
                ("agents", 1901700011),
                ("messages", 1901720000),
                ("agent_events", 1901730000),
            ] {
                tx.conn().execute(
                    "UPDATE sqlite_sequence SET seq=? WHERE name=?",
                    rusqlite::params![seq, table],
                )?;
            }
            let watcher = User::create_bot(tx, "Stream Watcher", None)?;
            assert_eq!(watcher.id, 1901700011);
            let agent = Agent::create(
                tx,
                NewAgent {
                    user_id: watcher.id,
                    owner_id: Some(DAVID),
                    kind: AgentKind::Workspace,
                    ..Default::default()
                },
            )?;
            assert_eq!(agent.id, 1901700012);
            tx.conn().execute(
                "UPDATE sqlite_sequence SET seq=1901700020 WHERE name='users'",
                [],
            )?;
            let legacy = User::create_bot(
                tx,
                "Legacy Stream",
                Some("https://example.test/legacy-stream"),
            )?;
            assert_eq!(legacy.id, 1901700021);
            Room::find(tx.conn(), ROOM)?.grant_to(tx, &[BOT, watcher.id, legacy.id])?;
            tx.conn().execute(
                "UPDATE memberships SET unread_at=NULL WHERE room_id=? AND user_id IN (?,?)",
                rusqlite::params![ROOM, DAVID, JASON],
            )?;
            tx.conn().execute("DELETE FROM background_jobs", [])?;
            Ok(())
        })
        .await
        .unwrap();
    let listener = bind_listener().await;
    let addr = listener.local_addr().unwrap();
    let router = app.cable.router::<()>("/cable");
    let _listener = Listener(tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    }));
    let mut request = format!("ws://{addr}/cable").into_client_request().unwrap();
    request.headers_mut().insert(
        "origin",
        format!(
            "{}://{addr}",
            if app.cable.config().assume_ssl {
                "https"
            } else {
                "http"
            }
        )
        .parse()
        .unwrap(),
    );
    request
        .headers_mut()
        .insert("cookie", david_cookie().parse().unwrap());
    request.headers_mut().insert(
        "sec-websocket-protocol",
        "actioncable-v1-json".parse().unwrap(),
    );
    let (socket, _) = tokio_tungstenite::connect_async(request).await.unwrap();
    let mut client = Client { socket };
    assert_eq!(client.next_text().await, r#"{"type":"welcome"}"#);
    let identifier = json!({"channel":"ReadRoomsChannel"}).to_string();
    client.confirm(&identifier).await;
    client
        .confirm(&json!({"channel":"UnreadRoomsChannel"}).to_string())
        .await;
    let vectors: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/campfire_sessions.json"
    ))
    .unwrap();
    let cookie = vectors["sessions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["user_id"] == JASON)
        .unwrap()["cookie_header"]
        .as_str()
        .unwrap();
    let mut other_request = format!("ws://{addr}/cable").into_client_request().unwrap();
    other_request.headers_mut().insert(
        "origin",
        format!(
            "{}://{addr}",
            if app.cable.config().assume_ssl {
                "https"
            } else {
                "http"
            }
        )
        .parse()
        .unwrap(),
    );
    other_request
        .headers_mut()
        .insert("cookie", cookie.parse().unwrap());
    other_request.headers_mut().insert(
        "sec-websocket-protocol",
        "actioncable-v1-json".parse().unwrap(),
    );
    let (other_socket, _) = tokio_tungstenite::connect_async(other_request)
        .await
        .unwrap();
    let mut other = Client {
        socket: other_socket,
    };
    assert_eq!(other.next_text().await, r#"{"type":"welcome"}"#);
    other
        .confirm(&json!({"channel":"UnreadRoomsChannel"}).to_string())
        .await;
    let source = match name {
        "start" => "Starting",
        "finalize" => "Hey @[David] and @[Stream Watcher] and @[Legacy Stream] hovercraft",
        "append" => "Hey @[David]",
        "trailing" => "One",
        _ => unreachable!(),
    };
    let id = app
        .db
        .write(move |tx| {
            Ok(Message::create(
                tx,
                NewMessage {
                    room_id: ROOM,
                    creator_id: BOT,
                    streaming: true,
                    markdown_source: Some(source.into()),
                    client_message_id: Some(format!("ws11-next-2-{name}")),
                    ..Default::default()
                },
            )?
            .id)
        })
        .await
        .unwrap();
    let gold = gold(name);
    assert_eq!(id, gold["message_id"].as_i64().unwrap());
    let mut observer = Observer {
        client,
        other,
        identifier,
        frames: vec![],
        badges: vec![],
        other_badges: vec![],
    };
    let mut returns = vec![];
    compare_phase(&app, id, &mut observer, &gold["snapshots"][0]).await;
    match name {
        "start" => {
            app.db
                .write(move |tx| {
                    agent_posting::broadcast_stream_start(tx, &Message::find(tx.conn(), id)?)
                })
                .await
                .unwrap();
            compare_phase(&app, id, &mut observer, &gold["snapshots"][1]).await;
        }
        "finalize" => {
            for phase in 1..=2 {
                returns.push(
                    app.db
                        .write(move |tx| Message::find(tx.conn(), id)?.finalize_stream(tx))
                        .await
                        .unwrap(),
                );
                compare_phase(&app, id, &mut observer, &gold["snapshots"][phase]).await;
            }
        }
        "append" => {
            for (phase, text) in [
                (1, "Hey @[David] hovercraft"),
                (2, "Hey @[David] hovercraft eels"),
            ] {
                app.db
                    .write(move |tx| {
                        let mut m = Message::find(tx.conn(), id)?;
                        m.update(
                            tx,
                            MessageChanges {
                                markdown_source: Some(text.into()),
                                ..Default::default()
                            },
                        )?;
                        if phase == 1 {
                            agent_streaming::broadcast_update(tx, &mut m)?;
                        }
                        Ok(())
                    })
                    .await
                    .unwrap();
                compare_phase(&app, id, &mut observer, &gold["snapshots"][phase]).await;
            }
            returns.push(
                app.db
                    .write(move |tx| Message::find(tx.conn(), id)?.finalize_stream(tx))
                    .await
                    .unwrap(),
            );
            compare_phase(&app, id, &mut observer, &gold["snapshots"][3]).await;
        }
        "trailing" => {
            returns.push(
                app.db
                    .write(move |tx| {
                        agent_streaming::broadcast_update(tx, &mut Message::find(tx.conn(), id)?)
                    })
                    .await
                    .unwrap(),
            );
            returns.push(
                app.db
                    .write(move |tx| {
                        let mut m = Message::find(tx.conn(), id)?;
                        m.update(
                            tx,
                            MessageChanges {
                                markdown_source: Some("One two".into()),
                                ..Default::default()
                            },
                        )?;
                        agent_streaming::broadcast_update(tx, &mut m)
                    })
                    .await
                    .unwrap(),
            );
            compare_phase(&app, id, &mut observer, &gold["snapshots"][1]).await;
            let job:agent_streaming::StreamTrailingBroadcastJob=app.db.read(|conn|{let args:Value=conn.query_row("SELECT arguments FROM background_jobs WHERE job_class='Message::StreamTrailingBroadcastJob'",[],|r|r.get(0))?;Ok(serde_json::from_value(args).unwrap())}).await.unwrap();
            clock.advance(jiff::SignedDuration::from_millis(350));
            // Run the actual registered handler with the real queued arguments.
            crate::integrations::run_trailing_fixture(app.clone(), job).await;
            compare_phase(&app, id, &mut observer, &gold["snapshots"][2]).await;
        }
        _ => unreachable!(),
    }
    assert_eq!(json!(returns), gold["returns"]);
    observer.client.socket.close(None).await.unwrap();
    observer.other.socket.close(None).await.unwrap();
    println!("WS11 remaining streaming: {name}; state/jobs/complete socket frames match Rails");
}
#[tokio::test]
async fn ws11_next2_stream_start_without_unreads() {
    case("start").await;
}
#[tokio::test]
async fn ws11_next2_stream_finalize_every_side_effect_once() {
    case("finalize").await;
}
#[tokio::test]
async fn ws11_next2_stream_appends_render_without_side_effects() {
    case("append").await;
}
#[tokio::test]
async fn ws11_next2_stream_trailing_final_text() {
    case("trailing").await;
}
