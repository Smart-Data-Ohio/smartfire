//! Creation receipts for requests the client may retry but whose rows have no column to hold
//! the client's key, such as a board post created without a brief (`clientPostId`). A retry
//! finds the receipt and answers what the first attempt made instead of making it again.
//!
//! Receipts live in this process only, for a day, and at most [`LIMIT`] of them: a retry after a
//! restart, or a day later, creates anew. Callers check that a receipt's row still exists before
//! answering with it, since a receipt is taken inside a transaction that may yet roll back.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// How long a receipt is honoured.
pub const KEEP: Duration = Duration::from_secs(24 * 60 * 60);
/// The most receipts held; the oldest go first.
pub const LIMIT: usize = 10_000;

/// What was made, by kind, scope (a room), creator and client key.
type Key = (&'static str, i64, i64, String);

#[derive(Default)]
pub struct Receipts {
    held: Mutex<HashMap<Key, (i64, Instant)>>,
}

impl Receipts {
    #[cfg(any(test, feature = "test-support"))]
    pub fn fixture_snapshot(&self) -> Self {
        Self {
            held: Mutex::new(self.held.lock().unwrap_or_else(|p| p.into_inner()).clone()),
        }
    }

    /// The id of the `kind` row `creator_id` made in `scope_id` with `key`, if still honoured.
    pub fn find(&self, kind: &'static str, scope_id: i64, creator_id: i64, key: &str) -> Option<i64> {
        let held = self.held.lock().unwrap_or_else(|p| p.into_inner());
        held.get(&(kind, scope_id, creator_id, key.to_string()))
            .filter(|(_, at)| at.elapsed() < KEEP)
            .map(|(id, _)| *id)
    }

    /// Records that `creator_id` made the `kind` row `id` in `scope_id` with `key`.
    pub fn record(&self, kind: &'static str, scope_id: i64, creator_id: i64, key: &str, id: i64) {
        let mut held = self.held.lock().unwrap_or_else(|p| p.into_inner());
        held.retain(|_, (_, at)| at.elapsed() < KEEP);
        if held.len() >= LIMIT
            && let Some(oldest) = held.iter().min_by_key(|(_, (_, at))| *at).map(|(k, _)| k.clone())
        {
            held.remove(&oldest);
        }
        held.insert((kind, scope_id, creator_id, key.to_string()), (id, Instant::now()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_a_receipt_by_kind_scope_creator_and_key() {
        let receipts = Receipts::default();
        receipts.record("board_post", 1, 2, "abc", 9);

        assert_eq!(receipts.find("board_post", 1, 2, "abc"), Some(9));
        assert_eq!(receipts.find("board_post", 1, 3, "abc"), None);
        assert_eq!(receipts.find("board_post", 4, 2, "abc"), None);
        assert_eq!(receipts.find("board_post", 1, 2, "abd"), None);
    }
}
