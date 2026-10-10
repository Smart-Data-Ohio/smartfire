//! The `/api/v1/sync` socket over real WebSockets: hello/welcome, topic authorization, batching,
//! resume, typing coalescing, and the ways a connection ends.
mod support;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use campfire_cable::sync::{Audience, SyncConfig, SyncHandler, SyncPublication, SyncSession};
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use support::{TestServer, User, bind_listener, start, test_config};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;

type Log = Arc<Mutex<Vec<String>>>;

/// Members of room 1 only (like the support app's users); logs what sessions are told.
struct Handler(Log);

struct Session {
    user: Arc<User>,
    log: Log,
}

#[async_trait::async_trait]
impl SyncHandler<User> for Handler {
    fn user_id(&self, user: &User) -> i64 {
        user.id as i64
    }

    async fn open(&self, user: Arc<User>) -> Box<dyn SyncSession> {
        self.0.lock().unwrap().push(format!("open {}", user.id));
        Box::new(Session {
            user,
            log: self.0.clone(),
        })
    }
}

#[async_trait::async_trait]
impl SyncSession for Session {
    async fn authorize(&mut self, topic: &str) -> bool {
        topic
            .strip_prefix("room:")
            .and_then(|id| id.parse().ok())
            .is_some_and(|id| self.user.room_ids.contains(&id))
    }
    async fn typing(&mut self, conversation: &str, on: bool) {
        self.log
            .lock()
            .unwrap()
            .push(format!("typing {} {conversation} {on}", self.user.id));
    }
    async fn present(&mut self, room_id: i64) {
        self.log
            .lock()
            .unwrap()
            .push(format!("present {} {room_id}", self.user.id));
    }
    async fn absent(&mut self, room_id: i64) {
        self.log
            .lock()
            .unwrap()
            .push(format!("absent {} {room_id}", self.user.id));
    }
    /// A test ends the session by logging `expire <id>`.
    async fn heartbeat(&mut self, active: bool) -> bool {
        let mut log = self.log.lock().unwrap();
        log.push(format!("hb {} {active}", self.user.id));
        !log.contains(&format!("expire {}", self.user.id))
    }
    async fn close(&mut self) {
        self.log
            .lock()
            .unwrap()
            .push(format!("close {}", self.user.id));
    }
}

struct App {
    cable: TestServer,
    url: String,
    log: Log,
}

async fn app(config: SyncConfig) -> App {
    let cable = start(test_config()).await;
    let log = Log::default();
    cable.server.install_sync(Handler(log.clone()), config);
    let listener = bind_listener().await;
    let addr = listener.local_addr().unwrap();
    let router = cable.server.sync_router::<()>("/api/v1/sync");
    tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    App {
        url: format!("ws://{addr}/api/v1/sync"),
        cable,
        log,
    }
}

fn fast() -> SyncConfig {
    SyncConfig {
        flush_interval: Duration::from_millis(5),
        ..SyncConfig::default()
    }
}

struct Client(support::Socket);

impl App {
    async fn connect(&self, user_id: Option<u64>) -> Client {
        let mut request = self.url.as_str().into_client_request().unwrap();
        let origin = self
            .url
            .replace("ws://", "http://")
            .replace("/api/v1/sync", "");
        request
            .headers_mut()
            .insert("origin", HeaderValue::from_str(&origin).unwrap());
        if let Some(id) = user_id {
            request.headers_mut().insert(
                "cookie",
                HeaderValue::from_str(&format!("session_token={id}")).unwrap(),
            );
        }
        Client(
            tokio_tungstenite::connect_async(request)
                .await
                .expect("upgrade")
                .0,
        )
    }

    /// Connects, says hello, and returns the client with its welcome.
    async fn hello(&self, user_id: u64, resume: Value, topics: &[&str]) -> (Client, Value) {
        let mut client = self.connect(Some(user_id)).await;
        client
            .send(json!({ "t": "hello", "v": 1, "resume": resume, "topics": topics }))
            .await;
        let welcome = client.next().await;
        assert_eq!(welcome["t"], "welcome", "{welcome}");
        (client, welcome)
    }

    fn publish(&self, audience: Audience, data: Value) -> u64 {
        let payload = json!({ "type": "test", "data": data }).to_string();
        self.cable
            .server
            .sync_publish(SyncPublication::new(audience, payload))
            .unwrap()
    }

    fn logged(&self) -> Vec<String> {
        self.log.lock().unwrap().clone()
    }
}

impl Client {
    async fn send(&mut self, frame: Value) {
        self.0
            .send(Message::Text(frame.to_string().into()))
            .await
            .unwrap();
    }

    /// The next text frame, skipping pings.
    async fn next(&mut self) -> Value {
        loop {
            let message = tokio::time::timeout(Duration::from_secs(5), self.0.next())
                .await
                .expect("frame within 5s");
            match message {
                Some(Ok(Message::Text(text))) => {
                    let frame: Value = serde_json::from_str(&text).unwrap();
                    if frame["t"] != "ping" {
                        return frame;
                    }
                }
                other => panic!("expected a text frame, got {other:?}"),
            }
        }
    }

    /// The next batch's events as (seq, topic, data).
    async fn batch(&mut self) -> Vec<(u64, String, Value)> {
        let frame = self.next().await;
        assert_eq!(frame["t"], "batch", "{frame}");
        frame["events"]
            .as_array()
            .unwrap()
            .iter()
            .map(|event| {
                (
                    event["seq"].as_u64().unwrap(),
                    event["topic"].as_str().unwrap().to_string(),
                    event["data"].clone(),
                )
            })
            .collect()
    }

    async fn closed(&mut self) {
        loop {
            match tokio::time::timeout(Duration::from_secs(5), self.0.next())
                .await
                .expect("close within 5s")
            {
                // Reading on after the close frame sends tungstenite's reply.
                Some(Ok(Message::Close(_))) => {}
                None | Some(Err(_)) => return,
                Some(Ok(Message::Text(text))) => panic!("unexpected frame after bye: {text}"),
                Some(Ok(_)) => {}
            }
        }
    }
}

#[tokio::test]
async fn delivers_the_users_and_authorized_topics_events_in_batches() {
    let app = app(fast()).await;
    let (mut client, welcome) = app
        .hello(1, Value::Null, &["room:1", "room:2", "user", "bogus"])
        .await;
    assert_eq!(welcome["resumed"], false);
    assert_eq!(welcome["seq"], 0);
    assert_eq!(welcome["epoch"].as_str().unwrap().len(), 24);

    // Room 2 was refused, and someone else's user topic isn't ours.
    let first = app.publish(Audience::Topic("room:2".into()), json!("other room"));
    app.publish(Audience::User(2), json!("not mine"));
    let mine = app.publish(Audience::Topic("room:1".into()), json!("room"));
    let everyone = app.publish(Audience::Everyone, json!("everyone"));
    let user = app.publish(Audience::User(1), json!("mine"));
    assert!(first < mine);

    let mut events = Vec::new();
    while events.len() < 3 {
        events.extend(client.batch().await);
    }
    assert_eq!(
        events,
        [
            (mine, "room:1".into(), json!("room")),
            (everyone, "user".into(), json!("everyone")),
            (user, "user".into(), json!("mine")),
        ]
    );
}

#[tokio::test]
async fn a_refused_subscription_gets_nothing_from_that_topic() {
    let app = app(fast()).await;
    let (mut client, _) = app.hello(1, Value::Null, &[]).await;
    client
        .send(json!({ "t": "sub", "topics": ["room:2", "room:1"] }))
        .await;
    client.send(json!({ "t": "hb", "active": true })).await;
    // The heartbeat is handled after the subscriptions, so they're in place once it's logged.
    wait_for(&app, "hb 1 true").await;
    app.publish(Audience::Topic("room:2".into()), json!("secret"));
    let open = app.publish(Audience::Topic("room:1".into()), json!("open"));
    assert_eq!(
        client.batch().await,
        [(open, "room:1".into(), json!("open"))]
    );

    client
        .send(json!({ "t": "unsub", "topics": ["room:1"] }))
        .await;
    client.send(json!({ "t": "hb", "active": false })).await;
    wait_for(&app, "hb 1 false").await;
    app.publish(Audience::Topic("room:1".into()), json!("gone"));
    let user = app.publish(Audience::User(1), json!("still here"));
    assert_eq!(
        client.batch().await,
        [(user, "user".into(), json!("still here"))]
    );
}

#[tokio::test]
async fn resumes_within_the_ring_and_starts_over_otherwise() {
    let app = app(SyncConfig {
        ring_capacity: 3,
        ..fast()
    })
    .await;
    let (mut client, welcome) = app.hello(1, Value::Null, &["room:1"]).await;
    let epoch = welcome["epoch"].as_str().unwrap().to_string();
    let a = app.publish(Audience::Topic("room:1".into()), json!("a"));
    assert_eq!(client.batch().await, [(a, "room:1".into(), json!("a"))]);
    drop(client);

    // Missed while away: replayed in order after a resumed welcome.
    let b = app.publish(Audience::Topic("room:1".into()), json!("b"));
    // Typing is stale by the time anyone resumes.
    app.cable.server.sync_publish(SyncPublication {
        ephemeral: true,
        ..SyncPublication::new(
            Audience::Topic("room:1".into()),
            json!({ "type": "typing", "data": "stale" }).to_string(),
        )
    });
    let c = app.publish(Audience::User(1), json!("c"));
    let (mut client, welcome) = app
        .hello(1, json!({ "epoch": epoch, "seq": a }), &["room:1"])
        .await;
    assert_eq!(welcome["resumed"], true);
    assert_eq!(welcome["seq"], a);
    assert_eq!(welcome["replayThrough"], c);
    assert_eq!(
        client.batch().await,
        [
            (b, "room:1".into(), json!("b")),
            (c, "user".into(), json!("c"))
        ]
    );
    drop(client);

    // Another epoch (another boot) starts at the head, with nothing replayed.
    let (mut client, welcome) = app
        .hello(1, json!({ "epoch": "0123", "seq": a }), &["room:1"])
        .await;
    assert_eq!(welcome["resumed"], false);
    assert_eq!(welcome["seq"], c);
    let d = app.publish(Audience::User(1), json!("d"));
    assert_eq!(client.batch().await, [(d, "user".into(), json!("d"))]);
    drop(client);

    // The ring (capacity 3) has rolled past `a`: the client must refetch.
    for n in 0..3 {
        app.publish(Audience::User(1), json!(n));
    }
    let (_client, welcome) = app
        .hello(1, json!({ "epoch": epoch, "seq": a }), &["room:1"])
        .await;
    assert_eq!(welcome["resumed"], false);
}

#[tokio::test]
async fn coalesces_typing_and_never_echoes_it() {
    let app = app(SyncConfig {
        flush_interval: Duration::from_millis(300),
        ..SyncConfig::default()
    })
    .await;
    let (mut watcher, _) = app.hello(2, Value::Null, &["room:1"]).await;
    let (mut typist, _) = app.hello(1, Value::Null, &["room:1"]).await;
    let typing = |on: bool| SyncPublication {
        except_user: Some(1),
        coalesce: Some("typing:room:1:1".into()),
        ..SyncPublication::new(
            Audience::Topic("room:1".into()),
            json!({ "type": "typing", "data": { "userId": 1, "on": on } }).to_string(),
        )
    };
    app.cable.server.sync_publish(typing(true));
    let last = app.cable.server.sync_publish(typing(false)).unwrap();
    let marker = app.publish(Audience::User(1), json!("marker"));

    let frame = watcher.next().await;
    let events = frame["events"].as_array().unwrap();
    assert_eq!(events.len(), 1, "only the latest typing state: {frame}");
    assert_eq!(events[0]["seq"], last);
    assert_eq!(events[0]["data"], json!({ "userId": 1, "on": false }));
    assert_eq!(
        typist.batch().await,
        [(marker, "user".into(), json!("marker"))]
    );

    // Typing frames reach the session only for followed conversations.
    typist
        .send(json!({ "t": "typing", "conv": "room:2", "on": true }))
        .await;
    typist
        .send(json!({ "t": "typing", "conv": "room:1", "on": true }))
        .await;
    typist.send(json!({ "t": "present", "room": 1 })).await;
    typist.send(json!({ "t": "absent", "room": 1 })).await;
    wait_for(&app, "absent 1 1").await;
    let logged = app.logged();
    assert!(
        logged.contains(&"typing 1 room:1 true".to_string()),
        "{logged:?}"
    );
    assert!(
        !logged.iter().any(|line| line.contains("room:2")),
        "{logged:?}"
    );
    assert!(logged.contains(&"present 1 1".to_string()), "{logged:?}");
}

#[tokio::test]
async fn an_unsubscribe_publication_stops_the_topic() {
    let app = app(fast()).await;
    let (mut client, _) = app.hello(1, Value::Null, &["room:1"]).await;
    let removed = app.cable.server.sync_publish(SyncPublication {
        unsubscribe: Some("room:1".into()),
        ..SyncPublication::new(
            Audience::User(1),
            json!({ "type": "test", "data": "removed" }).to_string(),
        )
    });
    assert_eq!(
        client.batch().await,
        [(removed.unwrap(), "user".into(), json!("removed"))]
    );
    app.publish(Audience::Topic("room:1".into()), json!("after removal"));
    let user = app.publish(Audience::User(1), json!("user"));
    assert_eq!(client.batch().await, [(user, "user".into(), json!("user"))]);
}

#[tokio::test]
async fn a_publication_read_before_a_fencing_event_is_refused() {
    let app = app(fast()).await;
    let (mut client, _) = app.hello(1, Value::Null, &[]).await;
    let server = &app.cable.server;
    let event = |data: &str| {
        SyncPublication::new(
            Audience::User(1),
            json!({ "type": "test", "data": data }).to_string(),
        )
    };
    let read = server.sync_head();
    let fence = server
        .sync_publish_fencing(event("read"), "unread:1:7".into())
        .unwrap();
    // Read before the fencing event: refused, and nothing is sent.
    assert_eq!(
        server.sync_publish_fresh(event("stale row"), "unread:1:7", read),
        Err(campfire_cable::sync::Stale)
    );
    // Another key isn't fenced, and a fresh read goes out.
    let other = server
        .sync_publish_fresh(event("other room"), "unread:1:8", read)
        .unwrap()
        .unwrap();
    let fresh = server
        .sync_publish_fresh(event("fresh row"), "unread:1:7", server.sync_head())
        .unwrap()
        .unwrap();
    assert_eq!(
        client.batch().await,
        [
            (fence, "user".into(), json!("read")),
            (other, "user".into(), json!("other room")),
            (fresh, "user".into(), json!("fresh row")),
        ]
    );
}

#[tokio::test]
async fn a_remote_disconnect_says_bye_and_closes() {
    let app = app(fast()).await;
    let (mut client, _) = app.hello(1, Value::Null, &[]).await;
    let (mut other, _) = app.hello(2, Value::Null, &[]).await;
    assert!(app.cable.server.disconnect("user-1", false) >= 1);
    assert_eq!(
        client.next().await,
        json!({ "t": "bye", "reconnect": false, "reason": "remote" })
    );
    client.closed().await;
    wait_for(&app, "close 1").await;

    // Others stay connected.
    let user = app.publish(Audience::User(2), json!("still"));
    assert_eq!(other.batch().await, [(user, "user".into(), json!("still"))]);

    app.cable.server.restart();
    assert_eq!(
        other.next().await,
        json!({ "t": "bye", "reconnect": true, "reason": "server_restart" })
    );
    other.closed().await;
}

#[tokio::test]
async fn a_remote_disconnect_sends_nothing_published_after_it() {
    let app = app(fast()).await;
    let (mut client, _) = app.hello(1, Value::Null, &[]).await;
    // Disconnected, then published at once (a revocation, then a post the revoked person can no
    // longer read): the bye comes first, and nothing follows it. A resume picks the event up
    // only if it's still theirs to read.
    for round in 0..20 {
        assert!(app.cable.server.disconnect("user-1", true) >= 1);
        app.publish(Audience::User(1), json!(round));
        assert_eq!(
            client.next().await,
            json!({ "t": "bye", "reconnect": true, "reason": "remote" })
        );
        client.closed().await;
        (client, _) = app.hello(1, Value::Null, &[]).await;
    }
}

#[tokio::test]
async fn a_remote_disconnect_sends_what_came_before_it_and_nothing_after() {
    let app = app(SyncConfig {
        flush_interval: Duration::from_secs(60),
        ..SyncConfig::default()
    })
    .await;
    let (mut client, _) = app.hello(1, Value::Null, &[]).await;
    // All three land before the connection's task runs again (this runtime has one thread), so
    // it sees the disconnect with the earlier event still unread in the ring.
    let before = app.publish(Audience::User(1), json!("before"));
    assert!(app.cable.server.disconnect("user-1", false) >= 1);
    app.publish(Audience::User(1), json!("after"));
    assert_eq!(
        client.batch().await,
        [(before, "user".into(), json!("before"))]
    );
    assert_eq!(
        client.next().await,
        json!({ "t": "bye", "reconnect": false, "reason": "remote" })
    );
    client.closed().await;
}

#[tokio::test]
async fn a_heartbeat_after_the_session_ended_says_bye_and_closes() {
    let app = app(fast()).await;
    let (mut client, _) = app.hello(1, Value::Null, &[]).await;
    app.log.lock().unwrap().push("expire 1".into());
    client.send(json!({ "t": "hb", "active": true })).await;
    assert_eq!(
        client.next().await,
        json!({ "t": "bye", "reconnect": false, "reason": "session_expired" })
    );
    client.closed().await;
    wait_for(&app, "close 1").await;
}

#[tokio::test]
async fn nothing_is_wanted_without_a_socket_and_the_gap_ends_resumes() {
    let app = app(fast()).await;
    let (mut client, welcome) = app.hello(1, Value::Null, &[]).await;
    let epoch = welcome["epoch"].as_str().unwrap().to_string();
    assert!(app.cable.server.sync_wanted());
    let seen = app.publish(Audience::User(1), json!("seen"));
    assert_eq!(client.batch().await, [(seen, "user".into(), json!("seen"))]);
    drop(client);
    wait_for(&app, "close 1").await;
    // Nobody is connected: the broadcast points skip building events, and the ring notes it.
    let mut waits = 0;
    while app.cable.server.sync_wanted() {
        waits += 1;
        assert!(waits < 500, "the closed socket is still counted");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let (client, welcome) = app
        .hello(1, json!({ "epoch": epoch, "seq": seen }), &[])
        .await;
    assert_eq!(
        welcome["resumed"], false,
        "the skipped event can't be replayed: {welcome}"
    );

    // The second gap: welcomed at the first gap's sequence with nothing kept, the client drops,
    // and an event goes unbuilt again. Resuming at the welcome's sequence must not skip past it.
    let welcomed = welcome["seq"].as_i64().unwrap();
    drop(client);
    let mut waits = 0;
    while app.cable.server.sync_wanted() {
        waits += 1;
        assert!(waits < 500, "the closed socket is still counted");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let (_client, welcome) = app
        .hello(1, json!({ "epoch": epoch, "seq": welcomed }), &[])
        .await;
    assert_eq!(
        welcome["resumed"], false,
        "the second skipped event can't be replayed either: {welcome}"
    );
    assert!(welcome["seq"].as_i64().unwrap() > welcomed);
}

#[tokio::test]
async fn a_skip_for_someone_without_a_socket_ends_their_resume_only() {
    let app = app(fast()).await;
    // Person 2 keeps a socket open throughout, so events are wanted.
    let (mut other, _) = app.hello(2, Value::Null, &[]).await;
    let (client, welcome) = app.hello(1, Value::Null, &[]).await;
    let epoch = welcome["epoch"].as_str().unwrap().to_string();
    assert!(app.cable.server.sync_connected(1));
    drop(client);
    wait_for(&app, "close 1").await;
    let mut waits = 0;
    while app.cable.server.sync_connected(1) {
        waits += 1;
        assert!(waits < 500, "the closed socket is still counted");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(app.cable.server.sync_connected(2));
    let before = app.publish(Audience::User(2), json!("before"));
    assert_eq!(
        other.batch().await,
        [(before, "user".into(), json!("before"))]
    );

    // Two skips for person 1 leave one marker; person 2 doesn't see it.
    app.cable.server.sync_skipped_for(1);
    app.cable.server.sync_skipped_for(1);
    let after = app.publish(Audience::User(2), json!("after"));
    assert_eq!(after, before + 2, "one marker for both skips");
    assert_eq!(
        other.batch().await,
        [(after, "user".into(), json!("after"))]
    );

    // Person 1 resuming from before the marker starts afresh.
    let (client, welcome) = app
        .hello(1, json!({ "epoch": epoch, "seq": before }), &[])
        .await;
    assert_eq!(welcome["resumed"], false, "{welcome}");
    assert_eq!(welcome["seq"], after);
    drop(client);
    wait_for_closes(&app, 2).await;

    // The marker was cleared when that socket opened: the next skip leaves another.
    app.cable.server.sync_skipped_for(1);
    let last = app.publish(Audience::User(2), json!("last"));
    assert_eq!(last, after + 2);
    let (_client, welcome) = app
        .hello(1, json!({ "epoch": epoch, "seq": after }), &[])
        .await;
    assert_eq!(welcome["resumed"], false, "{welcome}");
}

#[tokio::test]
async fn every_skip_after_a_live_resync_resyncs_again() {
    let app = app(fast()).await;
    let (mut client, _) = app.hello(1, Value::Null, &[]).await;
    let resync = json!({ "t": "resync", "topics": ["user"], "reason": "skipped" });
    for _ in 0..3 {
        // A skip while the socket is open (an event that couldn't be built in time): each one
        // after the last resync is a resync of its own, on the same socket.
        app.cable.server.sync_skipped_for(1);
        assert_eq!(client.next().await, resync);
    }
    let after = app.publish(Audience::User(1), json!("after"));
    assert_eq!(
        client.batch().await,
        [(after, "user".into(), json!("after"))]
    );
}

/// Waits until person 1's sockets have closed `count` times in all.
async fn wait_for_closes(app: &App, count: usize) {
    let mut waits = 0;
    while app
        .logged()
        .iter()
        .filter(|line| *line == "close 1")
        .count()
        < count
        || app.cable.server.sync_connected(1)
    {
        waits += 1;
        assert!(waits < 500, "the socket didn't close");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

#[tokio::test]
async fn unauthenticated_and_unsupported_clients_are_turned_away() {
    let app = app(fast()).await;
    let mut anonymous = app.connect(None).await;
    assert_eq!(
        anonymous.next().await,
        json!({ "t": "bye", "reconnect": false, "reason": "unauthorized" })
    );
    anonymous.closed().await;

    let mut future = app.connect(Some(1)).await;
    future
        .send(json!({ "t": "hello", "v": 2, "resume": null, "topics": [] }))
        .await;
    assert_eq!(
        future.next().await,
        json!({ "t": "bye", "reconnect": false, "reason": "unsupported_version" })
    );
    future.closed().await;
}

#[tokio::test]
async fn pings_an_idle_connection() {
    let app = app(SyncConfig {
        ping_after: Duration::from_millis(50),
        ..fast()
    })
    .await;
    let (mut client, _) = app.hello(1, Value::Null, &[]).await;
    let message = tokio::time::timeout(Duration::from_secs(5), client.0.next())
        .await
        .unwrap();
    let Some(Ok(Message::Text(text))) = message else {
        panic!("{message:?}")
    };
    assert_eq!(text.as_str(), r#"{"t":"ping"}"#);
}

#[tokio::test]
async fn publishing_without_the_engine_does_nothing() {
    let cable = start(test_config()).await;
    assert!(!cable.server.sync_enabled());
    assert_eq!(
        cable
            .server
            .sync_publish(SyncPublication::new(Audience::Everyone, "{}".into())),
        None
    );
}

async fn wait_for(app: &App, line: &str) {
    for _ in 0..500 {
        if app.logged().iter().any(|logged| logged == line) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("never logged {line:?}: {:?}", app.logged());
}
