//! Pauses the endpoints at points a test needs to reach deterministically. Built only with the
//! `test-support` feature.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tokio::sync::Barrier;

static AFTER_DUPLICATE_CHECK: Mutex<Option<HashMap<String, Arc<Barrier>>>> = Mutex::new(None);

/// Holds every `POST /api/v1/rooms/:id/messages` with this `clientMessageId` after its
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
