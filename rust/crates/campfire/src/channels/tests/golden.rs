//! Frame sequences recorded from the reference app's channels, replayed against ours.
//!
//! `golden/reference.json` holds what `golden/fixtures.rb` created in the reference (the rows
//! the replay loads into a fresh database, the session cookies and the signed stream names)
//! and the frames each socket received at every step of [`script`]. Server-side events (an
//! unread fanout, a message removal, revoking a membership, deactivating a user) run through
//! `golden/trigger.rb` inside the reference, and through the equivalent Rust calls on replay.
//! Both sides use `SECRET_KEY_BASE` from `parity/.env.reference`, so the cookies and signed
//! names work on either. To re-record:
//!
//!   bash reference-tools/cable/record.sh
//!
//! Re-recording uses the current `ws7-reference:d7c7de92` image, a fresh database, port 47040, and
//! `cargo -j 4`. Its container names start with `ws7-`; scratch stays in `/home/riels/.cache/rust-port/ws7/`.
//! The checked-in recording was produced at `fec615be`; #151 changed only the Edge install image path.
//!
//! Frames that reach one socket in one step by different paths (a confirmation and a broadcast)
//! race in Rails, where the confirmation waits for Redis to acknowledge the subscription, so each
//! step's frames are compared per socket as a sorted list. Pings are dropped.
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use campfire_cable::Config;
use campfire_db::{Database, Event, EventSink, Message, Room, User};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;

use crate::channels::{self, Broadcasts, Cable, Deps, sink};

const GOLDEN: &str = "crates/campfire/src/channels/tests/golden/reference.json";
/// How long a replay waits for a frame the recording says is coming.
const EXPECTED_FRAME_WAIT: Duration = crate::test_support::WAIT;
const TRIGGER: &str = "crates/campfire/src/channels/tests/golden/trigger.rb";

/// The repository root (`CAMPFIRE_REPO_ROOT` when this module is built outside the workspace).
fn repo_root() -> std::path::PathBuf {
    std::env::var_os("CAMPFIRE_REPO_ROOT").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
        Into::into,
    )
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Fixtures {
    cookies: BTreeMap<String, String>,
    tokens: BTreeMap<String, String>,
    rows: BTreeMap<String, Vec<serde_json::Map<String, Value>>>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Recording {
    fixtures: Fixtures,
    steps: Vec<Exchange>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct Exchange {
    step: String,
    /// Socket name -> the frames it received during the step (quiet sockets are left out).
    frames: BTreeMap<String, Vec<String>>,
}

enum Step {
    /// Opens socket `.0` with the named cookie, or none.
    Connect(&'static str, Option<&'static str>),
    Send(&'static str, String),
    /// A server-side event, with token names as its arguments.
    Trigger(&'static str, Vec<&'static str>),
}

fn subscribe(identifier: &Value) -> String {
    json!({ "command": "subscribe", "identifier": identifier.to_string() }).to_string()
}

fn perform(identifier: &Value, data: Value) -> String {
    json!({ "command": "message", "identifier": identifier.to_string(), "data": data.to_string() })
        .to_string()
}

fn script(tokens: &BTreeMap<String, String>) -> Vec<(String, Step)> {
    let t = |name: &str| tokens[name].clone();
    let room_id: i64 = t("ROOM_ID").parse().unwrap();
    let closed_id: i64 = t("CLOSED_ID").parse().unwrap();
    let thread_id: i64 = t("THREAD_ID").parse().unwrap();

    let reads = json!({ "channel": "ReadRoomsChannel" });
    let unreads = json!({ "channel": "UnreadRoomsChannel" });
    let presence = json!({ "channel": "PresenceChannel", "room_id": room_id });
    let room = json!({ "channel": "RoomChannel", "room_id": room_id });
    let typing = json!({ "channel": "TypingNotificationsChannel", "room_id": room_id });
    let messages = json!({ "channel": "RoomMessagesChannel", "signed_stream_name": t("ROOM_MESSAGES_SIGNED") });
    let turbo = |signed: String| json!({ "channel": "Turbo::StreamsChannel", "signed_stream_name": signed });
    let guarded = turbo(t("ROOM_MESSAGES_SIGNED"));
    let rooms = turbo(t("ROOMS_SIGNED"));
    let forged = "InJvb21zIg==--0000".to_string();
    let thread_typing = json!({ "channel": "TypingNotificationsChannel", "room_id": room_id, "thread_id": thread_id });
    let workspace = json!({ "channel": "WorkspacePresenceChannel" });

    use Step::*;
    let steps = vec![
        ("connect A", Connect("A", Some("A"))),
        ("connect B", Connect("B", Some("B"))),
        ("connect without a cookie", Connect("X", None)),
        ("connect two-factor pending", Connect("P", Some("PENDING"))),
        (
            "connect bot without two-factor",
            Connect("BOT", Some("BOT")),
        ),
        (
            "bot activity rejected",
            Send("BOT", subscribe(&json!({ "channel": "ActivityChannel" }))),
        ),
        (
            "bot huddle notices rejected",
            Send(
                "BOT",
                subscribe(&json!({ "channel": "HuddleNoticeChannel" })),
            ),
        ),
        (
            "bot agents rejected",
            Send("BOT", subscribe(&json!({ "channel": "AgentsChannel" }))),
        ),
        (
            "A activity",
            Send("A", subscribe(&json!({ "channel": "ActivityChannel" }))),
        ),
        (
            "A huddle notices",
            Send("A", subscribe(&json!({ "channel": "HuddleNoticeChannel" }))),
        ),
        (
            "A agents",
            Send("A", subscribe(&json!({ "channel": "AgentsChannel" }))),
        ),
        (
            "A unread threads",
            Send(
                "A",
                subscribe(&json!({ "channel": "UnreadThreadsChannel" })),
            ),
        ),
        ("A workspace presence", Send("A", subscribe(&workspace))),
        (
            "A workspace heartbeat",
            Send(
                "A",
                perform(&workspace, json!({ "action": "heartbeat", "active": true })),
            ),
        ),
        (
            "A workspace unsubscribe",
            Send(
                "A",
                json!({ "command": "unsubscribe", "identifier": workspace.to_string() })
                    .to_string(),
            ),
        ),
        (
            "A heartbeat",
            Send("A", subscribe(&json!({ "channel": "HeartbeatChannel" }))),
        ),
        (
            "A base channel",
            Send(
                "A",
                subscribe(&json!({ "channel": "ApplicationCable::Channel" })),
            ),
        ),
        ("A reads", Send("A", subscribe(&reads))),
        ("A unreads", Send("A", subscribe(&unreads))),
        ("A presence", Send("A", subscribe(&presence))),
        (
            "A presence in a room A isn't in",
            Send(
                "A",
                subscribe(&json!({ "channel": "PresenceChannel", "room_id": closed_id })),
            ),
        ),
        (
            "A presence with a non-numeric room",
            Send(
                "A",
                subscribe(&json!({ "channel": "PresenceChannel", "room_id": "abc" })),
            ),
        ),
        (
            "A presence with a numeric string room",
            Send(
                "A",
                subscribe(&json!({ "channel": "PresenceChannel", "room_id": room_id.to_string() })),
            ),
        ),
        ("A room", Send("A", subscribe(&room))),
        (
            "A room A isn't in",
            Send(
                "A",
                subscribe(&json!({ "channel": "RoomChannel", "room_id": closed_id })),
            ),
        ),
        (
            "A room without an id",
            Send("A", subscribe(&json!({ "channel": "RoomChannel" }))),
        ),
        ("A typing", Send("A", subscribe(&typing))),
        ("B typing", Send("B", subscribe(&typing))),
        ("A thread typing", Send("A", subscribe(&thread_typing))),
        ("B thread typing", Send("B", subscribe(&thread_typing))),
        (
            "A starts thread typing",
            Send("A", perform(&thread_typing, json!({ "action": "start" }))),
        ),
        (
            "B stops thread typing",
            Send("B", perform(&thread_typing, json!({ "action": "stop" }))),
        ),
        (
            "A starts typing",
            Send("A", perform(&typing, json!({ "action": "start" }))),
        ),
        (
            "B stops typing",
            Send(
                "B",
                perform(&typing, json!({ "action": "stop", "extra": 1 })),
            ),
        ),
        (
            "A performs an unknown typing action",
            Send("A", perform(&typing, json!({ "action": "dance" }))),
        ),
        ("A room messages", Send("A", subscribe(&messages))),
        (
            "A thread messages",
            Send(
                "A",
                subscribe(
                    &json!({ "channel": "RoomMessagesChannel", "signed_stream_name": t("THREAD_MESSAGES_SIGNED") }),
                ),
            ),
        ),
        (
            "A room threads",
            Send(
                "A",
                subscribe(
                    &json!({ "channel": "RoomMessagesChannel", "signed_stream_name": t("ROOM_THREADS_SIGNED") }),
                ),
            ),
        ),
        (
            "A stock thread messages rejected",
            Send("A", subscribe(&turbo(t("THREAD_MESSAGES_SIGNED")))),
        ),
        (
            "A stock room threads rejected",
            Send("A", subscribe(&turbo(t("ROOM_THREADS_SIGNED")))),
        ),
        (
            "A turbo status",
            Send("A", subscribe(&turbo(t("A_STATUS_SIGNED")))),
        ),
        (
            "A turbo ooo notice",
            Send("A", subscribe(&turbo(t("A_OOO_SIGNED")))),
        ),
        (
            "new channel and signature-only broadcasts",
            Trigger("notifications", vec!["A_ID", "ROOM_ID", "THREAD_ID"]),
        ),
        (
            "A room messages for a room A isn't in",
            Send(
                "A",
                subscribe(
                    &json!({ "channel": "RoomMessagesChannel", "signed_stream_name": t("CLOSED_MESSAGES_SIGNED") }),
                ),
            ),
        ),
        (
            "A room messages without a name",
            Send("A", subscribe(&json!({ "channel": "RoomMessagesChannel" }))),
        ),
        (
            "A room messages with a forged name",
            Send(
                "A",
                subscribe(
                    &json!({ "channel": "RoomMessagesChannel", "signed_stream_name": forged }),
                ),
            ),
        ),
        (
            "A room messages with the rooms name",
            Send(
                "A",
                subscribe(
                    &json!({ "channel": "RoomMessagesChannel", "signed_stream_name": t("ROOMS_SIGNED") }),
                ),
            ),
        ),
        ("A turbo rooms", Send("A", subscribe(&rooms))),
        (
            "A turbo own rooms",
            Send("A", subscribe(&turbo(t("A_ROOMS_SIGNED")))),
        ),
        (
            "A turbo guarded room messages",
            Send("A", subscribe(&guarded)),
        ),
        (
            "A turbo forged",
            Send("A", subscribe(&turbo(forged.clone()))),
        ),
        (
            "A presence present",
            Send("A", perform(&presence, json!({ "action": "present" }))),
        ),
        (
            "A presence refresh",
            Send("A", perform(&presence, json!({ "action": "refresh" }))),
        ),
        (
            "A unreads subscribed again",
            Send("A", perform(&unreads, json!({ "action": "subscribed" }))),
        ),
        ("unread fanout", Trigger("unread", vec!["MESSAGE_ID"])),
        (
            "message removed",
            Trigger("remove_message", vec!["MESSAGE_ID"]),
        ),
        ("B presence", Send("B", subscribe(&presence))),
        (
            "A typing again",
            Send("A", perform(&typing, json!({ "action": "start" }))),
        ),
        ("revoke A", Trigger("revoke", vec!["ROOM_ID", "A_ID"])),
        ("reconnect A", Connect("A2", Some("A"))),
        ("A2 presence", Send("A2", subscribe(&presence))),
        ("A2 room", Send("A2", subscribe(&room))),
        ("A2 typing", Send("A2", subscribe(&typing))),
        ("A2 room messages", Send("A2", subscribe(&messages))),
        (
            "A2 turbo guarded room messages",
            Send("A2", subscribe(&guarded)),
        ),
        ("A2 turbo rooms", Send("A2", subscribe(&rooms))),
        ("A2 unreads", Send("A2", subscribe(&unreads))),
        (
            "B typing after A left",
            Send("B", perform(&typing, json!({ "action": "start" }))),
        ),
        ("deactivate B", Trigger("deactivate", vec!["B_ID"])),
        ("reconnect B", Connect("B2", Some("B"))),
    ];
    steps
        .into_iter()
        .map(|(name, step)| (name.to_string(), step))
        .collect()
}

type Socket =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

/// Where the script runs: the reference (triggers through its runner) or our server.
struct Target {
    url: String,
    origin: String,
    fixtures: Fixtures,
    rust: Option<RustApp>,
    reference_port: Option<String>,
    /// How long a socket must stay quiet before a step's frames are considered complete.
    quiet: Duration,
}

struct RustApp {
    db: Database,
    broadcasts: Broadcasts,
}

/// Runs the script, returning what each socket received at each step. With `expected` (a
/// replay), each step first waits, generously, for as many frames as the recording has, so a slow
/// machine can't cut a step short; `quiet` then only has to catch frames beyond those.
async fn run_script(target: &Target, expected: Option<&[Exchange]>) -> Vec<Exchange> {
    let mut sockets: BTreeMap<String, Socket> = BTreeMap::new();
    let mut exchanges = Vec::new();
    for (index, (name, step)) in script(&target.fixtures.tokens).into_iter().enumerate() {
        match step {
            Step::Connect(socket, cookie) => {
                let mut request = target.url.as_str().into_client_request().unwrap();
                let headers = request.headers_mut();
                headers.insert("origin", target.origin.parse().unwrap());
                headers.insert(
                    "sec-websocket-protocol",
                    "actioncable-v1-json, actioncable-unsupported"
                        .parse()
                        .unwrap(),
                );
                if let Some(cookie) = cookie {
                    headers.insert("cookie", target.fixtures.cookies[cookie].parse().unwrap());
                }
                let (ws, _) = crate::test_support::wait("golden replay WebSocket upgrade", tokio_tungstenite::connect_async(request))
                    .await
                    .expect("upgrade");
                sockets.insert(socket.to_string(), ws);
            }
            Step::Send(socket, text) => {
                crate::test_support::wait(
                    "golden replay command send",
                    sockets.get_mut(socket).unwrap().send(WsMessage::Text(text.into())),
                ).await.unwrap();
            }
            Step::Trigger(event, args) => {
                let args: Vec<String> = args
                    .iter()
                    .map(|name| target.fixtures.tokens[*name].clone())
                    .collect();
                trigger(target, event, &args).await;
            }
        }
        let mut frames = BTreeMap::new();
        for (socket, ws) in sockets.iter_mut() {
            let at_least = expected
                .and_then(|steps| steps.get(index))
                .and_then(|step| step.frames.get(socket))
                .map_or(0, Vec::len);
            let received = collect(ws, target.quiet, at_least).await;
            if !received.is_empty() {
                frames.insert(socket.clone(), received);
            }
        }
        exchanges.push(Exchange { step: name, frames });
    }
    exchanges
}

/// Frames until the socket has sent `at_least` of them and then been quiet for a moment. Reading
/// on after a close frame sends the client's half of the close handshake.
async fn collect(ws: &mut Socket, quiet: Duration, at_least: usize) -> Vec<String> {
    let deadline = tokio::time::Instant::now() + EXPECTED_FRAME_WAIT;
    let mut frames = Vec::new();
    loop {
        let wait = if frames.len() < at_least {
            EXPECTED_FRAME_WAIT
        } else {
            quiet
        };
        let until = (tokio::time::Instant::now() + wait).min(deadline);
        let Ok(message) = tokio::time::timeout_at(until, ws.next()).await else {
            break;
        };
        match message {
            Some(Ok(WsMessage::Text(text))) if text.starts_with(r#"{"type":"ping""#) => {}
            Some(Ok(WsMessage::Text(text))) => frames.push(text.to_string()),
            Some(Ok(WsMessage::Close(frame))) => {
                frames.push(format!(
                    "close {:?}",
                    frame.map(|f| (u16::from(f.code), f.reason.to_string()))
                ));
            }
            Some(Ok(_)) => {}
            Some(Err(_)) | None => break,
        }
    }
    frames
}

async fn trigger(target: &Target, event: &str, args: &[String]) {
    if let Some(port) = &target.reference_port {
        let root = repo_root();
        let mut command = if let Ok(container) = std::env::var("CHANNELS_REFERENCE_CONTAINER") {
            let mut command = tokio::process::Command::new("docker");
            command.args([
                "exec",
                "-e",
                "RAILS_LOG_LEVEL=warn",
                &container,
                "bin/rails",
                "runner",
            ]);
            command.arg(format!("/work/{TRIGGER}")).arg(event);
            command
        } else {
            let mut command = tokio::process::Command::new(root.join("parity/bin/reference"));
            command
                .current_dir(&root)
                .args(["runner", "--port", port, TRIGGER, event]);
            command
        };
        let status = command.args(args).status().await.unwrap();
        assert!(status.success(), "trigger {event} failed");
        return;
    }
    let app = target.rust.as_ref().unwrap();
    let ids: Vec<i64> = args.iter().map(|a| a.parse().unwrap()).collect();
    let broadcasts = app.broadcasts.clone();
    match event {
        "unread" => app
            .db
            .read(move |conn| {
                let message = Message::find(conn, ids[0])?;
                broadcasts.unread_room(
                    conn,
                    &Room::find(conn, message.room_id)?,
                    &message,
                    &campfire_db::rich_text::BasicRichText,
                )
            })
            .await
            .unwrap(),
        "remove_message" => app
            .db
            .read(move |conn| {
                let message = Message::find(conn, ids[0])?;
                broadcasts.message_remove(&Room::find(conn, message.room_id)?, &message);
                Ok(())
            })
            .await
            .unwrap(),
        "revoke" => app
            .db
            .write(move |tx| Room::find(tx.conn(), ids[0])?.revoke_from(tx, &[ids[1]]))
            .await
            .unwrap(),
        "deactivate" => app
            .db
            .write(move |tx| User::find(tx.conn(), ids[0])?.deactivate(tx))
            .await
            .unwrap(),
        "notifications" => {
            use crate::channels::broadcasts::Stream;
            broadcasts.channel(
                &format!("user_{}_activity", ids[0]),
                &json!({ "activityItemId": 42 }),
            );
            broadcasts.channel(
                &format!("user_{}_huddle_notices", ids[0]),
                &json!({ "huddleJoinNotice": { "eventType": "huddle_ended", "roomId": ids[1] } }),
            );
            broadcasts.channel(
                &format!("user_{}_unread_threads", ids[0]),
                &json!({ "threadId": ids[2], "roomId": ids[1] }),
            );
            broadcasts.replace(
                &Stream::named("agents:all"),
                "status_badge_agent_1",
                "<span>ready</span>",
            );
            broadcasts.update(
                &Stream::user_status(ids[0]),
                &format!("status_badge_user_{}", ids[0]),
                "<span>online</span>",
            );
            broadcasts.update(
                &Stream::ooo_notice(ids[0]),
                &format!("ooo_notice_user_{}", ids[0]),
                "<span>away</span>",
            );
            broadcasts.remove(&Stream::thread_messages(ids[2]), "message_golden");
            let room = app
                .db
                .read(move |conn| Room::find(conn, ids[1]))
                .await
                .unwrap();
            broadcasts.remove(
                &Stream::record(&channels::room_gid(&room), "threads"),
                "channel_thread_golden",
            );
        }
        other => panic!("unknown trigger {other}"),
    }
}

fn reference_secret_key_base() -> String {
    let env = std::fs::read_to_string(repo_root().join("parity/.env.reference")).unwrap();
    env.lines()
        .find_map(|line| line.strip_prefix("SECRET_KEY_BASE="))
        .expect("SECRET_KEY_BASE")
        .trim()
        .to_string()
}

#[derive(Default)]
struct CableSink {
    server: OnceLock<Cable>,
}

impl EventSink for CableSink {
    fn emit(&self, event: Event) {
        if let Some(server) = self.server.get() {
            sink::deliver(server, None, &event);
        }
    }
}

fn sql_value(value: &Value) -> rusqlite::types::Value {
    use rusqlite::types::Value as Sql;
    match value {
        Value::Null => Sql::Null,
        Value::Bool(b) => Sql::Integer(i64::from(*b)),
        Value::Number(n) => n
            .as_i64()
            .map(Sql::Integer)
            .unwrap_or_else(|| Sql::Real(n.as_f64().unwrap())),
        Value::String(s) => Sql::Text(s.clone()),
        other => Sql::Text(other.to_string()),
    }
}

/// A fresh database holding the reference's rows, and our channels over it.
async fn start_rust(fixtures: &Fixtures, dir: &Path) -> Target {
    let sink = Arc::new(CableSink::default());
    let env = campfire_db::Env {
        sink: sink.clone(),
        ..campfire_db::Env::default()
    };
    let mut config = campfire_db::Config::new(dir.join("production.sqlite3"));
    config.readers = 2;
    let db = Database::open(config, env).unwrap();
    let rows = fixtures.rows.clone();
    db.write(move |tx| {
        for table in [
            "users",
            "rooms",
            "memberships",
            "sessions",
            "messages",
            "channel_threads",
        ] {
            // Only the columns our schema has: the reference records every column of its own
            // (two-factor, agent and bot token state the port doesn't store yet).
            let known: Vec<String> = tx
                .conn()
                .prepare(&format!("SELECT name FROM pragma_table_info('{table}')"))?
                .query_map([], |row| row.get(0))?
                .collect::<Result<_, _>>()?;
            for row in &rows[table] {
                let values: Vec<(&String, &Value)> = row
                    .iter()
                    .filter(|(column, _)| known.contains(column))
                    .collect();
                let columns: Vec<String> = values.iter().map(|(c, _)| format!("\"{c}\"")).collect();
                let sql = format!(
                    "INSERT INTO {table} ({}) VALUES ({})",
                    columns.join(", "),
                    vec!["?"; columns.len()].join(", ")
                );
                tx.conn().execute(
                    &sql,
                    rusqlite::params_from_iter(values.iter().map(|(_, v)| sql_value(v))),
                )?;
            }
        }
        Ok(())
    })
    .await
    .unwrap();

    let secrets = Arc::new(rails_compat::Secrets::new(&reference_secret_key_base()));
    let deps = Deps {
        db: db.clone(),
        secrets: secrets.clone(),
        crypto: Arc::new(campfire_kit::RailsCrypto::new(secrets)),
        clock: Arc::new(campfire_kit::SystemClock),
        admin_session_idle_timeout: crate::config::admin_session_idle_timeout(None),
    };
    // The reference runs with DISABLE_SSL, so without assume_ssl.
    let server = channels::server(
        deps,
        Config {
            assume_ssl: false,
            ..Config::default()
        },
    );
    let _ = sink.server.set(server.clone());

    let listener = super::support::bind_listener().await;
    let addr = listener.local_addr().unwrap();
    let app = server.router::<()>("/cable");
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    Target {
        url: format!("ws://{addr}/cable"),
        origin: format!("http://{addr}"),
        fixtures: fixtures.clone(),
        rust: Some(RustApp {
            db,
            broadcasts: Broadcasts::new(server),
        }),
        reference_port: None,
        quiet: Duration::from_millis(100),
    }
}

fn sorted(steps: &[Exchange]) -> Vec<Exchange> {
    let sort = |frames: &Vec<String>| {
        let mut frames = frames.clone();
        frames.sort();
        frames
    };
    steps
        .iter()
        .map(|exchange| Exchange {
            step: exchange.step.clone(),
            frames: exchange
                .frames
                .iter()
                .map(|(socket, frames)| (socket.clone(), sort(frames)))
                .collect(),
        })
        .collect()
}

#[tokio::test]
#[ignore = "utility: needs a running reference app; see the module docs"]
async fn record_reference() {
    let port = std::env::var("CHANNELS_REFERENCE_PORT").expect("CHANNELS_REFERENCE_PORT");
    let path = std::env::var("CHANNELS_REFERENCE_FIXTURES").expect("CHANNELS_REFERENCE_FIXTURES");
    let fixtures: Fixtures = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let target = Target {
        url: format!("ws://127.0.0.1:{port}/cable"),
        origin: format!("http://127.0.0.1:{port}"),
        fixtures: fixtures.clone(),
        rust: None,
        reference_port: Some(port),
        quiet: Duration::from_millis(500),
    };
    let steps = run_script(&target, None).await;
    let recording = Recording { fixtures, steps };
    std::fs::write(
        repo_root().join(GOLDEN),
        serde_json::to_string_pretty(&recording).unwrap() + "\n",
    )
    .unwrap();
}

#[tokio::test]
async fn replays_reference_frames() {
    let golden: Recording =
        serde_json::from_str(&std::fs::read_to_string(repo_root().join(GOLDEN)).unwrap()).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let target = start_rust(&golden.fixtures, dir.path()).await;
    let actual = sorted(&run_script(&target, Some(&golden.steps)).await);
    for (expected, actual) in sorted(&golden.steps).iter().zip(&actual) {
        assert_eq!(actual, expected, "step {:?}", expected.step);
    }
    assert_eq!(actual.len(), golden.steps.len());
}
