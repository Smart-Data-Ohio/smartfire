//! Global search and recent searches.
//!
//! Ports `searches#index/create/clear` (`crates/campfire/src/controllers/searches.rs`, from
//! `app/controllers/searches_controller.rb`), the query grammar of `SearchQuery`
//! (`crates/db/src/models/search_query.rs`, from `app/models/search_query.rb`) and the recent
//! searches of `Search` (`crates/db/src/models/search.rs`, from `app/models/search.rb`). See
//! also `docs/search.md`.
//!
//! What search covers is what the classic page covers: messages (full-text, newest first, no
//! relevance ranking) and, on the first page, matching board posts, work threads and events.
//! There are no people, room or file results on the server:
//! - **people and rooms** come from the quick switcher's [`crate::Switcher`], matched on the
//!   client;
//! - **files** are messages with an attachment: `has:file` (an upload or a Drive file) or
//!   `has:image`, whose hits carry their [`crate::Attachment`]. The per-room file list is
//!   `GET /api/v1/rooms/:id/files` ([`crate::FileList`]).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{ConversationName, MessageDTO, RoomKind, Timestamp, User};

/// `GET /api/v1/search?q=&before=`: one page of results (`searches#index`). Doesn't record the
/// query; the client posts [`RecordSearch`] when the person submits it.
///
/// `q` is free text plus operators (`SearchQuery::parse`):
/// - `from:name` (`@` optional): messages by anyone whose name contains it, case-insensitively.
/// - `in:room` (`#` optional): messages in rooms whose name contains it (direct messages never
///   match).
/// - `has:link`, `has:file`, `has:image`, `has:pin`: each must hold.
/// - `before:YYYY-MM-DD`, `after:YYYY-MM-DD` (strictly after that day), `on:YYYY-MM-DD`: days in
///   the viewer's time zone; the last of each wins.
/// - `is:thread`: replies in threads only.
///
/// Repeated `from:` or `in:` values are alternatives; trailing `,.!?;:)` is trimmed from them.
/// An operator that doesn't parse (`has:bogus`, a bad date) stays in the text. The remaining
/// text is split into words, all of which must match (FTS5 with Porter stemming over the plain
/// text). System notes are never found.
///
/// Only rooms the viewer belongs to, which aren't deleted, are searched. A blank `q` (no words
/// and no operators) answers an empty page, not an error.
///
/// `before` is the previous page's `before` cursor (a message id): keyset paging on
/// `(createdAt, id)`, 40 a page. A cursor the viewer can't reach is a 404.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SearchResults {
    /// The query as understood: `q` with runs of whitespace collapsed (`display_query`), the
    /// form recent searches store.
    pub query: String,
    /// One per operator that parsed, in the order written: the filter chips.
    pub chips: Vec<SearchChip>,
    /// The matching messages on this page, oldest first (the page is the newest 40 older than
    /// the cursor). Root messages and thread replies alike; a reply's `threadId` says which
    /// thread.
    pub messages: Vec<MessageDTO>,
    /// The messages' creators, once each.
    pub users: Vec<User>,
    /// The rooms and threads the messages and section rows are in.
    pub conversations: Vec<ConversationName>,
    /// Pass as `before` for the next (older) page: the oldest message here when more exist,
    /// else `null`.
    pub before: Option<i64>,
    /// First page only, and only when `q` has words: up to 10 of each kind whose name (title or
    /// description, for events) contains every word. Narrowed by `in:` but not by the other
    /// operators. Kinds with no matches are left out, so this is often empty.
    pub sections: Vec<SearchSection>,
}

/// A parsed operator (`SearchQuery::chips`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SearchChip {
    pub operator: SearchOperator,
    /// The value as understood: `from:`/`in:` without the `@`/`#` and trailing punctuation,
    /// `has:` lowercased, a date as written, and `true` for `is:thread`. E.g. `ada`.
    pub value: String,
    /// The token as written in `q`, e.g. `from:@ada,`.
    pub token: String,
    /// What the chip shows: `"{operator}: {value}"`, e.g. `from: ada`.
    pub label: String,
    /// `q` without this token: what the chip's remove button searches for.
    pub remove_query: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum SearchOperator {
    From,
    In,
    Has,
    Before,
    After,
    On,
    Is,
}

/// A block of non-message hits above the messages (`SearchQuery::sections_for_user`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SearchSection {
    pub kind: SearchSectionKind,
    pub rows: Vec<SearchSectionRow>,
}

/// The classic `board-posts`, `work-threads` and `events` sections.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum SearchSectionKind {
    /// Threads in boards, by last activity.
    BoardPosts,
    /// Threads outside boards that have a work status, by last activity.
    WorkThreads,
    /// Calendar events, newest start first.
    Events,
}

/// One section hit (`SearchSectionRecord`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SearchSectionRow {
    /// The thread's id, or the event's for `events`.
    pub id: i64,
    pub room_id: i64,
    pub room_kind: RoomKind,
    /// The thread's name or the event's title.
    pub title: String,
    /// The thread's `lastActivityAt`, or the event's `startsAt`.
    pub time: Timestamp,
    /// Work threads (and board posts that have one): `planned`, `in_progress`, `blocked` or
    /// `done` (`channel_threads.work_status`). `null` otherwise.
    pub work_status: Option<String>,
    /// Events only: it was cancelled.
    pub cancelled: bool,
}

/// `GET /api/v1/searches`: the viewer's recent searches, most recent first, at most 10
/// (`Search::recent_for_user`, `RECENT_SEARCHES`). Also the reply to [`RecordSearch`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RecentSearchList {
    pub searches: Vec<RecentSearch>,
}

/// One `searches` row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RecentSearch {
    pub id: i64,
    pub query: String,
    /// When it was last searched for (`updated_at`, touched on each repeat).
    pub searched_at: Timestamp,
}

/// `POST /api/v1/searches`: remember a submitted query (`searches#create`, `Search::record`).
/// Repeating a query moves it to the top instead of adding a row; the list is trimmed to the
/// newest 10. Answers the [`RecentSearchList`] (201). A blank query is a 422 ("Enter a word to
/// search for."), recording nothing. `DELETE /api/v1/searches` forgets them all (204;
/// `searches#clear`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RecordSearch {
    /// Stored with whitespace collapsed.
    pub query: String,
}
