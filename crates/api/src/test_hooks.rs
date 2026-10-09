//! Pauses the endpoints at points a test needs to reach deterministically. Built only with the
//! `test-support` feature.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;

use tokio::sync::Barrier;

static AFTER_DUPLICATE_CHECK: Mutex<Option<HashMap<String, Arc<Barrier>>>> = Mutex::new(None);

/// Holds every message, poll or room post with this `clientMessageId` / `clientRoomId` after its
/// pre-transaction duplicate lookup misses, until `posts` of them have got that far. Other posts
/// aren't held.
pub fn hold_after_duplicate_check(client_message_id: &str, posts: usize) {
    AFTER_DUPLICATE_CHECK
        .lock()
        .unwrap()
        .get_or_insert_with(HashMap::new)
        .insert(client_message_id.to_owned(), Arc::new(Barrier::new(posts)));
}

pub(crate) async fn after_duplicate_check(client_message_id: &str) {
    let barrier = AFTER_DUPLICATE_CHECK
        .lock()
        .unwrap()
        .as_ref()
        .and_then(|held| held.get(client_message_id).cloned());
    if let Some(barrier) = barrier {
        barrier.wait().await;
    }
}

/// A write held just before its transaction: the test waits on `reached`, does what it likes,
/// then waits on `release` to let the write go on.
pub struct WriteHold {
    pub reached: Arc<Barrier>,
    pub release: Arc<Barrier>,
}

type Holds = Mutex<Option<HashMap<i64, WriteHold>>>;

static BEFORE_CATEGORY_WRITE: Holds = Mutex::new(None);
static BEFORE_INVOLVEMENT_WRITE: Holds = Mutex::new(None);
static BEFORE_POLL_VOTE_WRITE: Holds = Mutex::new(None);
static BEFORE_ATTENDANCE_WRITE: Holds = Mutex::new(None);
static BEFORE_PROFILE_READ: Holds = Mutex::new(None);
static AFTER_APPROVAL_PAGE_READ: Holds = Mutex::new(None);

fn hold(holds: &Holds, id: i64) -> WriteHold {
    let held = WriteHold {
        reached: Arc::new(Barrier::new(2)),
        release: Arc::new(Barrier::new(2)),
    };
    let copy = WriteHold {
        reached: held.reached.clone(),
        release: held.release.clone(),
    };
    holds
        .lock()
        .unwrap()
        .get_or_insert_with(HashMap::new)
        .insert(id, held);
    copy
}

async fn wait(holds: &Holds, id: i64) {
    let held = holds
        .lock()
        .unwrap()
        .as_mut()
        .and_then(|held| held.remove(&id));
    if let Some(held) = held {
        held.reached.wait().await;
        held.release.wait().await;
    }
}

/// Holds the next `PATCH /api/v1/room_categories/:id` of this category just before its write
/// transaction, until the test has waited on both barriers.
pub fn hold_before_category_write(category_id: i64) -> WriteHold {
    hold(&BEFORE_CATEGORY_WRITE, category_id)
}

pub(crate) async fn before_category_write(category_id: i64) {
    wait(&BEFORE_CATEGORY_WRITE, category_id).await;
}

/// Holds the next `PUT /api/v1/rooms/:id/involvement` of this membership after the room and
/// membership are looked up, just before its write transaction, until the test has waited on
/// both barriers.
pub fn hold_before_involvement_write(membership_id: i64) -> WriteHold {
    hold(&BEFORE_INVOLVEMENT_WRITE, membership_id)
}

pub(crate) async fn before_involvement_write(membership_id: i64) {
    wait(&BEFORE_INVOLVEMENT_WRITE, membership_id).await;
}

/// Holds the next vote on this poll after its lookups, just before the write transaction.
pub fn hold_before_poll_vote_write(poll_id: i64) -> WriteHold {
    hold(&BEFORE_POLL_VOTE_WRITE, poll_id)
}

pub(crate) async fn before_poll_vote_write(poll_id: i64) {
    wait(&BEFORE_POLL_VOTE_WRITE, poll_id).await;
}

/// Holds the next response to this event after its lookups, just before the write transaction.
pub fn hold_before_attendance_write(event_id: i64) -> WriteHold {
    hold(&BEFORE_ATTENDANCE_WRITE, event_id)
}

pub(crate) async fn before_attendance_write(event_id: i64) {
    wait(&BEFORE_ATTENDANCE_WRITE, event_id).await;
}

/// Holds the next person profile reply after its initial lookup or ban write, just before
/// reading the profile, until the test has waited on both barriers.
pub fn hold_before_profile_read(user_id: i64) -> WriteHold {
    hold(&BEFORE_PROFILE_READ, user_id)
}

pub(crate) async fn before_profile_read(user_id: i64) {
    wait(&BEFORE_PROFILE_READ, user_id).await;
}

/// Holds the next approvals page for this agent after its row read, before expiry/presentation.
pub fn hold_after_approval_page_read(agent_id: i64) -> WriteHold {
    hold(&AFTER_APPROVAL_PAGE_READ, agent_id)
}

pub(crate) async fn after_approval_page_read(agent_id: i64) {
    wait(&AFTER_APPROVAL_PAGE_READ, agent_id).await;
}

/// Holds one rendered sync snapshot before publication, scoped to this app's database.
pub struct SnapshotHold {
    pub reached: tokio::sync::oneshot::Receiver<()>,
    pub release: mpsc::Sender<()>,
}

struct SnapshotPause {
    reached: tokio::sync::oneshot::Sender<()>,
    release: mpsc::Receiver<()>,
}

type SnapshotHolds = Mutex<Option<HashMap<(PathBuf, i64), SnapshotPause>>>;
static AFTER_THREAD_SNAPSHOT: SnapshotHolds = Mutex::new(None);
static AFTER_SIDEBAR_SNAPSHOT: SnapshotHolds = Mutex::new(None);
static AFTER_SIDEBAR_MEMBERSHIPS: SnapshotHolds = Mutex::new(None);

fn hold_snapshot(holds: &SnapshotHolds, database: &Path, id: i64) -> SnapshotHold {
    let (reached, ready) = tokio::sync::oneshot::channel();
    let (release, wait) = mpsc::channel();
    holds.lock().unwrap().get_or_insert_with(HashMap::new).insert(
        (database.to_owned(), id),
        SnapshotPause {
            reached,
            release: wait,
        },
    );
    SnapshotHold {
        reached: ready,
        release,
    }
}

fn after_snapshot(holds: &SnapshotHolds, database: &Path, id: i64) {
    let pause = holds
        .lock()
        .unwrap()
        .as_mut()
        .and_then(|holds| holds.remove(&(database.to_owned(), id)));
    if let Some(pause) = pause {
        pause.reached.send(()).unwrap();
        pause.release.recv_timeout(Duration::from_secs(10)).unwrap();
    }
}

/// Holds the next `thread.updated` snapshot of this thread after it is rendered.
pub fn hold_after_thread_snapshot(database: &Path, thread_id: i64) -> SnapshotHold {
    hold_snapshot(&AFTER_THREAD_SNAPSHOT, database, thread_id)
}

pub(crate) fn after_thread_snapshot(database: &Path, thread_id: i64) {
    after_snapshot(&AFTER_THREAD_SNAPSHOT, database, thread_id);
}

/// Holds the next sidebar row rendered for this room, before it is published.
pub fn hold_after_sidebar_snapshot(database: &Path, room_id: i64) -> SnapshotHold {
    hold_snapshot(&AFTER_SIDEBAR_SNAPSHOT, database, room_id)
}

pub(crate) fn after_sidebar_snapshot(database: &Path, room_id: i64) {
    after_snapshot(&AFTER_SIDEBAR_SNAPSHOT, database, room_id);
}

/// Holds the person's next `GET /api/v1/sidebar` once its memberships are read, before the rest
/// of the sidebar is.
pub fn hold_after_sidebar_memberships(database: &Path, user_id: i64) -> SnapshotHold {
    hold_snapshot(&AFTER_SIDEBAR_MEMBERSHIPS, &canonical(database), user_id)
}

pub(crate) fn after_sidebar_memberships(conn: &campfire_db::Connection, user_id: i64) {
    if let Some(database) = conn.path() {
        after_snapshot(
            &AFTER_SIDEBAR_MEMBERSHIPS,
            &canonical(Path::new(database)),
            user_id,
        );
    }
}

/// The database file as SQLite names it, whichever way the test spelt its path.
fn canonical(database: &Path) -> PathBuf {
    std::fs::canonicalize(database).unwrap_or_else(|_| database.to_owned())
}

static READ_AFTER_SIDEBAR_SNAPSHOT: Mutex<Option<HashMap<(PathBuf, i64), usize>>> =
    Mutex::new(None);

/// After each of the next `renders` sidebar rows rendered for this room, its viewer reads the
/// room again (`room.read` goes out), as if their unread state kept changing faster than a row
/// renders.
pub fn read_after_sidebar_snapshots(database: &Path, room_id: i64, renders: usize) {
    READ_AFTER_SIDEBAR_SNAPSHOT
        .lock()
        .unwrap()
        .get_or_insert_with(HashMap::new)
        .insert((database.to_owned(), room_id), renders);
}

/// Whether the row just rendered for this room is followed by a read: see
/// [`read_after_sidebar_snapshots`].
pub(crate) fn read_after_sidebar_snapshot(database: &Path, room_id: i64) -> bool {
    let mut holds = READ_AFTER_SIDEBAR_SNAPSHOT.lock().unwrap();
    let Some(left) = holds
        .as_mut()
        .and_then(|holds| holds.get_mut(&(database.to_owned(), room_id)))
        .filter(|left| **left > 0)
    else {
        return false;
    };
    *left -= 1;
    true
}
