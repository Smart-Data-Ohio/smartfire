//! The single-page app's `/api/v1/sync` socket: a small JSON protocol beside Action Cable, fed by
//! the same hub (`frontend-plan.md` §2.5, frames in `campfire_api_types::sync`).
//!
//! Every event is published once into a bounded in-memory replay ring, under the hub lock that
//! assigns the global publication sequence, so ring order is sequence order. Connections read the
//! ring directly with a cursor of their own: live delivery and resuming after a reconnect are the
//! same read, and each connection sees strictly increasing sequence numbers. A cursor the ring
//! has moved past (the client fell behind, or asks to resume from before the oldest event kept)
//! means events were lost: the client is told to refetch over REST instead.
//!
//! The ring keeps the last [`SyncConfig::ring_capacity`] events or [`SyncConfig::ring_max_age`]
//! of them, whichever is fewer. It counts events, not bytes, and live-only events (typing) take
//! room like any other. Its epoch is random per boot, so a resume point from another process
//! never matches.
//!
//! While no sync socket is open, the broadcast points skip building their events
//! ([`crate::Server::sync_wanted`]); the ring records the gap instead ([`Ring::skip`]), so a
//! client resuming from before it refetches rather than missing what was never built.
//!
//! The same holds per person for the costlier per-person events (sidebar rows): while someone has
//! no sync socket open, those are skipped for them and the ring keeps a gap marker on their
//! `user` topic instead ([`Ring::push_gap`]), so their next resume from before it refetches.
//!
//! What a connection may subscribe to, and what its typing, presence and heartbeat frames do, is
//! the app's: [`SyncHandler`] opens a [`SyncSession`] per connection.
mod connection;

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::atomic::AtomicUsize;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tokio::sync::watch;

/// The ring, batching and keep-alive settings.
#[derive(Debug, Clone)]
pub struct SyncConfig {
    /// Events kept for replay at most.
    pub ring_capacity: usize,
    /// How long an event stays replayable at most.
    pub ring_max_age: Duration,
    /// How long a connection holds events before writing them as one batch.
    pub flush_interval: Duration,
    /// Events per batch at most; this many pending go out at once.
    pub flush_max: usize,
    /// A `ping` goes out after this long without any other frame.
    pub ping_after: Duration,
    /// How long a new connection may take to send `hello`.
    pub hello_timeout: Duration,
    /// A connection that sends nothing (the client heartbeats every 25 s) for this long is closed.
    pub client_timeout: Duration,
    /// Conversation topics one connection may hold at most.
    pub max_topics: usize,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            ring_capacity: 20_000,
            ring_max_age: Duration::from_secs(15 * 60),
            flush_interval: Duration::from_millis(25),
            flush_max: 64,
            ping_after: Duration::from_secs(15),
            hello_timeout: Duration::from_secs(10),
            client_timeout: Duration::from_secs(90),
            max_topics: 256,
        }
    }
}

/// Who an event is for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Audience {
    /// One person's implicit `user` topic, on each of their connections.
    User(i64),
    /// Every connection's `user` topic (workspace presence).
    Everyone,
    /// Connections subscribed to a conversation topic (`room:<id>`, `thread:<id>`).
    Topic(String),
}

impl Audience {
    pub fn topic(&self) -> &str {
        match self {
            Audience::User(_) | Audience::Everyone => USER_TOPIC,
            Audience::Topic(topic) => topic,
        }
    }
}

/// The implicit per-person topic.
pub const USER_TOPIC: &str = "user";

/// One event to publish.
#[derive(Debug, Clone)]
pub struct SyncPublication {
    pub audience: Audience,
    /// The serialized `SyncPayload`: a JSON object `{"type":…,"data":…}`.
    pub payload: String,
    /// Not delivered to this person's connections (typing isn't echoed to the typist).
    pub except_user: Option<i64>,
    /// Events with the same key replace each other in a connection's pending batch: only the
    /// latest goes out (typing per person and conversation, presence per person).
    pub coalesce: Option<String>,
    /// The audience's connections stop following this topic (they left the room).
    pub unsubscribe: Option<String>,
    /// Live only: not replayed to a resuming client (typing is stale by then).
    pub ephemeral: bool,
}

impl SyncPublication {
    pub fn new(audience: Audience, payload: String) -> Self {
        Self {
            audience,
            payload,
            except_user: None,
            coalesce: None,
            unsubscribe: None,
            ephemeral: false,
        }
    }
}

/// An event in the ring, already encoded as it goes into a batch.
#[derive(Debug)]
pub(crate) struct Entry {
    pub seq: u64,
    pub audience: Audience,
    pub except_user: Option<i64>,
    pub coalesce: Option<String>,
    pub unsubscribe: Option<String>,
    pub ephemeral: bool,
    pub at: Instant,
    /// `{"seq":…,"topic":…,"type":…,"data":…}`
    pub json: Box<str>,
    /// A marker, not an event: the audience's events were skipped here, so a connection that
    /// reads it refetches instead ([`Ring::push_gap`]).
    pub gap: bool,
}

impl Entry {
    /// Whether a connection of `user_id` following `topics` receives this event.
    pub fn delivered_to(&self, user_id: i64, follows: impl Fn(&str) -> bool) -> bool {
        if self.except_user == Some(user_id) {
            return false;
        }
        match &self.audience {
            Audience::User(id) => *id == user_id,
            Audience::Everyone => true,
            Audience::Topic(topic) => follows(topic),
        }
    }
}

/// The replay ring and its epoch.
pub(crate) struct Ring {
    config: SyncConfig,
    epoch: String,
    state: Mutex<RingState>,
    head: watch::Sender<u64>,
}

#[derive(Default)]
struct RingState {
    entries: VecDeque<Arc<Entry>>,
    /// The latest sequence published here (0 before any).
    head: u64,
    /// The latest sequence dropped from the ring (0 before any): a cursor below it missed events.
    evicted_through: u64,
    /// Whether a `hello` has been answered since the last gap: some client may hold a cursor at
    /// the head, even with the ring empty, so skipping an event must be recorded.
    handed_out: bool,
}

/// What a cursor reads from the ring.
pub(crate) enum Read {
    /// The events past the cursor, and the head to move the cursor to.
    Events(Vec<Arc<Entry>>, u64),
    /// The ring dropped events past the cursor; the cursor moves to `head`.
    Lost(u64),
}

impl Ring {
    pub fn new(config: SyncConfig) -> Self {
        let (head, _) = watch::channel(0);
        Self {
            config,
            epoch: new_epoch(),
            state: Mutex::new(RingState::default()),
            head,
        }
    }

    pub fn config(&self) -> &SyncConfig {
        &self.config
    }

    pub fn epoch(&self) -> &str {
        &self.epoch
    }

    pub fn head(&self) -> u64 {
        self.state.lock().unwrap().head
    }

    pub fn watch(&self) -> watch::Receiver<u64> {
        self.head.subscribe()
    }

    /// Appends the event published at `seq`. The caller holds the hub lock, so calls come in
    /// sequence order.
    pub fn push(&self, seq: u64, publication: SyncPublication) {
        let SyncPublication {
            audience,
            payload,
            except_user,
            coalesce,
            unsubscribe,
            ephemeral,
        } = publication;
        let json = encode_event(seq, audience.topic(), &payload);
        let now = Instant::now();
        let entry = Arc::new(Entry {
            seq,
            audience,
            except_user,
            coalesce,
            unsubscribe,
            ephemeral,
            at: now,
            json,
            gap: false,
        });
        self.append(entry, now);
    }

    /// Appends a gap marker for `user_id` at `seq`: their events were skipped while they had no
    /// socket open, so a cursor of theirs from before it has lost events. The caller holds the
    /// hub lock, as for [`Ring::push`].
    pub fn push_gap(&self, seq: u64, user_id: i64) {
        let now = Instant::now();
        let entry = Arc::new(Entry {
            seq,
            audience: Audience::User(user_id),
            except_user: None,
            coalesce: None,
            unsubscribe: None,
            ephemeral: false,
            at: now,
            json: "".into(),
            gap: true,
        });
        self.append(entry, now);
    }

    fn append(&self, entry: Arc<Entry>, now: Instant) {
        let seq = entry.seq;
        let mut state = self.state.lock().unwrap();
        state.entries.push_back(entry);
        state.head = seq;
        self.evict(&mut state, now);
        drop(state);
        self.head.send_replace(seq);
    }

    /// The events after `cursor`, or `Lost` if the ring no longer holds all of them.
    pub fn read(&self, cursor: u64) -> Read {
        let mut state = self.state.lock().unwrap();
        self.evict(&mut state, Instant::now());
        if state.evicted_through > cursor {
            return Read::Lost(state.head);
        }
        let start = state.entries.partition_point(|entry| entry.seq <= cursor);
        Read::Events(state.entries.range(start..).cloned().collect(), state.head)
    }

    /// Whether [`Ring::skip`] has anything to record: a client may hold a cursor that would
    /// otherwise resume past the skipped event. That's so once any `hello` has been answered since
    /// the last gap (its cursor can sit at the head of an empty ring), or with events kept. A
    /// fresh ring that nobody has read needs no gap.
    pub fn gap_needed(&self) -> bool {
        let state = self.state.lock().unwrap();
        state.handed_out || !state.entries.is_empty()
    }

    /// Records that the event published at `seq` was skipped (nobody was connected to read it):
    /// a cursor before it has lost events, so everything kept so far is dropped. The caller holds
    /// the hub lock, as for [`Ring::push`].
    pub fn skip(&self, seq: u64) {
        let mut state = self.state.lock().unwrap();
        state.entries.clear();
        state.head = seq;
        state.evicted_through = seq;
        state.handed_out = false;
        drop(state);
        self.head.send_replace(seq);
    }

    /// Where a `hello` starts reading: `(cursor, resumed)`. A resume point from this epoch that
    /// the ring still covers continues from there; anything else starts at the head.
    pub fn resume(&self, point: Option<(&str, i64)>) -> (u64, bool) {
        let mut state = self.state.lock().unwrap();
        self.evict(&mut state, Instant::now());
        state.handed_out = true;
        match point {
            Some((epoch, seq))
                if epoch == self.epoch
                    && seq >= 0
                    && (seq as u64) <= state.head
                    && (seq as u64) >= state.evicted_through =>
            {
                (seq as u64, true)
            }
            _ => (state.head, false),
        }
    }

    fn evict(&self, state: &mut RingState, now: Instant) {
        while let Some(front) = state.entries.front() {
            let expired = now.saturating_duration_since(front.at) > self.config.ring_max_age;
            if state.entries.len() <= self.config.ring_capacity && !expired {
                break;
            }
            state.evicted_through = front.seq;
            state.entries.pop_front();
        }
    }
}

/// `{"seq":…,"topic":…,` followed by the payload object's members.
fn encode_event(seq: u64, topic: &str, payload: &str) -> Box<str> {
    let members = payload
        .trim_start()
        .strip_prefix('{')
        .expect("a sync payload is a JSON object");
    let topic = serde_json::to_string(topic).expect("strings serialize");
    let separator = if members.trim_start().starts_with('}') {
        ""
    } else {
        ","
    };
    format!(r#"{{"seq":{seq},"topic":{topic}{separator}{members}"#).into_boxed_str()
}

fn new_epoch() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 12];
    rand::rng().fill_bytes(&mut bytes);
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// What the app decides for one sync connection.
#[async_trait::async_trait]
pub trait SyncSession: Send {
    /// Whether this connection may follow `topic` (`room:<id>` or `thread:<id>`).
    async fn authorize(&mut self, topic: &str) -> bool;
    /// `typing`: the person started or stopped typing in `conversation`.
    async fn typing(&mut self, conversation: &str, on: bool);
    /// `present`: the person is looking at the room.
    async fn present(&mut self, room_id: i64);
    /// `absent`: the person stopped looking at the room.
    async fn absent(&mut self, room_id: i64);
    /// `hb`: the connection is alive; `active` if the person used the tab since the last one.
    /// False when the person's session has ended (it idled out): the connection says `bye`.
    async fn heartbeat(&mut self, active: bool) -> bool;
    /// The connection closed.
    async fn close(&mut self);
}

/// Opens a [`SyncSession`] for each authenticated connection.
#[async_trait::async_trait]
pub trait SyncHandler<U>: Send + Sync + 'static {
    /// The person's id, as [`Audience::User`] names them.
    fn user_id(&self, user: &U) -> i64;
    async fn open(&self, user: Arc<U>) -> Box<dyn SyncSession>;
}

/// The engine a [`crate::Server`] runs once [`crate::Server::install_sync`] is called.
pub(crate) struct Engine<U> {
    pub ring: Ring,
    pub handler: Arc<dyn SyncHandler<U>>,
    /// Sync sockets open, from the upgrade until they close.
    pub connections: AtomicUsize,
    /// Who has a sync socket open (from its `hello` until it closes), and who has a gap marker
    /// in the ring since their last one opened.
    pub people: Mutex<People>,
    /// The latest event published for each fence key: see [`crate::Server::sync_publish_fresh`].
    pub fences: Mutex<Fences>,
}

/// [`crate::Server::sync_publish_fresh`] refused a publication read before an event under its
/// fence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stale;

/// [`Engine::fences`]: for each key, the sequence of the latest event published under it.
#[derive(Default)]
pub(crate) struct Fences {
    latest: HashMap<String, u64>,
    /// Keys forgotten to bound the map: their latest event was at or before this sequence.
    forgotten_through: u64,
}

impl Fences {
    /// Keys remembered at most; when full, the older half is forgotten.
    const LIMIT: usize = 4096;

    /// An event was published under `key` at `seq`. The caller holds the hub lock.
    pub fn record(&mut self, key: String, seq: u64) {
        if self.latest.len() >= Self::LIMIT && !self.latest.contains_key(&key) {
            let mut seqs: Vec<u64> = self.latest.values().copied().collect();
            let (_, &mut cutoff, _) = seqs.select_nth_unstable(Self::LIMIT / 2);
            self.latest.retain(|_, seq| *seq > cutoff);
            self.forgotten_through = cutoff;
        }
        self.latest.insert(key, seq);
    }

    /// Whether an event under `key` may have been published after `since`.
    pub fn crossed(&self, key: &str, since: u64) -> bool {
        since < self.forgotten_through || self.latest.get(key).is_some_and(|&seq| seq > since)
    }
}

impl<U> Engine<U> {
    /// A skip for `user_id`: whether it needs a gap marker of its own, which it does unless one
    /// is already pending (pushed, and not yet acted on by a refetch). Marks one pending.
    pub fn gap_needed_for(&self, user_id: i64) -> bool {
        self.people.lock().unwrap().gapped.insert(user_id)
    }

    /// The person's client was told to refetch everything up to its cursor (a `resync`, or a
    /// `welcome` that doesn't resume): any pending marker is acted on.
    pub fn refetched(&self, user_id: i64) {
        self.people.lock().unwrap().gapped.remove(&user_id);
    }
}

/// [`Engine::people`].
#[derive(Default)]
pub(crate) struct People {
    /// Open sockets per person.
    pub open: HashMap<i64, usize>,
    /// People with a [`Ring::push_gap`] marker nobody has acted on yet: since their last socket
    /// opened, or since a live socket of theirs last resynced for one. One is enough until then.
    pub gapped: HashSet<i64>,
}

pub(crate) use connection::run;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fences_forget_the_older_half_by_fencing_everything_read_before_it() {
        let mut fences = Fences::default();
        fences.record("a".into(), 5);
        assert!(fences.crossed("a", 4));
        assert!(!fences.crossed("a", 5));
        assert!(!fences.crossed("b", 0));
        for n in 0..Fences::LIMIT as u64 {
            fences.record(format!("k{n}"), 10 + n);
        }
        // The last key found the map full: the older half went ("a" among them), so a read from
        // before the newest forgotten one is fenced under any key, and the newer half stays.
        let cutoff = 10 + (Fences::LIMIT / 2) as u64 - 1;
        assert!(fences.latest.len() <= Fences::LIMIT / 2 + 1);
        assert!(fences.crossed("a", cutoff - 1));
        assert!(!fences.crossed("a", cutoff));
        let newest = 10 + Fences::LIMIT as u64 - 2;
        assert!(fences.crossed(&format!("k{}", Fences::LIMIT - 2), newest - 1));
        assert!(!fences.crossed(&format!("k{}", Fences::LIMIT - 2), newest));
    }

    fn ring(capacity: usize, max_age: Duration) -> Ring {
        Ring::new(SyncConfig {
            ring_capacity: capacity,
            ring_max_age: max_age,
            ..SyncConfig::default()
        })
    }

    fn publication(topic: &str, n: u64) -> SyncPublication {
        SyncPublication::new(
            Audience::Topic(topic.into()),
            format!(r#"{{"type":"x","data":{n}}}"#),
        )
    }

    fn seqs(read: Read) -> Vec<u64> {
        match read {
            Read::Events(events, _) => events.iter().map(|entry| entry.seq).collect(),
            Read::Lost(head) => panic!("lost (head {head})"),
        }
    }

    #[test]
    fn events_are_encoded_with_seq_and_topic_first() {
        let ring = ring(10, Duration::from_secs(60));
        ring.push(7, publication("room:1", 3));
        let Read::Events(events, head) = ring.read(0) else {
            panic!()
        };
        assert_eq!(head, 7);
        assert_eq!(
            &*events[0].json,
            r#"{"seq":7,"topic":"room:1","type":"x","data":3}"#
        );
        assert_eq!(
            &*encode_event(1, "user", "{}"),
            r#"{"seq":1,"topic":"user"}"#
        );
    }

    #[test]
    fn reads_past_the_cursor_in_sequence_order() {
        let ring = ring(10, Duration::from_secs(60));
        for seq in [2, 5, 9] {
            ring.push(seq, publication("room:1", seq));
        }
        assert_eq!(seqs(ring.read(0)), [2, 5, 9]);
        assert_eq!(seqs(ring.read(4)), [5, 9]);
        assert_eq!(seqs(ring.read(9)), Vec::<u64>::new());
    }

    #[test]
    fn resumes_within_the_ring_and_not_after_rollover() {
        let ring = ring(3, Duration::from_secs(60));
        let epoch = ring.epoch().to_string();
        for seq in 1..=3 {
            ring.push(seq, publication("room:1", seq));
        }
        assert_eq!(ring.resume(Some((&epoch, 1))), (1, true));
        assert_eq!(
            ring.resume(Some((&epoch, 0))),
            (0, true),
            "nothing was evicted yet"
        );
        ring.push(4, publication("room:1", 4));
        // Seq 1 was dropped: resuming from 0 would miss it, from 1 would not.
        assert_eq!(ring.resume(Some((&epoch, 0))), (4, false));
        assert_eq!(ring.resume(Some((&epoch, 1))), (1, true));
        assert!(matches!(ring.read(0), Read::Lost(4)));
        assert_eq!(seqs(ring.read(1)), [2, 3, 4]);
    }

    #[test]
    fn another_epoch_or_a_future_seq_starts_fresh() {
        let ring = ring(3, Duration::from_secs(60));
        ring.push(1, publication("room:1", 1));
        assert_eq!(ring.resume(Some(("someone-else", 1))), (1, false));
        assert_eq!(ring.resume(Some((ring.epoch(), 5))), (1, false));
        assert_eq!(ring.resume(Some((ring.epoch(), -1))), (1, false));
        assert_eq!(ring.resume(None), (1, false));
        assert_ne!(
            Ring::new(SyncConfig::default()).epoch(),
            ring.epoch(),
            "a new epoch per ring"
        );
    }

    #[test]
    fn events_older_than_the_max_age_roll_over() {
        let ring = ring(100, Duration::ZERO);
        ring.push(1, publication("room:1", 1));
        std::thread::sleep(Duration::from_millis(2));
        assert!(matches!(ring.read(0), Read::Lost(1)));
        assert_eq!(ring.resume(Some((ring.epoch(), 0))), (1, false));
        // A cursor at the head lost nothing.
        assert_eq!(seqs(ring.read(1)), Vec::<u64>::new());
        assert_eq!(ring.resume(Some((ring.epoch(), 1))), (1, true));
    }

    #[test]
    fn a_skipped_event_ends_every_resume_point_before_it() {
        let ring = ring(10, Duration::from_secs(60));
        let epoch = ring.epoch().to_string();
        ring.push(1, publication("room:1", 1));
        assert!(ring.gap_needed());
        ring.skip(2);
        assert!(
            !ring.gap_needed(),
            "the gap is recorded and nobody has read since"
        );
        assert_eq!(ring.resume(Some((&epoch, 1))), (2, false));
        assert!(matches!(ring.read(1), Read::Lost(2)));
        assert_eq!(ring.resume(Some((&epoch, 2))), (2, true));
        ring.push(3, publication("room:1", 3));
        assert_eq!(seqs(ring.read(2)), [3]);
    }

    #[test]
    fn a_fresh_ring_needs_no_gap_until_someone_reads_it() {
        let ring = ring(10, Duration::from_secs(60));
        let epoch = ring.epoch().to_string();
        assert!(
            !ring.gap_needed(),
            "no client holds a cursor from this epoch"
        );
        // A welcome hands out the head of the empty ring: skipping now must end that resume point.
        assert_eq!(ring.resume(None), (0, false));
        assert!(ring.gap_needed());
        ring.skip(1);
        assert_eq!(ring.resume(Some((&epoch, 0))), (1, false));
    }

    #[test]
    fn every_gap_after_a_welcome_ends_its_resume_point() {
        let ring = ring(10, Duration::from_secs(60));
        let epoch = ring.epoch().to_string();
        ring.push(1, publication("room:1", 1));
        ring.skip(2);
        // A client connects after the first gap and is welcomed at its sequence, with the ring
        // empty, then drops. The next skipped event still has to be recorded.
        assert_eq!(ring.resume(None), (2, false));
        assert!(ring.gap_needed(), "the welcome's cursor sits at the head");
        ring.skip(3);
        assert!(!ring.gap_needed());
        assert_eq!(ring.resume(Some((&epoch, 2))), (3, false));
        assert!(matches!(ring.read(2), Read::Lost(3)));
        // So does a gap after events aged out of the ring, leaving it empty at the head.
        ring.push(4, publication("room:1", 4));
        assert_eq!(ring.resume(Some((&epoch, 3))), (3, true));
        ring.skip(5);
        assert_eq!(ring.resume(Some((&epoch, 4))), (5, false));
    }

    #[test]
    fn delivery_follows_the_audience() {
        let entry = |audience, except_user| Entry {
            seq: 1,
            audience,
            except_user,
            coalesce: None,
            unsubscribe: None,
            ephemeral: false,
            at: Instant::now(),
            json: "{}".into(),
            gap: false,
        };
        let follows_room_1 = |topic: &str| topic == "room:1";
        assert!(entry(Audience::User(3), None).delivered_to(3, follows_room_1));
        assert!(!entry(Audience::User(3), None).delivered_to(4, follows_room_1));
        assert!(entry(Audience::Everyone, None).delivered_to(4, follows_room_1));
        assert!(entry(Audience::Topic("room:1".into()), None).delivered_to(4, follows_room_1));
        assert!(!entry(Audience::Topic("room:2".into()), None).delivered_to(4, follows_room_1));
        assert!(!entry(Audience::Topic("room:1".into()), Some(4)).delivered_to(4, follows_room_1));
    }
}
