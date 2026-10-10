//! Boards: forum-style rooms (`RoomKind::Board`) whose threads are posts. Every post is tracked
//! work (a status, an owner, tags and a result; see [`crate::WorkFacts`]) and has no parent
//! message. The SPA shows a board as a list or as four status columns in place of the timeline;
//! a post opens in the right pane like any thread (`GET /api/v1/threads/:id`), and its replies
//! are the thread's messages.
//!
//! Ports `rooms/boards#show` (the board page: `presenters/boards.rs` `listing`,
//! `ChannelThread::board_posts_for`), `channel_threads#new` and `#create` for a board
//! (`presenters/board_posts.rs` `new_post`, `ChannelThread::create_board_post`), and the tags
//! field of `channel_threads#update`.
//!
//! Live updates ride on the thread events of `room:<id>`: `thread.created` when a post is created
//! (by a person, an agent or the API), `thread.updated` when its title, lifecycle, work facts,
//! tags, reply count or last activity change, and `thread.removed` when it's deleted: the JSON
//! twins of the classic board-row frames. A client holding a [`BoardListing`] inserts, moves,
//! re-filters and drops rows from these events without refetching.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{CreateMessage, ThreadSummary, User, WorkOwnerCandidate, WorkStatus};

/// The board's status filter (`status=`): `open` (the default, and what an unknown value
/// reads as) hides done posts, `done` shows only them, `all` shows every post. The column view
/// always asks for `all`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum BoardStatusFilter {
    Open,
    Done,
    All,
}

/// One choice of the owner filter, after "Anyone", "Me" and "Agents": an active member of the
/// board, in `User::active_ordered` order. The label is the user's name, with " (agent)" for an
/// agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BoardOwnerOption {
    /// In the listing's `users`.
    pub user_id: i64,
    pub agent: bool,
}

/// A tag used on the board and how many posts carry it, whatever the filters
/// (`ChannelThread::board_tag_counts`): the tag filter's choices, "tag (count)", by name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BoardTagCount {
    pub name: String,
    pub count: i64,
}

/// The latest stale-work digest claim (`board_stale_digests`, newest `digest_on`), shown only
/// if that claim has an existing message: "Stale-work digest · October 7, 2026" over its plain
/// text. A newer unposted claim hides the previous digest, as on the classic board page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BoardDigest {
    /// The digest's day, `YYYY-MM-DD` (UTC).
    pub date: String,
    /// The digest message's plain text.
    pub text: String,
}

/// `GET /api/v1/rooms/:room_id/board?status=open|done|all&owner=&tag=&page=` (`rooms#show` for a
/// board): the board's posts matching the filters, most recently active first
/// (`last_activity_at DESC, id DESC`).
///
/// Filters, as the classic page applies them:
/// - `owner`: `anyone` (the default, and what anything unrecognised reads as), `me` (owned by the
///   viewer), `agents` (owned by any agent's user, available or not), or a user id;
/// - `tag`: a tag name, stripped and lower-cased; blank for any tag.
///
/// Paged by a growing window, as "Load more" does: page `n` (1 to 20) lists the first `n × 50`
/// matching posts; `hasMore` says whether more match. 404 unless the room is a board the viewer
/// belongs to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BoardListing {
    pub room_id: i64,
    /// The filters as the server read them.
    pub status: BoardStatusFilter,
    /// `anyone`, `me`, `agents`, or a user id as a string.
    pub owner: String,
    /// `""` for any tag.
    pub tag: String,
    pub page: i64,
    /// Each post (`thread.work` is never `null` on a board post, and `thread.parentMessageId`
    /// always is) with the viewer's membership.
    pub posts: Vec<ThreadSummary>,
    pub has_more: bool,
    /// The board has any post at all: "No posts match these filters." when `true` and `posts` is
    /// empty, "No posts yet." when `false`.
    pub any_posts: bool,
    pub owner_options: Vec<BoardOwnerOption>,
    pub tag_counts: Vec<BoardTagCount>,
    pub digest: Option<BoardDigest>,
    /// The viewer is the board's creator or an administrator: they may open the board's
    /// automations and settings.
    pub can_administer: bool,
    /// The posts' creators and the owner options' users, once each. (Owners are whole on
    /// [`crate::WorkFacts::owner`].)
    pub users: Vec<User>,
    pub tags: Vec<BoardTag>,
    pub tags_required: bool,
    pub default_board_tag_id: Option<i64>,
}

/// `GET /api/v1/rooms/:room_id/posts/new` (`channel_threads#new` on a board): what the new-post
/// form offers. 404 unless the room is a board the viewer belongs to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BoardPostForm {
    /// The owner picker: the board's active humans, then its active agents allowed to post
    /// there, each by lower-cased name (`work_owner_candidates_for`). "Unassigned" is the
    /// default.
    pub owner_candidates: Vec<WorkOwnerCandidate>,
    /// The tags already used on the board, by name, for the tags field's suggestions.
    pub tag_suggestions: Vec<String>,
    /// The owner candidates' users.
    pub users: Vec<User>,
    pub tags: Vec<BoardTag>,
    pub tags_required: bool,
    pub default_board_tag_id: Option<i64>,
}

/// One curated tag, in the board's display order. Posts store its name in `thread_tags`;
/// free-text names remain valid alongside these labels.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BoardTag {
    pub id: i64,
    pub name: String,
    pub emoji: Option<String>,
    pub position: i64,
}

/// `GET /api/v1/rooms/:room_id/board/tags`, also returned by catalog/policy writes.
/// Every board member can read it; only the creator or an administrator can write it.
/// Writes publish the existing `board.automations.changed` invalidation on `room:<id>`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BoardTagCatalog {
    pub room_id: i64,
    pub tags: Vec<BoardTag>,
    pub tags_required: bool,
    pub default_board_tag_id: Option<i64>,
}

/// `POST .../board/tags` or `PATCH .../board/tags/:id`. Trimmed names have 1–20 characters;
/// the catalog allows at most 20 names, unique without case. Renames update posts using them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SaveBoardTag {
    pub name: String,
    pub emoji: Option<String>,
}

/// `PUT .../board/tags/order`: every tag id in its desired order, exactly once.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ReorderBoardTags {
    pub tag_ids: Vec<i64>,
}

/// `PATCH .../board`: replace the tag policy. A default must be in this board's catalog.
/// When required, post creation or an explicit tag edit needs a catalog tag; the default
/// supplies one if absent. Untouched legacy tag sets remain editable and readable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct UpdateBoardTagPolicy {
    pub tags_required: bool,
    pub default_board_tag_id: Option<i64>,
}

/// `POST /api/v1/rooms/:room_id/posts` (`channel_threads#create` on a board): create a post. Any
/// member of the board may. The creator joins it; a brief becomes its first message (the
/// board's opener). Answers the post's [`crate::ThreadDetail`] (201) and publishes
/// `thread.created` on `room:<id>`; tag rules may then assign an owner (`thread.updated`).
///
/// Idempotent on `clientPostId` (and on `message.clientMessageId` when there is a brief, which the
/// SPA sets to the same value): a retry by the same creator in the same board returns the post the
/// first attempt made (200), brief or not.
///
/// Errors:
/// - 404 unless the room is a board the viewer belongs to;
/// - `Validation` on `name` (blank, or past 100 characters);
/// - `Validation` on `tags` (more than 5 after normalising, a free-text name past 30 characters
///   or outside `[a-z0-9][a-z0-9-]*`, or no catalog tag when the board requires one and has no default);
/// - `Validation` on `ownerId` "must be an active human member of the parent room" (or, for an
///   agent, "must be an active agent member of the parent room with permission to post");
/// - `Validation` on `message` (past 50,000 characters).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CreateBoardPost {
    /// The title, 1 to 100 characters.
    pub name: String,
    /// The default is `planned`.
    pub status: WorkStatus,
    /// `null` leaves it unassigned.
    pub owner_id: Option<i64>,
    /// Catalog names resolve without case to their display label; other names are stripped
    /// and lower-cased. Blanks and repeats are dropped; up to 5 including any required default.
    pub tags: Vec<String>,
    /// The brief; `null` for none. A blank `markdownSource` without an attachment counts as none.
    pub message: Option<CreateMessage>,
    /// The submission's retry identity: a UUID the client keeps across retries of one post, so a
    /// retry after a lost reply can't make a second post. `null` makes no such promise.
    pub client_post_id: Option<String>,
}
