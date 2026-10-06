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
//! 2. posts on X ([`XPostCard`]);
//! 3. other web pages ([`LinkCard`]);
//! 4. calendar events ([`EventCard`], with [`EventAttendance`]);
//! 5. GitHub pull requests ([`GithubCardRef`] → [`GithubPullRequestCard`]);
//! 6. LinkedIn posts ([`LinkedinCard`]);
//! 7. Fizzy cards ([`FizzyCardRef`] → [`FizzyCardPreview`]).
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
/// 422 when the question is blank, there are fewer than 2 or more than 10 non-blank options,
/// a label is over 200 characters, or `closesAt` isn't in the future.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CreatePoll {
    /// Markdown, posted as the message.
    pub question: String,
    pub options: Vec<String>,
    pub multiple: bool,
    pub anonymous: bool,
    pub closes_at: Option<Timestamp>,
}

/// `POST /api/v1/rooms/:roomId/polls/:id/vote`: replace the viewer's whole ballot
/// (`rooms/polls#vote`, `Poll#cast_vote`); an empty list takes their vote back. Answers
/// [`PollResults`] and publishes `poll.updated`. 422 when the poll is closed, an option isn't
/// the poll's, or a single-choice poll gets more than one. Active human members only (403).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct VotePoll {
    pub option_ids: Vec<i64>,
}

/// One card under a message, tagged by `kind` with its body in `data`. In the classic slot
/// order: GitHub pull requests, posts on X, events, Fizzy cards, LinkedIn posts, then other
/// pages, each in the order the links appear in the body.
///
/// LinkedIn and other-page cards are left out while the message's `embedsSuppressed` is set
/// (the author hid previews); the others ignore it, as in the classic app.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", content = "data", rename_all = "lowercase")]
#[ts(export)]
pub enum MessageCard {
    Github(GithubCardRef),
    X(XPostCard),
    Event(EventCard),
    Fizzy(FizzyCardRef),
    Linkedin(LinkedinCard),
    Link(LinkCard),
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
/// Fill order: 2.
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
/// Fill order: 3.
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
/// Fill order: 4.
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
/// client fetches `GET /api/v1/github/pull_requests/:pullRequestId/card?messageId=`
/// ([`GithubPullRequestCard`]), as the classic lazy frame does
/// (`controllers/github/cards.rs`). Issues, commits and discussions get no card.
///
/// Fill order: 5.
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

/// The reply to `GET /api/v1/github/pull_requests/:id/card?messageId=`: the pull request as the
/// viewer may see it (`Github::PullRequest#visible_to`: a public repository, or one the
/// viewer's connected GitHub account can read). Tagged by `state`.
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

/// A LinkedIn post (a `link_embeds` row for a LinkedIn URL;
/// `crates/app/src/integrations/linkedin.rs`). Unlike [`LinkCard`] it's always shown: without
/// a title or description it's the classic "View post on LinkedIn" chip.
///
/// Fill order: 6.
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
/// `GET /api/v1/fizzy/cards/:fizzyCardId/card?messageId=` ([`FizzyCardPreview`]), as the
/// classic lazy frame does (`controllers/fizzy_cards.rs`).
///
/// Fill order: 7.
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

/// The reply to `GET /api/v1/fizzy/cards/:id/card?messageId=`: the card as the viewer's Fizzy
/// account sees it. Tagged by `state`.
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
/// JSON twin of the classic slot replaces (`github_pr_cards`, `twitter_cards`,
/// `[message, :event_cards]`, `fizzy_cards`, `linkedin_cards`, `link_embed_cards`). Replaces
/// the message's `cards` without touching its `updatedAt`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MessageCards {
    pub message_id: i64,
    pub room_id: i64,
    pub thread_id: Option<i64>,
    pub cards: Vec<MessageCard>,
}
