//! A cable server with Campfire's channels over a fixtures database, and a WebSocket client.
#![allow(dead_code)]

use std::sync::{Arc, OnceLock};
use std::time::Duration;

use campfire_cable::Config;
use campfire_db::fixtures::{self, identify};
use campfire_db::rich_text::BasicRichText;
use campfire_db::{
    Boost, Database, Event, EventSink, Membership, Message, NewMessage, NewSession, Room, Session,
    TestClock, WorkspacePresenceLease,
};
use campfire_kit::{Crypto, RailsCrypto, SystemClock};
use futures_util::{SinkExt, StreamExt};
use rails_compat::Secrets;
use serde_json::{Value, json};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

use crate::channels::{self, Broadcasts, Cable, Deps, Partials, sink};

pub use crate::test_support::{WAIT, bind_listener, eventually, wait};

pub const SECRET_KEY_BASE: &str = "channels-test-secret-key-base";

/// Delivers the cable's events (`DisconnectUser`, `Broadcast`) with `channels::sink`, as the app's
/// `Jobs` sink does.
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

pub struct TestApp {
    pub db: Database,
    pub server: Cable,
    pub broadcasts: Broadcasts,
    pub secrets: Arc<Secrets>,
    pub clock: TestClock,
    pub url: String,
    pub origin: String,
    _dir: tempfile::TempDir,
}

pub async fn start() -> TestApp {
    let dir = tempfile::tempdir().unwrap();
    let sink = Arc::new(CableSink::default());
    let clock = TestClock::new();
    let env = campfire_db::Env {
        clock: Arc::new(clock.clone()),
        sink: sink.clone(),
        rich_text: Arc::new(BasicRichText),
        bcrypt_cost: 4,
    };
    let mut config = campfire_db::Config::new(dir.path().join("test.sqlite3"));
    config.readers = 2;
    config.environment = "test".into();
    let db = Database::open(config, env).unwrap();
    db.write(|tx| {
        let options = fixtures::Options {
            now: tx.now(),
            bcrypt_cost: 4,
        };
        fixtures::load(tx.conn(), &fixtures::reference_dir(), &options).map(|_| ())
    })
    .await
    .unwrap();

    let secrets = Arc::new(Secrets::new(SECRET_KEY_BASE));
    let deps = Deps {
        db: db.clone(),
        secrets: secrets.clone(),
        crypto: Arc::new(RailsCrypto::new(secrets.clone())),
        clock: Arc::new(SystemClock),
        admin_session_idle_timeout: crate::config::admin_session_idle_timeout(None),
    };
    let server = channels::server(
        deps,
        Config {
            assume_ssl: false,
            ..Config::default()
        },
    );
    let _ = sink.server.set(server.clone());

    let listener = bind_listener().await;
    let addr = listener.local_addr().unwrap();
    let app = server.router::<()>("/cable");
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    TestApp {
        broadcasts: Broadcasts::new(server.clone()),
        db,
        server,
        secrets,
        clock,
        url: format!("ws://{addr}/cable"),
        origin: format!("http://{addr}"),
        _dir: dir,
    }
}

pub fn count(conn: &campfire_db::Connection, sql: &str, id: i64) -> campfire_db::Result<i64> {
    Ok(conn.query_row(sql, [id], |row| row.get(0))?)
}

pub fn id(label: &str) -> i64 {
    identify(label)
}

impl TestApp {
    /// A session cookie for the fixture user (`cookies.signed[:session_token]`), for a session that
    /// completed two-step sign-in.
    pub async fn cookie_for(&self, user: &str) -> String {
        self.cookie_with_token(&self.session_for(user, true).await.token)
    }

    /// A new session for the fixture user, verified or not.
    pub async fn session_for(&self, user: &str, verified: bool) -> Session {
        let user_id = id(user);
        let attributes = NewSession {
            user_agent: Some("test"),
            ip_address: Some("8.8.8.8"),
            two_factor_verified: verified,
            ..Default::default()
        };
        self.db
            .write(move |tx| Session::start_with(tx, user_id, attributes))
            .await
            .unwrap()
    }

    /// Connects with the session's cookie and reads the welcome.
    pub async fn connect_with_session(&self, session: &Session) -> Client {
        let cookie = self.cookie_with_token(&session.token);
        let mut client = self.connect_with_cookie(Some(&cookie)).await;
        assert_eq!(client.next_text().await, r#"{"type":"welcome"}"#);
        client
    }

    /// Runs one statement, bypassing the models (as `delete`/`update_columns` do in the Ruby
    /// tests).
    pub async fn sql(&self, sql: &'static str, params: Vec<rusqlite::types::Value>) -> usize {
        self.db
            .write(move |tx| Ok(tx.conn().execute(sql, rusqlite::params_from_iter(params))?))
            .await
            .unwrap()
    }

    /// Every workspace presence lease, oldest first.
    pub async fn leases(&self) -> Vec<WorkspacePresenceLease> {
        self.db.read(|conn| read_leases(conn, || {})).await.unwrap()
    }

    pub async fn session_exists(&self, session_id: i64) -> bool {
        self.db
            .read(move |conn| {
                Ok(crate::channels::tests::support::count(
                    conn,
                    "SELECT COUNT(*) FROM sessions WHERE id = ?",
                    session_id,
                )? > 0)
            })
            .await
            .unwrap()
    }

    /// `room.messages.create!(body:, creator:, client_message_id:)`.
    pub async fn create_message(
        &self,
        room: &str,
        creator: &str,
        body: &str,
        client_message_id: &str,
    ) -> Message {
        let attributes = NewMessage {
            room_id: id(room),
            creator_id: id(creator),
            body: Some(body.to_string()),
            client_message_id: Some(client_message_id.to_string()),
            ..Default::default()
        };
        self.db
            .write(move |tx| Message::create(tx, attributes))
            .await
            .unwrap()
    }

    pub fn cookie_with_token(&self, token: &str) -> String {
        let signed =
            RailsCrypto::new(self.secrets.clone()).sign_cookie("session_token", token, None);
        format!(
            "session_token={}",
            signed
                .replace('+', "%2B")
                .replace('/', "%2F")
                .replace('=', "%3D")
        )
    }

    /// Connects as the fixture user and reads the welcome.
    pub async fn connect(&self, user: &str) -> Client {
        let cookie = self.cookie_for(user).await;
        let mut client = self.connect_with_cookie(Some(&cookie)).await;
        assert_eq!(client.next_text().await, r#"{"type":"welcome"}"#);
        client
    }

    pub async fn connect_with_cookie(&self, cookie: Option<&str>) -> Client {
        let mut request = self.url.as_str().into_client_request().unwrap();
        let headers = request.headers_mut();
        headers.insert("origin", self.origin.parse().unwrap());
        headers.insert(
            "sec-websocket-protocol",
            "actioncable-v1-json, actioncable-unsupported"
                .parse()
                .unwrap(),
        );
        if let Some(cookie) = cookie {
            headers.insert("cookie", cookie.parse().unwrap());
        }
        let (socket, _) = wait("WebSocket upgrade", tokio_tungstenite::connect_async(request))
            .await
            .expect("upgrade");
        Client { socket }
    }

    pub fn signed_stream_name(&self, streamables: &[&str]) -> String {
        rails_compat::turbo::signed_stream_name(&self.secrets, streamables)
    }

    pub async fn room(&self, label: &str) -> Room {
        let room_id = id(label);
        self.db
            .read(move |conn| Room::find(conn, room_id))
            .await
            .unwrap()
    }

    pub async fn membership(&self, room: &str, user: &str) -> Option<Membership> {
        let (room_id, user_id) = (id(room), id(user));
        self.db
            .read(move |conn| Membership::find_by_room_and_user(conn, room_id, user_id))
            .await
            .unwrap()
    }

    pub async fn message(&self, label: &str) -> Message {
        let message_id = id(label);
        self.db
            .read(move |conn| Message::find(conn, message_id))
            .await
            .unwrap()
    }

    /// `ChannelThread.create!(room:, creator:, name:)`, as a bare row: the channels only read
    /// threads.
    pub async fn create_thread(&self, room: &str, creator: &str, name: &str) -> i64 {
        let (room_id, creator_id, name) = (id(room), id(creator), name.to_string());
        self.db
            .write(move |tx| {
                let now = tx.now();
                Ok(tx.conn().query_row(
                    "INSERT INTO channel_threads (room_id, creator_id, name, last_activity_at, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?) RETURNING id",
                    rusqlite::params![room_id, creator_id, name, now, now, now],
                    |row| row.get(0),
                )?)
            })
            .await
            .unwrap()
    }

    pub async fn set_involvement(&self, room: &str, user: &str, involvement: &str) {
        let (room_id, user_id, involvement) = (id(room), id(user), involvement.to_string());
        self.db
            .write(move |tx| {
                tx.conn().execute(
                    "UPDATE memberships SET involvement = ? WHERE room_id = ? AND user_id = ?",
                    rusqlite::params![involvement, room_id, user_id],
                )?;
                Ok(())
            })
            .await
            .unwrap();
    }

    pub async fn set_body(&self, message: &Message, html: &str) {
        let (message_id, html) = (message.id, html.to_string());
        self.db
            .write(move |tx| {
                tx.conn().execute(
                    "UPDATE action_text_rich_texts SET body = ? WHERE record_type = 'Message' AND record_id = ? AND name = 'body'",
                    rusqlite::params![html, message_id],
                )?;
                Ok(())
            })
            .await
            .unwrap();
    }

    /// `message.broadcast_create` with [`FakePartials`].
    pub async fn message_create(&self, room: &Room, message: &Message) {
        let (broadcasts, room, message) = (self.broadcasts.clone(), room.clone(), message.clone());
        self.db
            .read(move |conn| {
                broadcasts.message_create(conn, &room, &message, &FakePartials, &BasicRichText)
            })
            .await
            .unwrap();
    }

    pub async fn boost(&self, label: &str) -> Boost {
        let boost_id = id(label);
        self.db
            .read(move |conn| Boost::find(conn, boost_id))
            .await
            .unwrap()
    }
}

fn read_leases(
    conn: &campfire_db::Connection,
    after_listing: impl FnOnce(),
) -> campfire_db::Result<Vec<WorkspacePresenceLease>> {
    // Keep enumeration and lookup in the same WAL snapshot while unsubscribe deletes rows.
    let snapshot = conn.unchecked_transaction()?;
    let conn = &snapshot;
    let ids: Vec<i64> = conn
        .prepare(r#"SELECT "id" FROM "workspace_presence_leases" ORDER BY "id""#)?
        .query_map([], |row| row.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    after_listing();
    ids.into_iter()
        .map(|id| Ok(WorkspacePresenceLease::find_by_id(conn, id)?.expect("listed")))
        .collect()
}

#[tokio::test]
async fn lease_inspection_retains_its_snapshot_during_unsubscribe() {
    let app = start().await;
    let session = app.session_for("kevin", true).await;
    let lease = app
        .db
        .write(move |tx| WorkspacePresenceLease::establish(tx, id("kevin"), session.id))
        .await
        .unwrap()
        .unwrap();
    let (listed, listing) = tokio::sync::oneshot::channel();
    let (resume, resumed) = tokio::sync::oneshot::channel();
    let db = app.db.clone();
    let read = tokio::spawn(async move {
        db.read(move |conn| {
            read_leases(conn, || {
                listed.send(()).unwrap();
                resumed.blocking_recv().unwrap();
            })
        })
        .await
        .unwrap()
    });
    wait("lease snapshot enumeration", listing).await.unwrap();
    // Commit a real WAL write between enumeration and row lookup, as unsubscribe does.
    assert_eq!(
        app.sql(
            "DELETE FROM workspace_presence_leases WHERE id = ?",
            vec![lease.id.into()]
        )
        .await,
        1
    );
    resume.send(()).unwrap();
    let snapshot = wait("lease snapshot lookup", read).await.unwrap();
    assert_eq!(snapshot.len(), 1);
    assert_eq!(snapshot[0].id, lease.id);
    assert_eq!(snapshot[0].connection_id, lease.connection_id);
    assert!(app.leases().await.is_empty());
}

/// Stand-in partials that name what they render, so frames show which partial and record.
pub struct FakePartials;

impl Partials for FakePartials {
    fn message(&self, message: &Message) -> String {
        format!(
            r#"<div id="message_{}">message {}</div>"#,
            message.client_message_id, message.id
        )
    }
    fn message_presentation(&self, message: &Message) -> String {
        format!("<div>presentation {} & more</div>", message.id)
    }
    fn boost(&self, boost: &Boost) -> String {
        format!("<div>boost {}</div>", boost.id)
    }
    fn shared_room(&self, room: &Room) -> String {
        format!("<li>shared {}</li>", room.id)
    }
    fn direct_room(&self, membership: &Membership) -> String {
        format!("<li>direct {}</li>", membership.id)
    }
    fn sidebar_row(&self, room: &Room, _membership: &Membership, unread: Option<bool>) -> String {
        format!("<li>row {} unread {unread:?}</li>", room.id)
    }
}

pub fn identifier(value: Value) -> String {
    value.to_string()
}

pub fn room_identifier(channel: &str, room_id: i64) -> String {
    identifier(json!({ "channel": channel, "room_id": room_id }))
}

pub fn confirmation(identifier: &str) -> String {
    json!({ "identifier": identifier, "type": "confirm_subscription" }).to_string()
}

pub fn rejection(identifier: &str) -> String {
    json!({ "identifier": identifier, "type": "reject_subscription" }).to_string()
}

/// The frame a broadcast of `message` (already ActiveSupport-JSON-encoded) arrives in.
pub fn delivery(identifier: &str, encoded_message: &str) -> String {
    format!(
        r#"{{"identifier":{},"message":{}}}"#,
        campfire_cable::json::encode(identifier),
        encoded_message
    )
}

pub struct Client {
    pub socket: WebSocketStream<MaybeTlsStream<TcpStream>>,
}

#[derive(Debug, PartialEq)]
pub enum Frame {
    Text(String),
    Close,
    End,
}

impl Client {
    pub async fn send(&mut self, command: Value) {
        wait("cable command send", self.socket.send(WsMessage::Text(command.to_string().into())))
            .await
            .unwrap();
    }

    pub async fn subscribe(&mut self, identifier: &str) {
        self.send(json!({ "command": "subscribe", "identifier": identifier }))
            .await;
    }

    /// Subscribes and returns the confirm or reject frame.
    pub async fn subscribe_reply(&mut self, identifier: &str) -> String {
        self.subscribe(identifier).await;
        self.next_text().await
    }

    pub async fn confirm(&mut self, identifier: &str) {
        assert_eq!(
            self.subscribe_reply(identifier).await,
            confirmation(identifier),
            "subscribing to {identifier}"
        );
    }

    pub async fn reject(&mut self, identifier: &str) {
        assert_eq!(
            self.subscribe_reply(identifier).await,
            rejection(identifier),
            "subscribing to {identifier}"
        );
    }

    pub async fn unsubscribe(&mut self, identifier: &str) {
        self.send(json!({ "command": "unsubscribe", "identifier": identifier }))
            .await;
    }

    pub async fn perform(&mut self, identifier: &str, data: Value) {
        self.send(
            json!({ "command": "message", "identifier": identifier, "data": data.to_string() }),
        )
        .await;
    }

    /// The next frame, skipping pings.
    pub async fn next(&mut self) -> Frame {
        self.next_before(tokio::time::Instant::now() + WAIT)
            .await
            .unwrap_or_else(|_| panic!("no non-ping WebSocket frame within {WAIT:?}"))
    }

    async fn next_before(
        &mut self,
        deadline: tokio::time::Instant,
    ) -> Result<Frame, tokio::time::error::Elapsed> {
        loop {
            let message = tokio::time::timeout_at(deadline, self.socket.next()).await?;
            return Ok(match message {
                Some(Ok(WsMessage::Text(text))) if text.starts_with(r#"{"type":"ping""#) => {
                    continue;
                }
                Some(Ok(WsMessage::Text(text))) => Frame::Text(text.to_string()),
                Some(Ok(WsMessage::Close(_))) => Frame::Close,
                Some(Ok(_)) => continue,
                Some(Err(_)) | None => Frame::End,
            });
        }
    }

    pub async fn next_text(&mut self) -> String {
        match self.next().await {
            Frame::Text(text) => text,
            other => panic!("expected a text frame, got {other:?}"),
        }
    }

    /// Asserts nothing arrives (pings aside) for a moment.
    pub async fn assert_silent(&mut self) {
        let result = self.next_before(tokio::time::Instant::now() + Duration::from_millis(250)).await;
        assert!(result.is_err(), "expected no frame, got {result:?}");
    }

    /// Reads until the closing handshake finishes, with one deadline across all frames/pings.
    pub async fn until_closed(&mut self) -> Vec<String> {
        self.until_closed_before(tokio::time::Instant::now() + WAIT)
            .await
            .unwrap_or_else(|error| panic!("{error}"))
    }

    async fn until_closed_before(&mut self, deadline: tokio::time::Instant) -> Result<Vec<String>, String> {
        let mut frames = Vec::new();
        loop {
            let frame = self.next_before(deadline).await.map_err(|_| {
                format!("the socket is still open at the close deadline; frames so far: {frames:?}")
            })?;
            match frame {
                Frame::Text(text) => frames.push(text),
                // Reading on flushes the client's close reply and lets the server unsubscribe.
                Frame::Close => {}
                Frame::End => return Ok(frames),
            }
        }
    }

    /// Independent subscription streams can arrive in either order. Compare every full frame,
    /// preserving multiplicity, with a single deadline for the whole set.
    pub async fn assert_texts(&mut self, expected: &[String]) {
        let deadline = tokio::time::Instant::now() + WAIT;
        let mut frames = Vec::new();
        for _ in expected {
            match self.next_before(deadline).await {
                Ok(Frame::Text(text)) => frames.push(text),
                other => panic!("waiting for frames {expected:?}; received {frames:?}; next: {other:?}"),
            }
        }
        frames.sort();
        let mut expected = expected.to_vec();
        expected.sort();
        assert_eq!(frames, expected, "broadcast deliveries");
    }
}

/// A JSON string literal the way ActiveSupport encodes HTML in it: quotes and backslashes
/// escaped, and `<`, `>`, `&` as `<`, `>`, `&`.
pub fn html_json(html: &str) -> String {
    let backslash = '\\';
    let escaped = html
        .replace(backslash, &format!("{backslash}{backslash}"))
        .replace('"', &format!("{backslash}\""))
        .replace('<', &format!("{backslash}u003c"))
        .replace('>', &format!("{backslash}u003e"))
        .replace('&', &format!("{backslash}u0026"));
    format!("\"{escaped}\"")
}

/// Exercise the same close waiter with frequent real socket pings, without catching a panic.
#[tokio::test]
async fn until_closed_bounds_a_socket_that_keeps_pinging() {
    let listener = bind_listener().await;
    let addr = listener.local_addr().unwrap();
    let (pinged, mut pings) = tokio::sync::mpsc::unbounded_channel();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut socket = tokio_tungstenite::accept_async(stream).await.unwrap();
        loop {
            if socket.send(WsMessage::Text(r#"{"type":"ping"}"#.into())).await.is_err() {
                break;
            }
            let _ = pinged.send(());
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    });
    let (socket, _) = wait("pinging WebSocket upgrade", tokio_tungstenite::connect_async(format!("ws://{addr}"))).await.unwrap();
    let mut client = Client { socket };
    // Make sure this is a pinging socket before measuring the close wait.
    for _ in 0..2 {
        wait("server ping", pings.recv()).await.unwrap();
    }
    let result = wait("the close waiter's overall deadline", client.until_closed_before(tokio::time::Instant::now() + Duration::from_millis(100))).await;
    let error = result.expect_err("the open socket must fail");
    assert!(error.contains("the socket is still open"), "unexpected failure: {error}");
    server.abort();
    assert!(wait("pinging server cancellation", server).await.unwrap_err().is_cancelled());
}
