//! One `/api/v1/sync` socket: authenticate, wait for `hello`, then deliver ring events in
//! batches until the client goes, the person is disconnected (sign-out, ban) or the server
//! restarts.
use std::collections::BTreeSet;
use std::sync::Arc;

use campfire_api_types::{ClientFrame, ServerFrame};
use futures_util::StreamExt;
use futures_util::stream::{BoxStream, SelectAll};
use tokio::io::WriteHalf;
use tokio::sync::mpsc;
use tokio::time::{Instant, sleep_until};

use super::{Engine, Entry, Read, USER_TOPIC};
use crate::connection::{Io, close_socket, process_internal_reconnect, spawn_reader};
use crate::pubsub::{Delivery, Frame, RecvError};
use crate::server::{ConnectRequest, Identified, internal_channel};
use crate::socket::{Incoming, Writer};
use crate::Server;

type Sink = Writer<WriteHalf<Io>>;

/// What ends a connection, once decided.
struct Bye {
    reconnect: bool,
    reason: &'static str,
}

pub(crate) async fn run<U: Identified + Send + Sync + 'static>(
    server: Server<U>,
    engine: Arc<Engine<U>>,
    io: Io,
    deflate: bool,
    request: ConnectRequest,
) {
    let (read, write) = tokio::io::split(io);
    let mut sink = Writer::new(write, deflate);
    let (reader, mut incoming) = spawn_reader(crate::socket::Reader::new(read, deflate));
    let config = engine.ring.config().clone();
    let close_timeout = server.config().close_timeout;

    let unauthorized = Bye { reconnect: false, reason: "unauthorized" };
    let Some(user) = server.authenticator().connect(&request).await else {
        say_bye(&mut sink, &mut incoming, unauthorized, close_timeout).await;
        return reader.abort();
    };
    let mut internal = SelectAll::<BoxStream<'static, Result<Delivery, RecvError>>>::new();
    let identifier = user.connection_identifier();
    if !identifier.is_empty() {
        internal.push(server.hub().subscribe(&internal_channel(&identifier), None).ordered_deliveries());
        // A sign-out or ban between the check above and that subscription went unheard.
        if server.authenticator().connect(&request).await.is_none() {
            say_bye(&mut sink, &mut incoming, unauthorized, close_timeout).await;
            return reader.abort();
        }
    }
    drop(request);

    // `hello` comes first; nothing else is accepted before it.
    let hello_deadline = Instant::now() + config.hello_timeout;
    let (resume, topics) = loop {
        let message = tokio::select! {
            message = incoming.recv() => message,
            () = sleep_until(hello_deadline) => {
                let bye = Bye { reconnect: true, reason: "hello_timeout" };
                say_bye(&mut sink, &mut incoming, bye, close_timeout).await;
                return reader.abort();
            }
        };
        match message {
            Some(Incoming::Text(text)) => match serde_json::from_str::<ClientFrame>(&text) {
                Ok(ClientFrame::Hello { v: 1, resume, topics }) => break (resume, topics),
                Ok(ClientFrame::Hello { .. }) => {
                    let bye = Bye { reconnect: false, reason: "unsupported_version" };
                    say_bye(&mut sink, &mut incoming, bye, close_timeout).await;
                    return reader.abort();
                }
                _ => tracing::debug!("sync: frame before hello ignored"),
            },
            Some(Incoming::Ping(payload)) => {
                if sink.pong(&payload).await.is_err() {
                    return reader.abort();
                }
            }
            Some(Incoming::Binary | Incoming::Pong) => {}
            Some(Incoming::Close(code)) => {
                let _ = sink.close_reply(code).await;
                return reader.abort();
            }
            Some(Incoming::Invalid) => {
                let _ = sink.close(1002).await;
                return reader.abort();
            }
            None => return reader.abort(),
        }
    };

    let user = Arc::new(user);
    let user_id = engine.handler.user_id(&user);
    let session = engine.handler.open(user.clone()).await;
    let mut connection = Connection {
        engine: engine.clone(),
        session,
        user_id,
        topics: BTreeSet::new(),
        pending: Vec::new(),
        flush_at: None,
        cursor: 0,
        last_sent: Instant::now(),
    };
    connection.follow(topics).await;

    // Subscribe to the head before reading, so nothing published from here on goes unnoticed.
    let mut head = engine.ring.watch();
    head.mark_unchanged();
    let point = resume.as_ref().map(|point| (point.epoch.as_str(), point.seq));
    let (cursor, resumed) = engine.ring.resume(point);
    connection.cursor = cursor;
    let welcome = ServerFrame::Welcome { epoch: engine.ring.epoch().to_string(), seq: cursor as i64, resumed };
    let mut bye = None;
    if connection.send(&mut sink, &[encode(&welcome)]).await.is_err() {
        connection.session.close().await;
        return reader.abort();
    }
    // A resumed client gets what it missed straight away.
    if resumed {
        connection.catch_up(true);
        connection.flush_at = connection.flush_at.map(|_| Instant::now());
    }

    let mut restarts = server.restarts();
    let mut last_received = Instant::now();
    loop {
        let ping_at = connection.last_sent + config.ping_after;
        let flush_at = connection.flush_at;
        tokio::select! {
            biased;
            Some(message) = internal.next() => {
                if let Ok(message) = message
                    && let Some(reconnect) = process_internal_reconnect(message.frame.as_str())
                {
                    bye = Some(Bye { reconnect, reason: "remote" });
                }
            }
            Ok(()) = restarts.recv() => bye = Some(Bye { reconnect: true, reason: "server_restart" }),
            message = incoming.recv() => {
                last_received = Instant::now();
                match message {
                    Some(Incoming::Text(text)) => connection.dispatch(&text).await,
                    Some(Incoming::Ping(payload)) => {
                        if sink.pong(&payload).await.is_err() {
                            break;
                        }
                    }
                    Some(Incoming::Binary | Incoming::Pong) => {}
                    Some(Incoming::Close(code)) => {
                        let _ = sink.close_reply(code).await;
                        break;
                    }
                    Some(Incoming::Invalid) => {
                        let _ = sink.close(1002).await;
                        break;
                    }
                    None => break,
                }
            }
            Ok(()) = head.changed() => {
                head.mark_unchanged();
                if let Some(resync) = connection.catch_up(false) {
                    // Write what came before the gap, then ask for the refetch.
                    let frames = connection.take_batches();
                    if connection.send(&mut sink, &frames).await.is_err()
                        || connection.send(&mut sink, &[encode(&resync)]).await.is_err()
                    {
                        break;
                    }
                }
            }
            () = sleep_until(flush_at.unwrap_or_else(Instant::now)), if flush_at.is_some() => {}
            () = sleep_until(ping_at) => {
                if connection.send(&mut sink, &[encode(&ServerFrame::Ping)]).await.is_err() {
                    break;
                }
            }
            () = sleep_until(last_received + config.client_timeout) => {
                bye = Some(Bye { reconnect: true, reason: "timeout" });
            }
        }

        let due = connection.flush_at.is_some_and(|at| at <= Instant::now());
        if due || connection.pending.len() >= config.flush_max || bye.is_some() {
            let frames = connection.take_batches();
            if connection.send(&mut sink, &frames).await.is_err() {
                break;
            }
        }
        if let Some(bye) = bye.take() {
            say_bye(&mut sink, &mut incoming, bye, close_timeout).await;
            break;
        }
    }

    reader.abort();
    connection.session.close().await;
}

/// Sends `bye`, then closes the socket normally.
async fn say_bye(sink: &mut Sink, incoming: &mut mpsc::Receiver<Incoming>, bye: Bye, timeout: std::time::Duration) {
    let frame = encode(&ServerFrame::Bye { reconnect: bye.reconnect, reason: bye.reason.to_string() });
    let _ = sink.send(&[frame]).await;
    close_socket(sink, incoming, timeout).await;
}

fn encode(frame: &ServerFrame) -> Frame {
    serde_json::to_string(frame).expect("frames serialize").into()
}

struct Connection<U> {
    engine: Arc<Engine<U>>,
    session: Box<dyn super::SyncSession>,
    user_id: i64,
    /// The conversation topics followed, authorized when they were added.
    topics: BTreeSet<String>,
    pending: Vec<Arc<Entry>>,
    /// When the pending events must go out, once there are any.
    flush_at: Option<Instant>,
    /// The latest sequence read from the ring.
    cursor: u64,
    last_sent: Instant,
}

impl<U> Connection<U> {
    /// Reads the ring past the cursor into the pending batch (leaving out live-only events when
    /// `replay`ing for a resumed client). Returns the `resync` to send if the ring dropped events
    /// this connection hadn't read.
    fn catch_up(&mut self, replay: bool) -> Option<ServerFrame> {
        match self.engine.ring.read(self.cursor) {
            Read::Events(events, head) => {
                self.cursor = head;
                for entry in events {
                    if !(replay && entry.ephemeral) {
                        self.add(entry);
                    }
                }
                None
            }
            Read::Lost(head) => {
                self.cursor = head;
                let topics = std::iter::once(USER_TOPIC.to_string()).chain(self.topics.iter().cloned()).collect();
                Some(ServerFrame::Resync { topics, reason: "ring_rolled_over".into() })
            }
        }
    }

    fn add(&mut self, entry: Arc<Entry>) {
        if !entry.delivered_to(self.user_id, |topic| self.topics.contains(topic)) {
            return;
        }
        if let Some(topic) = &entry.unsubscribe {
            self.topics.remove(topic);
        }
        if let Some(key) = &entry.coalesce {
            self.pending.retain(|pending| pending.coalesce.as_ref() != Some(key));
        }
        self.pending.push(entry);
        if self.flush_at.is_none() {
            self.flush_at = Some(Instant::now() + self.engine.ring.config().flush_interval);
        }
    }

    /// The pending events as `batch` frames of at most `flush_max` events each.
    fn take_batches(&mut self) -> Vec<Frame> {
        self.flush_at = None;
        let max = self.engine.ring.config().flush_max.max(1);
        let frames = self
            .pending
            .chunks(max)
            .map(|events| {
                let mut frame = String::from(r#"{"t":"batch","events":["#);
                for (i, entry) in events.iter().enumerate() {
                    if i > 0 {
                        frame.push(',');
                    }
                    frame.push_str(&entry.json);
                }
                frame.push_str("]}");
                Frame::from(frame)
            })
            .collect();
        self.pending.clear();
        frames
    }

    async fn send(&mut self, sink: &mut Sink, frames: &[Frame]) -> std::io::Result<()> {
        if frames.is_empty() {
            return Ok(());
        }
        self.last_sent = Instant::now();
        sink.send(frames).await
    }

    /// Adds the topics this connection may follow; the rest are dropped.
    async fn follow(&mut self, topics: Vec<String>) {
        for topic in topics {
            if self.topics.contains(&topic) || !conversation_topic(&topic) {
                continue;
            }
            if self.topics.len() >= self.engine.ring.config().max_topics {
                tracing::warn!("sync: topic limit reached");
                break;
            }
            if self.session.authorize(&topic).await {
                self.topics.insert(topic);
            } else {
                tracing::info!(topic, "sync: subscription refused");
            }
        }
    }

    async fn dispatch(&mut self, text: &str) {
        let frame = match serde_json::from_str::<ClientFrame>(text) {
            Ok(frame) => frame,
            Err(error) => return tracing::debug!(%error, "sync: unreadable frame"),
        };
        match frame {
            ClientFrame::Hello { .. } => tracing::debug!("sync: repeated hello ignored"),
            ClientFrame::Sub { topics } => self.follow(topics).await,
            ClientFrame::Unsub { topics } => {
                for topic in topics {
                    self.topics.remove(&topic);
                }
            }
            ClientFrame::Typing { conv, on } => {
                if self.topics.contains(&conv) {
                    self.session.typing(&conv, on).await;
                }
            }
            ClientFrame::Present { room } => self.session.present(room).await,
            ClientFrame::Absent { room } => self.session.absent(room).await,
            ClientFrame::Hb { active } => self.session.heartbeat(active).await,
        }
    }
}

/// `room:<id>` or `thread:<id>`.
fn conversation_topic(topic: &str) -> bool {
    let id = topic.strip_prefix("room:").or_else(|| topic.strip_prefix("thread:"));
    id.is_some_and(|id| !id.is_empty() && id.len() <= 18 && id.bytes().all(|b| b.is_ascii_digit()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::{Audience, Ring, SyncConfig, SyncHandler, SyncPublication, SyncSession};

    struct Nobody;

    #[async_trait::async_trait]
    impl SyncSession for Nobody {
        async fn authorize(&mut self, _: &str) -> bool {
            true
        }
        async fn typing(&mut self, _: &str, _: bool) {}
        async fn present(&mut self, _: i64) {}
        async fn absent(&mut self, _: i64) {}
        async fn heartbeat(&mut self, _: bool) {}
        async fn close(&mut self) {}
    }

    #[async_trait::async_trait]
    impl SyncHandler<()> for Nobody {
        fn user_id(&self, _: &()) -> i64 {
            1
        }
        async fn open(&self, _: Arc<()>) -> Box<dyn SyncSession> {
            Box::new(Nobody)
        }
    }

    fn connection(config: SyncConfig) -> Connection<()> {
        Connection {
            engine: Arc::new(Engine { ring: Ring::new(config), handler: Arc::new(Nobody) }),
            session: Box::new(Nobody),
            user_id: 1,
            topics: ["room:1".to_string()].into(),
            pending: Vec::new(),
            flush_at: None,
            cursor: 0,
            last_sent: Instant::now(),
        }
    }

    fn publish(connection: &Connection<()>, seq: u64, audience: Audience) {
        let payload = format!(r#"{{"type":"room.read","data":{{"roomId":{seq}}}}}"#);
        connection.engine.ring.push(seq, SyncPublication::new(audience, payload));
    }

    #[test]
    fn a_connection_the_ring_rolled_past_is_told_to_resync() {
        let mut connection = connection(SyncConfig { ring_capacity: 2, ..SyncConfig::default() });
        publish(&connection, 1, Audience::User(1));
        assert!(connection.catch_up(false).is_none());
        assert_eq!(connection.pending.len(), 1);
        for seq in 2..=4 {
            publish(&connection, seq, Audience::Topic("room:1".into()));
        }
        // Seq 2 fell out before this connection read it.
        assert_eq!(
            connection.catch_up(false),
            Some(ServerFrame::Resync { topics: vec!["user".into(), "room:1".into()], reason: "ring_rolled_over".into() }),
        );
        assert_eq!(connection.cursor, 4);
        publish(&connection, 5, Audience::User(1));
        assert!(connection.catch_up(false).is_none());
        assert_eq!(connection.pending.iter().map(|entry| entry.seq).collect::<Vec<_>>(), [1, 5]);
    }

    #[test]
    fn batches_split_at_the_flush_size_and_keep_sequence_order() {
        let mut connection = connection(SyncConfig { flush_max: 2, ..SyncConfig::default() });
        for seq in 1..=5 {
            publish(&connection, seq, Audience::Everyone);
        }
        connection.catch_up(false);
        assert!(connection.flush_at.is_some());
        let frames = connection.take_batches();
        assert_eq!(frames.len(), 3);
        assert_eq!(
            frames[0].as_str(),
            concat!(
                r#"{"t":"batch","events":[{"seq":1,"topic":"user","type":"room.read","data":{"roomId":1}},"#,
                r#"{"seq":2,"topic":"user","type":"room.read","data":{"roomId":2}}]}"#,
            ),
        );
        assert!(connection.pending.is_empty() && connection.flush_at.is_none());
        for frame in &frames {
            serde_json::from_str::<ServerFrame>(frame.as_str()).expect("a contract frame");
        }
    }

    #[test]
    fn only_conversation_topics_can_be_followed() {
        assert!(conversation_topic("room:12"));
        assert!(conversation_topic("thread:3"));
        assert!(!conversation_topic("user"));
        assert!(!conversation_topic("room:"));
        assert!(!conversation_topic("room:1x"));
        assert!(!conversation_topic("room:-1"));
        assert!(!conversation_topic("chat:1"));
    }
}
