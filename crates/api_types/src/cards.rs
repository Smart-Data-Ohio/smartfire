//! Typed cards under a message: polls, calendar events, and link previews for GitHub pull
//! requests, posts on X, Fizzy cards, LinkedIn posts and other web pages.
//!
//! The classic app renders each kind as server HTML in its own slot of the message
//! (`crates/views/templates/messages/_message.html`) and replaces that slot when the card
//! changes. The SPA gets typed data instead: [`crate::MessageDTO::poll`] and
//! [`crate::MessageDTO::cards`], refreshed by the `poll.updated` and `message.cards` events.
//!
//! **Fill order.** The server moves one provider at a time from HTML to these types. Until it
//! fills a kind, that kind is simply absent (the link in the body still works). The order,
//! from the most structured and viewer-independent data to the least:
//! 1. polls ([`Poll`]);
//! 2. Google Drive attachments ([`DriveFileCard`]);
//! 3. posts on X ([`XPostCard`]);
//! 4. other web pages ([`LinkCard`]);
//! 5. quoted messages ([`QuoteCard`] → [`QuotePreviewResult`]);
//! 6. calendar events ([`EventCard`], with [`EventAttendance`]);
//! 7. GitHub pull requests ([`GithubCardRef`] → [`GithubPullRequestCard`]);
//! 8. LinkedIn posts ([`LinkedinCard`]);
//! 9. Fizzy cards ([`FizzyCardRef`] → [`FizzyCardPreview`]).
//!
//! **Ordering.** A message's poll and cards change without its `updatedAt` moving, and they
//! arrive from several places: pages, `message.created`/`message.updated`, `poll.updated`,
//! `message.cards` and HTTP replies. Each carries an `asOf` (or `cardsAsOf` on
//! [`crate::MessageDTO`]): the server time at the start of the read that built it. The client
//! keeps, per message, the poll and the cards with the latest `asOf`, and on a tie the one that
//! arrived last, so a late reply or a stale event never overwrites newer data. A message
//! payload's other fields follow `updatedAt` as before. A `poll: null` has no `asOf`, so it
//! never orders anything: a later `message.updated` (or page) whose `poll` is `null` must not
//! clear a poll the client already holds. Only a [`Poll`] with a later `asOf` replaces it.
//!
//! Everything on [`crate::MessageDTO`] is the same for every viewer, because the message is
//! broadcast. GitHub and Fizzy previews depend on the viewer's own account access, so the
//! message carries only a reference and the client fetches the preview, as the classic lazy
//! frames do.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::Timestamp;

/// A poll attached to its question message (`polls`, one per message; `Poll::results_payload`
/// without the viewer, from `app/models/poll.rb`). The question is the message's body.
///
/// Fill order: 1.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Poll {
    pub id: i64,
    pub message_id: i64,
    /// When the server read this state (see the module's **Ordering**): keep the poll with the
    /// latest one.
    pub as_of: Timestamp,
    /// Several options may be chosen; otherwise exactly one.
    pub multiple: bool,
    /// Voters are hidden: `voterIds` stay empty, and the viewer's own choice comes from
    /// `GET /api/v1/rooms/:roomId/polls/:id` ([`PollResults`]).
    pub anonymous: bool,
    /// When voting stops by itself; `null` for never.
    pub closes_at: Option<Timestamp>,
    /// When the periodic closer stamped it closed; `null` until then.
    pub closed_at: Option<Timestamp>,
    /// Closed when sent (`Poll#closed?`: `closedAt` set, or `closesAt` passed). The client also
    /// treats it as closed once `closesAt` passes, before `poll.updated` says so.
    pub closed: bool,
    /// Votes cast across every option (`poll_votes` rows; a voter in a multiple-choice poll
    /// counts once per option chosen).
    pub total_votes: i64,
    /// In `(position, id)` order, 2 to 10 of them.
    pub options: Vec<PollOption>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PollOption {
    pub id: i64,
    /// Up to 200 characters.
    pub label: String,
    pub votes: i64,
    /// Who chose it, by id. Empty for an anonymous poll. The classic card shows their names
    /// and works out the viewer's own vote from these ids.
    pub voter_ids: Vec<i64>,
}

/// `GET /api/v1/rooms/:roomId/polls/:id` (`rooms/polls#show`), and the reply to
/// [`VotePoll`]: the poll with the viewer's own choice.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PollResults {
    pub poll: Poll,
    /// The options the viewer chose (`voted`); empty when they haven't voted.
    pub my_option_ids: Vec<i64>,
}

/// `POST /api/v1/rooms/:roomId/polls`: post a question with a poll (`rooms/polls#create`,
/// `Poll::create_for_message`). Answers the question's [`crate::MessageDTO`] (201) with its
/// `poll`, published as `message.created`. Active human members only (403).
///
/// Idempotent on `clientMessageId`, as [`crate::CreateMessage`] is (`Message::find_duplicate`):
/// posting the same id again returns the question already created, with 200 instead of 201,
/// and creates no second poll. The optimistic row reconciles with `message.created` by that id.
/// New: the classic form has no client id and replies with a redirect.
///
/// 422 when the question is blank, there are fewer than 2 or more than 10 non-blank options,
/// a label is over 200 characters, or `closesAt` isn't in the future.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CreatePoll {
    /// The sender's id for the question message: a UUID, as for [`crate::CreateMessage`].
    pub client_message_id: String,
    /// Markdown, posted as the message.
    pub question: String,
    pub options: Vec<String>,
    pub multiple: bool,
    pub anonymous: bool,
    pub closes_at: Option<Timestamp>,
}

/// `POST /api/v1/rooms/:roomId/polls/:id/vote`: replace the viewer's whole ballot
/// (`rooms/polls#vote`, `Poll#cast_vote`); an empty list takes their vote back. Answers
/// [`PollResults`], publishes `poll.updated` ([`PollUpdated`]) to the room and `poll.ballot`
/// ([`PollBallot`]) to the voter's other tabs. 422 when the poll is closed, an option isn't the
/// poll's, or a single-choice poll gets more than one. Active human members only (403).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct VotePoll {
    pub option_ids: Vec<i64>,
}

/// One card under a message, tagged by `kind` with its body in `data`. In the classic slot
/// order: Drive attachments (the attachment slot, above the others), GitHub pull requests,
/// posts on X, events, Fizzy cards, quoted messages, LinkedIn posts, then other pages, each in
/// the order the links appear in the body.
///
/// LinkedIn and other-page cards are left out while the message's `embedsSuppressed` is set
/// (the author hid previews); the others ignore it, as in the classic app.
///
/// Kinds are added one provider at a time, so a client must accept a `kind` it doesn't know
/// (and a known kind whose `data` it can't read) by skipping that card, never by rejecting the
/// whole message. The SPA's schema decodes such a card to an `unknown` placeholder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", content = "data", rename_all = "lowercase")]
#[ts(export)]
pub enum MessageCard {
    Drive(DriveFileCard),
    Github(GithubCardRef),
    X(XPostCard),
    Event(EventCard),
    Fizzy(FizzyCardRef),
    Quote(QuoteCard),
    Linkedin(LinkedinCard),
    Link(LinkCard),
}

/// A Google Drive file attached to the message (`drive_attachments`, in id order). The classic
/// message shows it in the attachment slot as "Google Drive file · Open in Drive"
/// (`messages/_drive_attachments.html`); the file's name and type aren't stored. Search finds
/// these with `has:file`, but [`crate::FileList`] doesn't list them.
///
/// Fill order: 2.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DriveFileCard {
    /// `drive_attachments.file_id`.
    pub file_id: String,
    /// `https://drive.google.com/open?id={fileId}`.
    pub url: String,
}

/// A message this one quotes by linking to it (`message_references`, in id order;
/// `message_link_cards` in the classic slot). The source may be in a room the viewer can't see,
/// so the shared message carries a preview only when the source is in the same room (anyone
/// who can see this message can see it). Otherwise the client fetches
/// `GET /api/v1/rooms/:roomId/message_links/:referenceId/card` ([`QuotePreviewResult`]), as the
/// classic lazy frame does (`rooms/message_links#show`, `message_quote::visible`).
///
/// Fill order: 5.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct QuoteCard {
    /// `message_references.id`: the id the fetch takes. The source message's id isn't sent
    /// until the viewer is known to see it.
    pub reference_id: i64,
    /// Same-room sources only; `null` means fetch it.
    pub preview: Option<QuotePreview>,
}

/// What a quote card shows (`campfire_views::message_links::Card`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct QuotePreview {
    pub message_id: i64,
    pub room_id: i64,
    pub thread_id: Option<i64>,
    pub creator_id: i64,
    /// The author's name, so a card from another room renders without that room's people.
    pub author_name: String,
    /// The room's name, or `"a direct message"`.
    pub room_label: String,
    /// The source's plain text, at most 200 characters (197 and `...`).
    pub excerpt: String,
    pub created_at: Timestamp,
}

/// The reply to `GET /api/v1/rooms/:roomId/message_links/:referenceId/card`. 404 unless the
/// viewer is a member of `roomId` and the reference belongs to a message in that room
/// (`message_quote::source`). Tagged by `state`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(
    tag = "state",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
#[ts(export)]
pub enum QuotePreviewResult {
    /// The viewer can't see the source (`message_quote::visible` is false): show the plain
    /// link, as the classic frame's fallback does.
    Hidden,
    Loaded(QuotePreview),
}

/// Where a shared card's fetch stands. The server starts the fetch when the message is posted
/// or edited and publishes `message.cards` when it finishes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum CardFetch {
    /// Not fetched yet (`fetched_at` and `fetch_error` both unset).
    Loading,
    Loaded,
    /// The last fetch failed (`fetch_error`); the card falls back to a plain link.
    Failed,
}

/// A post on X (`twitter_posts`, shared by every message that links it; fetched from
/// fxtwitter: `crates/app/src/integrations/twitter`). Up to the classic card's fields
/// (`presenters/twitter_cards.rs`).
///
/// Fill order: 3.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct XPostCard {
    pub fetch: CardFetch,
    /// X's status id (a string: it's past 2^53).
    pub post_id: String,
    /// The post's link (`Post#view_url`): as written, or `https://x.com/i/status/<id>`.
    pub url: String,
    /// `Post#display_name`: the author's name, `@handle`, or "Post on X".
    pub author_name: String,
    /// `Post#display_handle`, without the `@`; `null` when unknown.
    pub author_handle: Option<String>,
    pub author_avatar_url: Option<String>,
    pub text: Option<String>,
    pub posted_at: Option<Timestamp>,
    pub replies: Option<i64>,
    pub reposts: Option<i64>,
    pub likes: Option<i64>,
    pub media: Vec<XMedia>,
    /// The post it quotes; `null` for none.
    pub quote: Option<XQuote>,
}

/// One photo, video or GIF on a post (`twitter_posts.media`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct XMedia {
    pub kind: XMediaKind,
    pub url: String,
    pub thumbnail_url: Option<String>,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub alt: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum XMediaKind {
    Photo,
    Video,
    Gif,
}

/// A quoted post (`twitter_posts.quote`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct XQuote {
    pub url: Option<String>,
    pub author_name: Option<String>,
    pub author_handle: Option<String>,
    pub text: Option<String>,
}

/// A preview of any other web page (`link_embeds`, shared and cached per normalized URL;
/// `crates/app/src/integrations/link_embed`). Pages excluded from it: GitHub pull requests, X,
/// LinkedIn, Google Drive, Fizzy hosts, this app's own room links, and links written as
/// `<url>`. Only pages with a title or description get a card (`usable`), so there's no
/// loading or failed state: the card appears once a fetch finds one.
///
/// Fill order: 4.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LinkCard {
    /// The link as the author wrote it (`link_embed_references.url`).
    pub url: String,
    /// Up to 100 characters.
    pub site_name: Option<String>,
    /// Up to 300 characters.
    pub title: Option<String>,
    /// Up to 1000 characters.
    pub description: Option<String>,
    pub image_url: Option<String>,
}

/// A calendar event the message links to (`event_references`: any `/rooms/:id/events/:id` URL
/// for an event in the message's own room; posting an event writes one). The classic card's
/// fields (`campfire_views::events::CardView`). The viewer's response and the counts are
/// [`EventAttendance`].
///
/// Fill order: 6.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct EventCard {
    pub event_id: i64,
    pub room_id: i64,
    pub title: String,
    pub organizer_id: i64,
    pub starts_at: Timestamp,
    /// `null` for an open-ended event.
    pub ends_at: Option<Timestamp>,
    /// The organizer's IANA zone the times were set in, e.g. `America/New_York`.
    pub time_zone: String,
    /// One occurrence of a repeating series.
    pub recurring: bool,
    pub cancelled: bool,
    /// The voice or stage room it's held in; `null` for none or once that room is gone.
    pub venue_room_id: Option<i64>,
    pub venue_name: Option<String>,
    /// A Google Meet link, HTTPS only; `null` for none.
    pub meet_link: Option<String>,
}

/// `event_attendances.response` (`calendar_event::attendance::RESPONSES`; no row means no response yet).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum AttendanceResponse {
    Going,
    Maybe,
    Declined,
}

/// `GET /api/v1/rooms/:roomId/events/:id/attendance`: the viewer's response and the counts
/// (`rooms/events/attendances#show`, the classic card's lazy frame). Also the reply to
/// [`RespondToEvent`]. Members of the event's room only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct EventAttendance {
    pub event_id: i64,
    /// `null` for "No response yet". The organizer starts as `going`.
    pub response: Option<AttendanceResponse>,
    pub going_count: i64,
    pub maybe_count: i64,
    pub declined_count: i64,
    /// The viewer may respond: an active human member, and the event isn't cancelled.
    pub respondable: bool,
    /// Offer "Apply to all future occurrences": it's the head of a series, or an occurrence
    /// with later ones.
    pub can_apply_to_future: bool,
}

/// `PUT /api/v1/rooms/:roomId/events/:id/attendance` (`rooms/events/attendances#update`,
/// `CalendarEvent::respond`). On a series' first event the response covers every future
/// occurrence; on a later one, only that one unless `applyToFuture`. Answers
/// [`EventAttendance`]. 422 when the event is cancelled or the viewer can't respond. The
/// classic app broadcasts nothing for responses, so counts elsewhere refresh on the next read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RespondToEvent {
    pub response: AttendanceResponse,
    pub apply_to_future: bool,
}

/// A GitHub pull request the message links to (`github_pull_request_references`). Only the
/// identity: what the card shows depends on whether the viewer can read the repository, so the
/// client fetches `GET /api/v1/rooms/:roomId/github/pull_requests/:pullRequestId/card?messageId=`
/// ([`GithubPullRequestCard`]), as the classic lazy frame does
/// (`controllers/github/cards.rs`). Issues, commits and discussions get no card.
///
/// Fill order: 7.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GithubCardRef {
    /// `github_pull_requests.id`.
    pub pull_request_id: i64,
    pub owner: String,
    pub repo: String,
    pub number: i64,
    /// `https://github.com/{owner}/{repo}/pull/{number}`.
    pub url: String,
}

/// The reply to `GET /api/v1/rooms/:roomId/github/pull_requests/:id/card?messageId=` or
/// `?threadId=` (`rooms/github/pull_request_cards#show`, `viewer_card_context`): the pull request
/// as the viewer may see it (`Github::PullRequest#visible_to`: a public repository, or one the
/// viewer's connected GitHub account can read). Tagged by `state`.
///
/// Exactly one of the two parameters:
/// - `messageId`: the card under a message. 404 unless the viewer is a member of `roomId`, the
///   message is in that room, and it references this pull request
///   (`github_pull_request_references`).
/// - `threadId`: the header of the room's discussion thread for the pull request, which also
///   lists the changed files ([`GithubPullRequest::files`]). 404 unless the viewer is a member of
///   `roomId`, the thread is in that room, and it's this pull request's discussion thread there
///   (`github_pull_request_threads`).
///
/// Neither, or an unknown pull request, is a 404.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(
    tag = "state",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
#[ts(export)]
pub enum GithubPullRequestCard {
    /// The viewer can't see it: show the plain link.
    Hidden,
    /// Not fetched yet.
    Loading,
    /// The last fetch failed ("Couldn't load this pull request.").
    Failed {
        message: String,
    },
    Loaded(Box<GithubPullRequest>),
}

/// A fetched pull request (`github_pull_requests`), as the classic card shows it
/// (`campfire_views::github::card`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GithubPullRequest {
    pub owner: String,
    pub repo: String,
    pub number: i64,
    pub title: String,
    pub url: String,
    pub status: GithubPullRequestStatus,
    pub author_login: Option<String>,
    pub author_avatar_url: Option<String>,
    pub base_branch: Option<String>,
    pub head_branch: Option<String>,
    /// `null` when GitHub reports none.
    pub review: Option<GithubReview>,
    /// `null` for "No checks".
    pub checks: Option<GithubChecks>,
    pub github_updated_at: Option<Timestamp>,
    /// The thread discussing it in this room, for "Discuss"; `null` until someone starts one
    /// (`POST /rooms/:id/github/pull_request_threads` in the classic app).
    pub discussion_thread_id: Option<i64>,
    /// The thread-header variant only (`?threadId=`, `card_with_files`): the changed files
    /// GitHub reported. `null` for the card under a message.
    pub files: Option<GithubChangedFiles>,
}

/// A pull request's changed files, as the classic thread header lists them
/// (`Github::PullRequest#changed_files_summary`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GithubChangedFiles {
    /// The files stored with the pull request, in GitHub's order (a capped list).
    pub files: Vec<GithubChangedFile>,
    /// How many files changed in all; more than `files.len()` when the list was capped
    /// ("and N more").
    pub total_count: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GithubChangedFile {
    pub filename: String,
    /// GitHub's file status (`added`, `modified`, `removed`, `renamed`, ...); `null` when
    /// missing.
    pub status: Option<String>,
    pub additions: i64,
    pub deletions: i64,
}

/// `github_pull_requests.state` as the fetcher derives it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum GithubPullRequestStatus {
    Open,
    Draft,
    Merged,
    Closed,
}

/// `github_pull_requests.review_decision`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum GithubReview {
    Approved,
    ChangesRequested,
    ReviewRequired,
}

/// `github_pull_requests.check_status`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum GithubChecks {
    Passing,
    Pending,
    Failing,
}

/// `GET /api/v1/rooms/:roomId/github/pull_requests/:id/actions`: what this viewer can do on the
/// pull request from its card. The same gate as the classic write-actions page
/// (`rooms/github/pull_request_write_actions#show`): the viewer is a member of the room, the
/// room has a discussion thread for this pull request, and their linked GitHub account is
/// usable. All three flags are on together, or all off. The pull request's [`status`] does not
/// hide them (a closed pull request still shows the forms); GitHub refuses a review the state
/// doesn't allow, and that refusal is the error on the write.
///
/// 404 unless that membership and thread mapping exist.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GithubPullRequestActions {
    /// The viewer's GitHub login when an account is linked, including one whose token was rejected.
    pub login: Option<String>,
    pub account: GithubAccountLink,
    pub can_comment: bool,
    pub can_review: bool,
    pub can_request_reviewers: bool,
    pub status: GithubPullRequestStatus,
}

/// Whether the viewer has a GitHub account the write actions can use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum GithubAccountLink {
    /// No linked account: the classic page says "Connect GitHub".
    None,
    /// Linked, and the stored token can be used.
    Connected,
    /// Linked, but the token was rejected or can't be read: "Reconnect GitHub".
    Rejected,
}

/// `POST /api/v1/rooms/:roomId/github/pull_requests/:id/comments`
/// (`rooms/github/pull_request_comments#create`). Posts an issue comment as the viewer.
/// Answers [`GithubWriteResult`]. 422 when the body is blank, GitHub refuses, or the account
/// can't be used.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CreateGithubComment {
    pub body: String,
}

/// The review the SPA dialog submits. `comment` is a GitHub review event `COMMENT` (a note on
/// the pull request); the classic form only offers approve and request-changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum GithubReviewKind {
    Approve,
    RequestChanges,
    Comment,
}

/// `POST /api/v1/rooms/:roomId/github/pull_requests/:id/reviews`
/// (`rooms/github/pull_request_reviews#create`, plus a `comment` review). `body` may be empty
/// for an approval; requesting changes or commenting requires it. Answers [`GithubWriteResult`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CreateGithubReview {
    pub event: GithubReviewKind,
    #[serde(default)]
    pub body: String,
}

/// `POST /api/v1/rooms/:roomId/github/pull_requests/:id/review_requests`
/// (`rooms/github/pull_request_review_requests#create`). GitHub usernames separated by commas
/// or whitespace; a leading `@` is optional. Answers [`GithubWriteResult`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CreateGithubReviewRequest {
    pub reviewers: String,
}

/// The confirmation the classic page shows after a write (`Outcome::notice`). The card itself
/// updates when GitHub's webhook publishes `message.cards`, the same way the classic card frame
/// is replaced on the `github_cards` channel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GithubWriteResult {
    pub notice: String,
}

/// `POST /api/v1/rooms/:roomId/github/pull_requests/:id/discussion`
/// (`github/pull_request_threads#create`). The message that links this pull request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CreateGithubDiscussion {
    pub message_id: i64,
}

/// The room's discussion thread for the pull request, created or reused by
/// [`CreateGithubDiscussion`]. `threadId` is that thread.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GithubDiscussion {
    pub thread_id: i64,
}

/// A LinkedIn post (a `link_embeds` row for a LinkedIn URL;
/// `crates/app/src/integrations/linkedin.rs`). Unlike [`LinkCard`] it's always shown: without
/// a title or description it's the classic "View post on LinkedIn" chip.
///
/// Fill order: 8.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LinkedinCard {
    pub url: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub image_url: Option<String>,
    /// LinkedIn's embeddable player for the post's URN, for "Show embedded post"; `null` when
    /// the URL has none.
    pub embed_url: Option<String>,
}

/// A Fizzy card the message links to (`fizzy_card_references`). Fizzy content is fetched with
/// each viewer's own Fizzy token and cached per viewer, so the client fetches
/// `GET /api/v1/rooms/:roomId/fizzy/cards/:fizzyCardId/card?messageId=` ([`FizzyCardPreview`]),
/// as the classic lazy frame does (`controllers/fizzy_cards.rs`).
///
/// Fill order: 9.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FizzyCardRef {
    /// `fizzy_cards.id`.
    pub fizzy_card_id: i64,
    pub account_id: String,
    pub number: i64,
    /// The card's web address as linked.
    pub url: String,
}

/// The reply to `GET /api/v1/rooms/:roomId/fizzy/cards/:id/card?messageId=`
/// (`rooms/fizzy/cards#show`): the card as the viewer's Fizzy account sees it. Tagged by
/// `state`. 404 unless the viewer is a member of `roomId`, `messageId` is given and is a message
/// in that room, and the message references this card (`fizzy_card_references`). Asking also
/// queues a refresh of the viewer's cached copy when they're connected, as the classic frame
/// does; the result arrives on the next fetch, not as an event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(
    tag = "state",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
#[ts(export)]
pub enum FizzyCardPreview {
    /// The viewer hasn't connected Fizzy: "Connect Fizzy to preview".
    NotConnected,
    /// Fizzy doesn't have it, or not for this viewer.
    NotFound,
    Loading,
    Failed {
        message: String,
    },
    Loaded(FizzyCard),
}

/// The fields the classic card reads from Fizzy's card JSON (`campfire_views::fizzy_cards`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FizzyCard {
    pub title: String,
    pub url: String,
    pub board_name: Option<String>,
    pub status: FizzyCardStatus,
    /// The column's name when `status` is `column`; `null` otherwise.
    pub column_name: Option<String>,
    pub assignees: Vec<FizzyAssignee>,
    /// Fizzy left some assignees out ("+ more").
    pub has_more_assignees: bool,
    pub tags: Vec<String>,
    pub steps_total: i64,
    pub steps_completed: i64,
    pub last_active_at: Option<Timestamp>,
}

/// Derived in this order: `closed`, `postponed`, in a `column`, else `triage` ("Maybe?").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum FizzyCardStatus {
    Closed,
    Postponed,
    Column,
    Triage,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FizzyAssignee {
    pub name: String,
    /// HTTPS only; `null` otherwise.
    pub avatar_url: Option<String>,
}

/// The `message.cards` event on the message's conversation topic: its cards changed (a fetch
/// finished, a pull request or post was refreshed, an event it links to was edited, cancelled
/// or reminded, previews were suppressed, or the body was edited to link something else). The
/// JSON twin of the classic slot replaces (`drive_attachments`, `github_pr_cards`,
/// `twitter_cards`, `[message, :event_cards]`, `fizzy_cards`, `message_link_cards`,
/// `linkedin_cards`, `link_embed_cards`). Replaces the message's `cards` without touching its
/// `updatedAt`, when `asOf` is at least the stored `cardsAsOf` (see the module's **Ordering**).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MessageCards {
    pub message_id: i64,
    pub room_id: i64,
    pub thread_id: Option<i64>,
    pub cards: Vec<MessageCard>,
    /// When the server read these cards.
    pub as_of: Timestamp,
}

/// The `poll.updated` event on the poll message's conversation topic (`room:<id>`, or
/// `thread:<id>` for a poll posted in a thread): a vote changed the counts, or the poll closed.
/// Applies when `poll.asOf` is at least the stored poll's (see the module's **Ordering**).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PollUpdated {
    pub room_id: i64,
    pub thread_id: Option<i64>,
    pub poll: Poll,
}

/// The `poll.ballot` event on the voter's `user` topic: their own choice changed in some tab.
/// Needed for anonymous polls, whose `voterIds` are always empty, so other tabs can't work out
/// the viewer's vote from `poll.updated`; sent for every poll so the client has one rule.
/// New: the classic app re-renders the voter's card only in the tab that voted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PollBallot {
    pub poll_id: i64,
    pub message_id: i64,
    pub room_id: i64,
    pub thread_id: Option<i64>,
    /// The options the viewer chose now; empty after taking their vote back.
    pub my_option_ids: Vec<i64>,
    /// When the server read the ballot; keep the latest per poll.
    pub as_of: Timestamp,
}
