//! Work tracking: threads with a status, an owner (a person or an agent) and a result (S4,
//! contract B).
//!
//! Ports the work facts of `thread_with_facts` and `thread_details`
//! (`crates/web/src/controllers/presenters/message_payload.rs`), the work section of the
//! thread page (`channel_threads/_work.html`, `presenters/board_posts.rs` for links and
//! history), the work list (`work_threads#index`, `ChannelThread::visible_work_threads`), the
//! work fields of `channel_threads#update` (`ChannelThread#update_work`, `#update_result`) and
//! the human handoff (`work_threads#create_handoff`, `ChannelThread#hand_off`).
//!
//! Any thread can be tracked as work, and every board post is (board posts arrive with S6; their
//! work facts are these). Work is for active humans in the thread's room
//! (`ChannelThread#work_viewable_by`); bots and agent tokens never use this API.
//!
//! Deferred, so not in this contract: managing links (classic `work_threads/links.rs` index,
//! create and destroy, and its event candidates), which arrives with boards in S6, and tags,
//! also S6. The links a thread has are on [`WorkFacts::links`].
//!
//! Every work change publishes `thread.updated` with the new [`WorkFacts`]: a status, owner,
//! result, run URL or link change, and a handoff. **New**: the classic app broadcasts none of
//! these (only board rows). A client holding the thread's [`WorkDetail`] refetches
//! `GET /api/v1/threads/:id` when `thread.updated` brings a `work` that differs from the one it
//! holds; any change to the detail changes the facts.

use serde::{Deserialize, Deserializer, Serialize};
use ts_rs::TS;

use crate::{AgentStep, Thread, Timestamp, User, WorkStatus};

/// A thread's work facts, the same for every room member: on [`Thread::work`], so on the thread
/// list, `thread.created` and `thread.updated`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WorkFacts {
    /// Clients read a status added later as unknown, since it rides on every thread list and
    /// thread event; [`UpdateWork::status`] only takes these four.
    pub status: WorkStatus,
    /// The person or agent who owns it, whole, so a client can show them from a `thread.created`
    /// or `thread.updated`, which carry no `users`, and when they've left the room (an agent's
    /// `agent` badge included). `null` reads "Unassigned". Send [`UpdateWork::owner_id`] to
    /// change it.
    pub owner: Option<User>,
    /// The owner can act on it: active, still a member of the room, and for an agent allowed to
    /// post there (`work_owner_active`). `false` when unassigned. Not republished when only the
    /// owner's membership or an agent's grants change, as in the classic app.
    pub owner_active: bool,
    /// The agent's run (a CI job, a session), set through the agent API. Only an `https://` URL:
    /// `null` for none and for any other stored value, as the classic page shows `run_url` only
    /// when it starts with `https://` (`board_posts.rs`). The server filters it; clients check
    /// again before putting it in an `href`.
    pub run_url: Option<String>,
    /// When the result was last edited; `null` while there's none. The result itself is on
    /// [`WorkDetail`].
    pub result_updated_at: Option<Timestamp>,
    /// Linked pull requests, calendar events and Drive files, oldest first.
    pub links: Vec<WorkLink>,
    /// The server's revision of these facts: the thread's `updated_at`, which every change to
    /// the status, owner, run URL, result or tracking moves (and other thread changes too).
    /// Links, the owner's own profile and `ownerActive` can change without moving it, so a
    /// client merges those fields on their own rather than by this revision.
    /// A client keeps a copy only if its `updatedAt` is not older than the one it holds, on
    /// every path (reads, write replies, events and refetches), so a late or replayed copy never
    /// undoes a newer one.
    pub updated_at: Timestamp,
}

/// `work_thread_links.kind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum WorkLinkKind {
    PullRequest,
    Event,
    DriveFile,
}

/// A linked pull request's state (`github::state_label`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum WorkPullRequestState {
    Open,
    Draft,
    Merged,
    Closed,
}

/// One linked item, as the classic links box shows it (`board_posts::LinkSources::items`). The
/// same for every viewer: a private repository's pull request never shows its title.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WorkLink {
    pub id: i64,
    pub kind: WorkLinkKind,
    /// `owner/repo#123` for a pull request, the event's title, or the Drive file's title (its
    /// URL when the title is unknown).
    pub label: String,
    /// The pull request on GitHub or the Drive file, always an `https://` URL, or the event's
    /// classic page, a site-relative path (`/rooms/:roomId/events/:id`). The server leaves out a
    /// link whose stored URL is neither; clients check again before putting it in an `href`.
    pub url: String,
    /// Pull requests only; `null` otherwise.
    pub pull_request_state: Option<WorkPullRequestState>,
    /// A pull request's title, only when its repository is public; `null` otherwise.
    pub title: Option<String>,
    /// Events only: when it starts, and the time zone it's shown in. `null` otherwise.
    pub event_starts_at: Option<Timestamp>,
    pub event_time_zone: Option<String>,
    /// Events only: it was cancelled. `false` otherwise.
    pub event_cancelled: bool,
}

/// The thread page's work section, for one viewer: on [`crate::ThreadDetail::work`] when the
/// thread is tracked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WorkDetail {
    /// The result as typed (up to 20,000 characters); `null` for none.
    pub result_markdown: Option<String>,
    /// The result rendered as the classic board post renders it (`render_markdown` in the
    /// thread's room); `null` when `resultMarkdown` is.
    pub result_html: Option<String>,
    /// Who last edited the result; `null` when there's none, and kept when that person's
    /// account is later deleted (no `users` entry then: "by someone").
    pub result_updated_by_id: Option<i64>,
    /// The steps the owning agent reports on the thread (at most 50), in `(position, id)` order;
    /// `agent.steps` on `thread:<id>` updates them.
    pub steps: Vec<AgentStep>,
    /// Newest first (`work_thread_events`, `created_at DESC, id DESC`). Not paged, as in the
    /// classic page.
    pub history: Vec<WorkHistoryEntry>,
    /// Who the work can be assigned to, when the viewer may assign it (`canAssignWork`): the
    /// room's active humans, then its active agents allowed to post there, each by lower-cased
    /// name (`work_owner_candidates_for`). Empty otherwise.
    pub owner_candidates: Vec<WorkOwnerCandidate>,
    /// The agents it can be handed off to, when the viewer may manage it (`canManageWork`):
    /// agents in the room that may post, manage threads and read messages there, except the
    /// current owner (`WorkHandoff::receivers_for`), by lower-cased name. Empty otherwise.
    pub handoff_receivers: Vec<WorkHandoffReceiver>,
}

/// One choice in the owner picker.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WorkOwnerCandidate {
    pub user_id: i64,
    /// Agents only (the agent's `provider` and `description`); `null` for people and when blank.
    pub provider: Option<String>,
    pub description: Option<String>,
}

/// An agent the work can be handed off to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WorkHandoffReceiver {
    /// Send as [`CreateWorkHandoff::receiver_agent_id`].
    pub agent_id: i64,
    /// The agent's bot user, in the detail's `users`.
    pub user_id: i64,
}

/// `work_thread_events.event_type` (`EVENT_TYPES`): `work_update` is `update`,
/// `work_assignment` is `assignment`, `work_handoff` is `handoff` and `result_updated` is
/// `result`. Clients read a kind added later as unknown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum WorkHistoryKind {
    /// `work_update`: the status changed (and maybe the owner): tracking started, moved or
    /// stopped.
    Update,
    /// `work_assignment`: only the owner changed.
    Assignment,
    /// `work_handoff`: a person handed the work off to an agent.
    Handoff,
    /// `result_updated`: the result was edited.
    Result,
}

/// An owner as it was at the time (`from_owner_id`/`from_owner_name`, `to_owner_id`/
/// `to_owner_name`): the name is a snapshot, so it survives a rename or a deleted account. An
/// entry with neither is `null` on [`WorkHistoryEntry`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WorkOwnerSnapshot {
    /// `null` when only the name was recorded.
    pub user_id: Option<i64>,
    /// `null` when no name was recorded; it reads "Unassigned", as in the classic history.
    pub name: Option<String>,
}

/// The handoff an entry records: "· {summary} ({n} links, {m} open questions)".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WorkHistoryHandoff {
    /// The handoff summary cut to 200 characters (ending "..." when cut).
    pub summary: String,
    pub link_count: i64,
    pub question_count: i64,
}

/// One line of the work history (`board_posts::history_records`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WorkHistoryEntry {
    pub id: i64,
    pub kind: WorkHistoryKind,
    pub created_at: Timestamp,
    /// `null`, or no `users` entry, reads "Former member".
    pub actor_id: Option<i64>,
    /// `null` is an ordinary thread (not tracked): "Ordinary thread". Read tolerantly, as
    /// [`WorkFacts::status`] is.
    pub from_status: Option<WorkStatus>,
    pub to_status: Option<WorkStatus>,
    /// `null` reads "Unassigned".
    pub from_owner: Option<WorkOwnerSnapshot>,
    pub to_owner: Option<WorkOwnerSnapshot>,
    /// The note an agent left with the change; `null` for none.
    pub note: Option<String>,
    /// `handoff` entries only; `null` otherwise.
    pub handoff: Option<WorkHistoryHandoff>,
}

/// Which work `GET /api/v1/work` lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum WorkFilter {
    /// The default: everything not done.
    Open,
    Done,
    All,
    /// Owned by an agent, whatever the status.
    Agents,
    /// Board posts, whatever the status.
    Boards,
}

/// One row of the work list (`work_threads/_thread.html`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WorkListRow {
    /// Its `work` is never `null` here. A board post has no `parentMessageId`.
    pub thread: Thread,
    /// The viewer-relative room name.
    pub room_name: String,
    /// The room is a board, so this is a board post.
    pub board: bool,
    /// "Updated …": the thread's `updated_at`, which the list is ordered by.
    pub updated_at: Timestamp,
}

/// `GET /api/v1/work?state=open|done|all|agents|boards` (`work_threads#index`): tracked threads
/// in the viewer's rooms, most recently updated first (`updated_at DESC, id DESC`). An unknown
/// or missing `state` is `open`. Not paged, as in the classic app. No live updates either: the
/// list refetches when it's shown again.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WorkList {
    pub threads: Vec<WorkListRow>,
    /// The threads' creators, once each. (Owners are whole on [`WorkFacts::owner`].)
    pub users: Vec<User>,
}

/// `PATCH /api/v1/threads/:id/work`: track, move, assign or stop tracking a thread, or edit its
/// result (`channel_threads#update`'s work fields). A missing field is left alone; `null`
/// clears it. Answers the [`crate::ThreadDetail`] and publishes `thread.updated` when anything
/// changed.
///
/// What each change needs ([`crate::ThreadPermissions`]), checked by the server whatever the
/// client showed; any refusal is a 403 and changes nothing:
/// - a status on an untracked thread starts tracking it: `canConvertWork`;
/// - `status: null` stops tracking it: `canAssignWork` (the client offers it per
///   `canRemoveWork`, which is `false` for a board post; the server doesn't refuse it there, as
///   in the classic app);
/// - another status on a tracked thread: `canUpdateWorkStatus`;
/// - `ownerId`: `canAssignWork`;
/// - `resultMarkdown`: `canManageWork`.
///
/// Errors:
/// - 404 unless the viewer is an active human member of the thread's room;
/// - `Validation` on `ownerId` "must be an active human member of the parent room" (or, for an
///   agent, "must be an active agent member of the parent room with permission to post");
/// - `Validation` on `ownerId` "requires work tracking" when the thread would have an owner but
///   no status: stopping tracking a thread with an owner needs `ownerId: null` too;
/// - `Validation` on `resultMarkdown` past 20,000 characters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct UpdateWork {
    /// Omit to leave alone; `null` stops tracking.
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<Option<WorkStatus>>,
    /// Omit to leave alone; `null` unassigns.
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_id: Option<Option<i64>>,
    /// Omit to leave alone; `null` or blank clears the result.
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_markdown: Option<Option<String>>,
}

/// [`UpdateWork`] as it decodes: serde reads a `null` and a missing key alike as `None` for an
/// `Option<Option<_>>`, so each field goes through [`present`]. Kept apart from `UpdateWork` so
/// ts-rs never sees `deserialize_with`, which it doesn't read.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateWorkFields {
    #[serde(default, deserialize_with = "present")]
    status: Option<Option<WorkStatus>>,
    #[serde(default, deserialize_with = "present")]
    owner_id: Option<Option<i64>>,
    #[serde(default, deserialize_with = "present")]
    result_markdown: Option<Option<String>>,
}

impl<'de> Deserialize<'de> for UpdateWork {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let fields = UpdateWorkFields::deserialize(deserializer)?;
        Ok(Self {
            status: fields.status,
            owner_id: fields.owner_id,
            result_markdown: fields.result_markdown,
        })
    }
}

/// A key that's present, as `Some`: `null` is `Some(None)`. A missing key is `None` (from
/// `default`).
fn present<'de, T, D>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}

/// `POST /api/v1/threads/:id/work/handoff`: hand tracked work to an agent
/// (`work_threads#create_handoff`). The agent becomes the owner, and gets the package as a
/// `work_handed_off` delivery. Answers the [`crate::ThreadDetail`] (201) and publishes
/// `thread.updated`.
///
/// Errors, each with the classic message:
/// - 404 unless the viewer is an active human member of the thread's room;
/// - `Validation` on `base` when the thread isn't tracked (a bare 422 in the classic app);
/// - 403 unless the viewer may manage the work (`canManageWork`);
/// - `Validation` on `receiverAgentId` when the agent isn't in [`WorkDetail::handoff_receivers`]
///   any more: "Receiver must be an active agent member of this room with permission to post",
///   "Receiver must hold the manage_threads capability in this room", "… the read_messages
///   capability …", or "Receiver is already the owner of this work";
/// - `Validation` on the package's fields: `summary` blank or past 2,000 characters; more than
///   10 `links` or `openQuestions`, one past 500 characters, or a link that isn't an http(s) URL.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CreateWorkHandoff {
    pub receiver_agent_id: i64,
    pub summary: String,
    /// Each is trimmed; blank ones and repeats are dropped.
    pub links: Vec<String>,
    /// As `links`.
    pub open_questions: Vec<String>,
}
