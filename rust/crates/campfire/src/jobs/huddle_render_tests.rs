//! The twenty outstanding grant/stream render declarations, captured from
//! actual Rails callbacks and compared byte-for-byte over real WebSockets.
use crate::channels::broadcasts::Stream as CableStream;
use crate::controllers::presenters::test_support::TestApp;
use campfire_db::models::{huddle_grant::HuddleGrant, room_delete::HuddleConfig, stream::Stream};
use campfire_db::{Room, Session, Timestamp};
use campfire_kit::Crypto;
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::Duration;
use tokio_tungstenite::tungstenite::{Message, client::IntoClientRequest};

type Socket =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

fn insert(tx: &campfire_db::Tx<'_>, table: &str, row: &Value) -> campfire_db::Result<()> {
    use rusqlite::types::Value as Sql;
    let row = row.as_object().unwrap();
    let columns = row
        .keys()
        .map(|k| format!("\"{k}\""))
        .collect::<Vec<_>>()
        .join(",");
    let values = row
        .values()
        .map(|v| match v {
            Value::Null => Sql::Null,
            Value::Bool(v) => Sql::Integer(i64::from(*v)),
            Value::Number(v) => Sql::Integer(v.as_i64().unwrap()),
            Value::String(v) => {
                Sql::Text(Timestamp::parse_db(v).map_or_else(|| v.clone(), |t| t.to_db()))
            }
            _ => Sql::Text(v.to_string()),
        })
        .collect::<Vec<_>>();
    tx.conn().execute(
        &format!(
            "INSERT INTO {table}({columns}) VALUES({}) ON CONFLICT(id) DO UPDATE SET {}",
            vec!["?"; values.len()].join(","),
            row.keys()
                .map(|k| format!("\"{k}\"=excluded.\"{k}\""))
                .collect::<Vec<_>>()
                .join(","),
        ),
        rusqlite::params_from_iter(values),
    )?;
    Ok(())
}

async fn next(socket: &mut Socket) -> Value {
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let message = socket.next().await.expect("socket closed").unwrap();
            if let Message::Text(text) = message {
                let frame: Value = serde_json::from_str(&text).unwrap();
                if frame["type"] != "ping" {
                    return frame;
                }
            }
        }
    })
    .await
    .expect("broadcast did not reach the socket")
}

async fn quiet(socket: &mut Socket, name: &str, operation: &str) {
    let unexpected = tokio::time::timeout(Duration::from_millis(150), async {
        loop {
            let message = socket.next().await.expect("socket closed").unwrap();
            if let Message::Text(text) = &message
                && serde_json::from_str::<Value>(text).unwrap()["type"] == "ping"
            {
                continue;
            }
            return message;
        }
    })
    .await;
    assert!(
        unexpected.is_err(),
        "extra frame: {name} {operation}: {unexpected:?}"
    );
}

async fn run(name: &str) {
    let vectors: Value =
        serde_json::from_str(include_str!("huddle_render_assertions.json")).unwrap();
    assert_eq!(vectors["reference_pin"], &include_str!("../../../../parity/reference.sha").trim()[..8]);
    assert_eq!(vectors["cases"].as_array().unwrap().len(), 20);
    let case = vectors["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == name)
        .unwrap();
    let clock = Arc::new(campfire_kit::clock::FrozenClock::new(
        jiff::Timestamp::from_second(vectors["now"].as_i64().unwrap()).unwrap(),
    ));
    let huddle = crate::huddle::Config::from_lookup(|key| {
        if key == "LIVEKIT_GATEWAY_SECRET" && case["configured"] == false {
            return None;
        }
        Some(
            match key {
                "LIVEKIT_URL" => "wss://public.example.test",
                "LIVEKIT_INTERNAL_URL" => "http://internal.example.test:7880",
                "LIVEKIT_API_KEY" => "ws13-fixture-api-key",
                "LIVEKIT_API_SECRET" => "ws13-fixture-api-secret",
                _ => "ws13-fixture-gateway-secret",
            }
            .into(),
        )
    });
    let test = TestApp::boot_with_huddle_and_clock(huddle, clock.clone())
        .await
        .expect("WS13b requires the parity seed");
    let app = test.booted.app.clone();
    test.booted.jobs.shutdown(Duration::from_secs(2)).await;
    let input = case["input"].clone();
    let room_id = input["room"]["id"].as_i64().unwrap();
    let member_id = input["memberships"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["user_id"] == input["room"]["creator_id"])
        .unwrap()["id"]
        .as_i64()
        .unwrap();
    let session_id = input["session_id"].as_i64().unwrap();
    let initial_grant = input["grant"]["id"].as_i64();
    let initial_stream = input["stream"]["id"].as_i64();
    app.db.write(move |tx| {
        tx.conn().execute_batch("DELETE FROM background_jobs; DELETE FROM huddle_cleanups; DELETE FROM activity_items; DELETE FROM huddle_grants; DELETE FROM streams; DELETE FROM sqlite_sequence WHERE name IN ('huddle_grants','activity_items','streams');")?;
        tx.conn().execute("DELETE FROM memberships WHERE room_id=?",[room_id])?;
        insert(tx,"rooms",&input["room"])?;
        for member in input["memberships"].as_array().unwrap() { insert(tx,"memberships",member)?; }
        for user in input["users"].as_array().unwrap() {
            tx.conn().execute("UPDATE users SET name=?,role=?,status=?,updated_at=? WHERE id=?",rusqlite::params![user["name"].as_str(),campfire_db::Role::from_name(user["role"].as_str().unwrap()).unwrap(),campfire_db::Status::from_name(user["status"].as_str().unwrap()).unwrap(),Timestamp::parse_db(user["updated_at"].as_str().unwrap()).unwrap(),user["id"].as_i64()])?;
        }
        if !input["grant"].is_null() { insert(tx,"huddle_grants",&input["grant"])?; }
        if !input["stream"].is_null() { insert(tx,"streams",&input["stream"])?; }
        Ok(())
    }).await.unwrap();
    let room = app
        .db
        .read(move |conn| Room::find(conn, room_id))
        .await
        .unwrap();
    let listener = crate::channels::tests::support::bind_listener().await;
    let addr = listener.local_addr().unwrap();
    let (stop, stopping) = tokio::sync::oneshot::channel::<()>();
    let router = test.booted.router.clone();
    let serving = tokio::spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(async {
                let _ = stopping.await;
            })
            .await
            .unwrap();
    });
    let mut sockets = Vec::new();
    for destination in case["destinations"].as_array().unwrap() {
        let user_id = destination["user_id"].as_i64().unwrap();
        let session = app
            .db
            .write(move |tx| {
                Session::start_with(
                    tx,
                    user_id,
                    campfire_db::NewSession {
                        two_factor_verified: true,
                        ..Default::default()
                    },
                )
            })
            .await
            .unwrap();
        let signed = campfire_kit::RailsCrypto::new(app.secrets.clone()).sign_cookie(
            "session_token",
            &session.token,
            None,
        );
        let mut request = format!("ws://{addr}/cable").into_client_request().unwrap();
        request
            .headers_mut()
            .insert("origin", format!("http://{addr}").parse().unwrap());
        request.headers_mut().insert(
            "cookie",
            format!("session_token={}", campfire_kit::cookies::escape(&signed))
                .parse()
                .unwrap(),
        );
        request.headers_mut().insert(
            "sec-websocket-protocol",
            "actioncable-v1-json".parse().unwrap(),
        );
        let (mut socket, _) = tokio_tungstenite::connect_async(request).await.unwrap();
        assert_eq!(next(&mut socket).await["type"], "welcome");
        let header = destination["kind"] == "header";
        let stream = if header {
            CableStream::room_messages(&room)
        } else {
            CableStream::user_rooms(user_id)
        };
        assert_eq!(
            stream.streamables().join(":"),
            destination["stream"].as_str().unwrap()
        );
        let identifier = json!({"channel":if header {"RoomMessagesChannel"} else {"Turbo::StreamsChannel"},"signed_stream_name":rails_compat::turbo::signed_stream_name(&app.secrets,&stream.streamables())}).to_string();
        socket
            .send(Message::Text(
                json!({"command":"subscribe","identifier":identifier})
                    .to_string()
                    .into(),
            ))
            .await
            .unwrap();
        assert_eq!(next(&mut socket).await["type"], "confirm_subscription");
        sockets.push((
            destination["stream"].as_str().unwrap().to_owned(),
            identifier,
            socket,
        ));
    }
    let config = HuddleConfig {
        api_secret: Some("ws13-fixture-api-secret".into()),
        admin_configured: false,
    };
    let mut grant_id = initial_grant;
    let mut stream_id = initial_stream;
    for step in case["steps"].as_array().unwrap() {
        let before = app
            .db
            .read(|conn| {
                Ok(
                    conn.query_row("SELECT COALESCE(MAX(id),0) FROM background_jobs", [], |r| {
                        r.get::<_, i64>(0)
                    })?,
                )
            })
            .await
            .unwrap();
        let operation = step["op"].as_str().unwrap();
        let mut runner = None;
        let value = match operation {
            "issue" => {
                let config = config.clone();
                let grant = app
                    .db
                    .write(move |tx| {
                        HuddleGrant::issue(tx, session_id, member_id, room_id, &config)
                    })
                    .await
                    .unwrap();
                grant_id = Some(grant.id);
                json!(true)
            }
            "seen" | "repeat_seen" => {
                if operation == "repeat_seen" {
                    clock.advance(jiff::SignedDuration::from_secs(11));
                }
                let id = grant_id.unwrap();
                app.db
                    .write(move |tx| {
                        HuddleGrant::find_by_id(tx.conn(), id)?
                            .unwrap()
                            .record_seen(tx)
                    })
                    .await
                    .unwrap();
                Value::Null
            }
            "presence_job" => {
                let mut registry = super::Registry::new();
                super::huddle::register(&mut registry);
                runner = Some(campfire_jobs::start(
                    app.db.clone(),
                    app.jobs.queue.clone(),
                    registry,
                    app.clone(),
                    crate::queue::runner_config(&app.config),
                ));
                Value::Null
            }
            "mark_out" => {
                let id = grant_id.unwrap();
                json!(
                    app.db
                        .write(move |tx| HuddleGrant::find_by_id(tx.conn(), id)?
                            .unwrap()
                            .mark_out_of_call(tx, None))
                        .await
                        .unwrap()
                )
            }
            "revoke" => {
                let id = grant_id.unwrap();
                let config = config.clone();
                app.db
                    .write(move |tx| {
                        HuddleGrant::find_by_id(tx.conn(), id)?
                            .unwrap()
                            .revoke(tx, true, &config)
                    })
                    .await
                    .unwrap();
                Value::Null
            }
            "destroy" => {
                app.db
                    .write(move |tx| Room::find(tx.conn(), room_id)?.destroy(tx))
                    .await
                    .unwrap();
                Value::Null
            }
            "stream_start" => {
                let stream = app
                    .db
                    .write(move |tx| {
                        Stream::create(tx, room_id, member_id, room.creator_id, "1080p15", None)
                    })
                    .await
                    .unwrap();
                stream_id = Some(stream.id);
                Value::Null
            }
            "stream_end" | "host_stop" | "self_stop" => {
                let id = stream_id.unwrap();
                let actor = (operation != "stream_end").then_some(room.creator_id);
                app.db
                    .write(move |tx| {
                        Stream::find_by_id(tx.conn(), id)?
                            .unwrap()
                            .end(tx, actor)
                            .map(drop)
                    })
                    .await
                    .unwrap();
                Value::Null
            }
            _ => panic!("unknown operation {operation}"),
        };
        assert_eq!(value, step["value"], "{name} {operation}");
        if let Some(id) = grant_id {
            let now = app.db.env().now();
            let state=app.db.read(move |conn| {let grant=HuddleGrant::find_by_id(conn,id)?.unwrap();Ok(json!({"last_seen_at":grant.last_seen_at.map(|at|at.as_second()),"revoked":grant.revoked(),"in_call":grant.in_call(now)}))}).await.unwrap();
            assert_eq!(state, step["grant_state"], "{name} {operation} grant state");
        }
        let jobs=app.db.read(move |conn| {let mut stmt=conn.prepare("SELECT arguments FROM background_jobs WHERE id>? AND job_class='Huddle::BroadcastPresenceJob' ORDER BY id")?;Ok(stmt.query_map([before],|r|r.get::<_,String>(0))?.collect::<std::result::Result<Vec<_>,_>>()?)}).await.unwrap();
        let job_ids = jobs
            .iter()
            .map(|s| {
                serde_json::from_str::<Value>(s).unwrap()["grant_id"]
                    .as_i64()
                    .unwrap()
            })
            .collect::<Vec<_>>();
        assert_eq!(
            json!(job_ids),
            step["presence_jobs"],
            "{name} {operation} enqueue"
        );
        for (stream, identifier, socket) in &mut sockets {
            let expected = step["frames"][&*stream].as_array().unwrap();
            for html in expected {
                let frame = next(socket).await;
                assert_eq!(
                    frame["identifier"], *identifier,
                    "{name} {operation} stream"
                );
                assert_eq!(frame["message"], *html, "{name} {operation} {stream}");
                let html = frame["message"].as_str().unwrap();
                assert!(campfire_cable::turbo::session_bound(html).is_none());
                assert!(!campfire_views::helpers::request_forgery::has_token_slots(
                    html
                ));
            }
        }
        if let Some(runner) = runner {
            runner.shutdown(Duration::from_secs(2)).await;
        }
        for (_, _, socket) in &mut sockets {
            quiet(socket, name, operation).await;
        }
    }
    for (_, _, socket) in &mut sockets {
        socket.close(None).await.unwrap();
    }
    let _ = stop.send(());
    tokio::time::timeout(Duration::from_secs(3), serving)
        .await
        .unwrap()
        .unwrap();
}

macro_rules! cases { ($($name:ident),* $(,)?) => {$ (
    #[tokio::test]
    async fn $name() { run(stringify!($name)).await; }
)*}; }
cases!(
    mark_out,
    issue_voice,
    revoke_voice,
    first_voice,
    issue_open,
    issue_closed,
    issue_direct,
    revoke_channel,
    revoke_direct,
    first_channel,
    unconfigured,
    destroyed_room,
    issue_stage,
    revoke_stage,
    stream_start,
    stream_end,
    stream_twice,
    host_stop,
    self_stop,
    automatic_stop
);
