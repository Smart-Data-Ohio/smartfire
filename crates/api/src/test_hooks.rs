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

/// Holds one rendered sync snapshot before publication, scoped to this app's database.
pub struct ThreadSnapshotHold {
    pub reached: tokio::sync::oneshot::Receiver<()>,
    pub release: mpsc::Sender<()>,
}

struct ThreadSnapshotPause {
    reached: tokio::sync::oneshot::Sender<()>,
    release: mpsc::Receiver<()>,
}

type ThreadSnapshotHolds = Mutex<Option<HashMap<(PathBuf, i64), ThreadSnapshotPause>>>;
static AFTER_THREAD_SNAPSHOT: ThreadSnapshotHolds = Mutex::new(None);

pub fn hold_after_thread_snapshot(database: &Path, thread_id: i64) -> ThreadSnapshotHold {
    let (reached, ready) = tokio::sync::oneshot::channel();
    let (release, wait) = mpsc::channel();
    AFTER_THREAD_SNAPSHOT
        .lock()
        .unwrap()
        .get_or_insert_with(HashMap::new)
        .insert(
            (database.to_owned(), thread_id),
            ThreadSnapshotPause {
                reached,
                release: wait,
            },
        );
    ThreadSnapshotHold {
        reached: ready,
        release,
    }
}

pub(crate) fn after_thread_snapshot(database: &Path, thread_id: i64) {
    let pause = AFTER_THREAD_SNAPSHOT
        .lock()
        .unwrap()
        .as_mut()
        .and_then(|holds| holds.remove(&(database.to_owned(), thread_id)));
    if let Some(pause) = pause {
        pause.reached.send(()).unwrap();
        pause.release.recv_timeout(Duration::from_secs(10)).unwrap();
    }
}
