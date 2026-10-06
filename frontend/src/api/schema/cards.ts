import { Schema } from "effect";
import type { AttendanceResponse as GeneratedAttendanceResponse } from "../../gen/AttendanceResponse.ts";
import type { CardFetch as GeneratedCardFetch } from "../../gen/CardFetch.ts";
import type { CreatePoll as GeneratedCreatePoll } from "../../gen/CreatePoll.ts";
import type { EventAttendance as GeneratedEventAttendance } from "../../gen/EventAttendance.ts";
import type { EventCard as GeneratedEventCard } from "../../gen/EventCard.ts";
import type { FizzyAssignee as GeneratedFizzyAssignee } from "../../gen/FizzyAssignee.ts";
import type { FizzyCard as GeneratedFizzyCard } from "../../gen/FizzyCard.ts";
import type { FizzyCardPreview as GeneratedFizzyCardPreview } from "../../gen/FizzyCardPreview.ts";
import type { FizzyCardRef as GeneratedFizzyCardRef } from "../../gen/FizzyCardRef.ts";
import type { FizzyCardStatus as GeneratedFizzyCardStatus } from "../../gen/FizzyCardStatus.ts";
import type { GithubCardRef as GeneratedGithubCardRef } from "../../gen/GithubCardRef.ts";
import type { GithubChecks as GeneratedGithubChecks } from "../../gen/GithubChecks.ts";
import type { GithubPullRequest as GeneratedGithubPullRequest } from "../../gen/GithubPullRequest.ts";
import type { GithubPullRequestCard as GeneratedGithubPullRequestCard } from "../../gen/GithubPullRequestCard.ts";
import type { GithubPullRequestStatus as GeneratedGithubPullRequestStatus } from "../../gen/GithubPullRequestStatus.ts";
import type { GithubReview as GeneratedGithubReview } from "../../gen/GithubReview.ts";
import type { LinkCard as GeneratedLinkCard } from "../../gen/LinkCard.ts";
import type { LinkedinCard as GeneratedLinkedinCard } from "../../gen/LinkedinCard.ts";
import type { MessageCard as GeneratedMessageCard } from "../../gen/MessageCard.ts";
import type { MessageCards as GeneratedMessageCards } from "../../gen/MessageCards.ts";
import type { Poll as GeneratedPoll } from "../../gen/Poll.ts";
import type { PollOption as GeneratedPollOption } from "../../gen/PollOption.ts";
import type { PollResults as GeneratedPollResults } from "../../gen/PollResults.ts";
import type { RespondToEvent as GeneratedRespondToEvent } from "../../gen/RespondToEvent.ts";
import type { VotePoll as GeneratedVotePoll } from "../../gen/VotePoll.ts";
import type { XMedia as GeneratedXMedia } from "../../gen/XMedia.ts";
import type { XMediaKind as GeneratedXMediaKind } from "../../gen/XMediaKind.ts";
import type { XPostCard as GeneratedXPostCard } from "../../gen/XPostCard.ts";
import type { XQuote as GeneratedXQuote } from "../../gen/XQuote.ts";
import {
  EventId,
  FizzyCardId,
  GithubPullRequestId,
  MessageId,
  PollId,
  PollOptionId,
  RoomId,
  ThreadId,
  UserId,
} from "./ids.ts";
import type { Assert, Pinned } from "./pin.ts";
import { Timestamp } from "./time.ts";

export const PollOption = Schema.Struct({
  id: PollOptionId,
  label: Schema.String,
  votes: Schema.Int,
  voterIds: Schema.Array(UserId),
});

export type PollOption = typeof PollOption.Type;

export type PollOptionPin = Assert<Pinned<typeof PollOption, GeneratedPollOption>>;

/**
 * A poll on its question message. `voterIds` are empty when it's anonymous; treat it as closed
 * once `closesAt` passes, before `poll.updated` says so. Server fill order: 1.
 */
export const Poll = Schema.Struct({
  id: PollId,
  messageId: MessageId,
  multiple: Schema.Boolean,
  anonymous: Schema.Boolean,
  closesAt: Schema.NullOr(Timestamp),
  closedAt: Schema.NullOr(Timestamp),
  closed: Schema.Boolean,
  totalVotes: Schema.Int,
  options: Schema.Array(PollOption),
});

export type Poll = typeof Poll.Type;

export type PollPin = Assert<Pinned<typeof Poll, GeneratedPoll>>;

/** `GET /api/v1/rooms/:roomId/polls/:id`, and a vote's reply: the poll with the viewer's choice. */
export const PollResults = Schema.Struct({ poll: Poll, myOptionIds: Schema.Array(PollOptionId) });

export type PollResults = typeof PollResults.Type;

export type PollResultsPin = Assert<Pinned<typeof PollResults, GeneratedPollResults>>;

/** The body of `POST /api/v1/rooms/:roomId/polls`: 2 to 10 options of up to 200 characters. */
export const CreatePoll = Schema.Struct({
  question: Schema.String,
  options: Schema.Array(Schema.String),
  multiple: Schema.Boolean,
  anonymous: Schema.Boolean,
  closesAt: Schema.NullOr(Timestamp),
});

export type CreatePoll = typeof CreatePoll.Type;

export type CreatePollPin = Assert<Pinned<typeof CreatePoll, GeneratedCreatePoll>>;

/** The body of `POST /api/v1/rooms/:roomId/polls/:id/vote`: the whole ballot; empty retracts. */
export const VotePoll = Schema.Struct({ optionIds: Schema.Array(PollOptionId) });

export type VotePoll = typeof VotePoll.Type;

export type VotePollPin = Assert<Pinned<typeof VotePoll, GeneratedVotePoll>>;

export const CardFetch = Schema.Literals(["loading", "loaded", "failed"]);

export type CardFetch = typeof CardFetch.Type;

export type CardFetchPin = Assert<Pinned<typeof CardFetch, GeneratedCardFetch>>;

export const XMediaKind = Schema.Literals(["photo", "video", "gif"]);

export type XMediaKindPin = Assert<Pinned<typeof XMediaKind, GeneratedXMediaKind>>;

export const XMedia = Schema.Struct({
  kind: XMediaKind,
  url: Schema.String,
  thumbnailUrl: Schema.NullOr(Schema.String),
  width: Schema.NullOr(Schema.Int),
  height: Schema.NullOr(Schema.Int),
  alt: Schema.NullOr(Schema.String),
});

export type XMedia = typeof XMedia.Type;

export type XMediaPin = Assert<Pinned<typeof XMedia, GeneratedXMedia>>;

export const XQuote = Schema.Struct({
  url: Schema.NullOr(Schema.String),
  authorName: Schema.NullOr(Schema.String),
  authorHandle: Schema.NullOr(Schema.String),
  text: Schema.NullOr(Schema.String),
});

export type XQuote = typeof XQuote.Type;

export type XQuotePin = Assert<Pinned<typeof XQuote, GeneratedXQuote>>;

/** A post on X, the same for every viewer. Server fill order: 2. */
export const XPostCard = Schema.Struct({
  fetch: CardFetch,
  postId: Schema.String,
  url: Schema.String,
  authorName: Schema.String,
  authorHandle: Schema.NullOr(Schema.String),
  authorAvatarUrl: Schema.NullOr(Schema.String),
  text: Schema.NullOr(Schema.String),
  postedAt: Schema.NullOr(Timestamp),
  replies: Schema.NullOr(Schema.Int),
  reposts: Schema.NullOr(Schema.Int),
  likes: Schema.NullOr(Schema.Int),
  media: Schema.Array(XMedia),
  quote: Schema.NullOr(XQuote),
});

export type XPostCard = typeof XPostCard.Type;

export type XPostCardPin = Assert<Pinned<typeof XPostCard, GeneratedXPostCard>>;

/** Any other page's preview; only pages with a title or description get one. Fill order: 3. */
export const LinkCard = Schema.Struct({
  url: Schema.String,
  siteName: Schema.NullOr(Schema.String),
  title: Schema.NullOr(Schema.String),
  description: Schema.NullOr(Schema.String),
  imageUrl: Schema.NullOr(Schema.String),
});

export type LinkCard = typeof LinkCard.Type;

export type LinkCardPin = Assert<Pinned<typeof LinkCard, GeneratedLinkCard>>;

/** A calendar event the message links to. Attendance is fetched per viewer. Fill order: 4. */
export const EventCard = Schema.Struct({
  eventId: EventId,
  roomId: RoomId,
  title: Schema.String,
  organizerId: UserId,
  startsAt: Timestamp,
  endsAt: Schema.NullOr(Timestamp),
  timeZone: Schema.String,
  recurring: Schema.Boolean,
  cancelled: Schema.Boolean,
  venueRoomId: Schema.NullOr(RoomId),
  venueName: Schema.NullOr(Schema.String),
  meetLink: Schema.NullOr(Schema.String),
});

export type EventCard = typeof EventCard.Type;

export type EventCardPin = Assert<Pinned<typeof EventCard, GeneratedEventCard>>;

export const AttendanceResponse = Schema.Literals(["going", "maybe", "declined"]);

export type AttendanceResponse = typeof AttendanceResponse.Type;

export type AttendanceResponsePin = Assert<
  Pinned<typeof AttendanceResponse, GeneratedAttendanceResponse>
>;

/** `GET /api/v1/rooms/:roomId/events/:id/attendance`, and a response's reply. */
export const EventAttendance = Schema.Struct({
  eventId: EventId,
  response: Schema.NullOr(AttendanceResponse),
  goingCount: Schema.Int,
  maybeCount: Schema.Int,
  declinedCount: Schema.Int,
  respondable: Schema.Boolean,
  canApplyToFuture: Schema.Boolean,
});

export type EventAttendance = typeof EventAttendance.Type;

export type EventAttendancePin = Assert<Pinned<typeof EventAttendance, GeneratedEventAttendance>>;

/** The body of `PUT /api/v1/rooms/:roomId/events/:id/attendance`. */
export const RespondToEvent = Schema.Struct({
  response: AttendanceResponse,
  applyToFuture: Schema.Boolean,
});

export type RespondToEvent = typeof RespondToEvent.Type;

export type RespondToEventPin = Assert<Pinned<typeof RespondToEvent, GeneratedRespondToEvent>>;

/** A pull request link; fetch its card per viewer. Server fill order: 5. */
export const GithubCardRef = Schema.Struct({
  pullRequestId: GithubPullRequestId,
  owner: Schema.String,
  repo: Schema.String,
  number: Schema.Int,
  url: Schema.String,
});

export type GithubCardRef = typeof GithubCardRef.Type;

export type GithubCardRefPin = Assert<Pinned<typeof GithubCardRef, GeneratedGithubCardRef>>;

export const GithubPullRequestStatus = Schema.Literals(["open", "draft", "merged", "closed"]);

export type GithubPullRequestStatusPin = Assert<
  Pinned<typeof GithubPullRequestStatus, GeneratedGithubPullRequestStatus>
>;

export const GithubReview = Schema.Literals(["approved", "changes_requested", "review_required"]);

export type GithubReviewPin = Assert<Pinned<typeof GithubReview, GeneratedGithubReview>>;

export const GithubChecks = Schema.Literals(["passing", "pending", "failing"]);

export type GithubChecksPin = Assert<Pinned<typeof GithubChecks, GeneratedGithubChecks>>;

const githubPullRequestFields = {
  owner: Schema.String,
  repo: Schema.String,
  number: Schema.Int,
  title: Schema.String,
  url: Schema.String,
  status: GithubPullRequestStatus,
  authorLogin: Schema.NullOr(Schema.String),
  authorAvatarUrl: Schema.NullOr(Schema.String),
  baseBranch: Schema.NullOr(Schema.String),
  headBranch: Schema.NullOr(Schema.String),
  review: Schema.NullOr(GithubReview),
  checks: Schema.NullOr(GithubChecks),
  githubUpdatedAt: Schema.NullOr(Timestamp),
  discussionThreadId: Schema.NullOr(ThreadId),
};

export const GithubPullRequest = Schema.Struct(githubPullRequestFields);

export type GithubPullRequest = typeof GithubPullRequest.Type;

export type GithubPullRequestPin = Assert<
  Pinned<typeof GithubPullRequest, GeneratedGithubPullRequest>
>;

/** `GET /api/v1/github/pull_requests/:id/card?messageId=`: the card as the viewer may see it. */
export const GithubPullRequestCard = Schema.Union([
  Schema.Struct({ state: Schema.Literal("hidden") }),
  Schema.Struct({ state: Schema.Literal("loading") }),
  Schema.Struct({ state: Schema.Literal("failed"), message: Schema.String }),
  Schema.Struct({ state: Schema.Literal("loaded"), ...githubPullRequestFields }),
]);

export type GithubPullRequestCard = typeof GithubPullRequestCard.Type;

export type GithubPullRequestCardPin = Assert<
  Pinned<typeof GithubPullRequestCard, GeneratedGithubPullRequestCard>
>;

/** A LinkedIn post; without a title or description it's a plain chip. Fill order: 6. */
export const LinkedinCard = Schema.Struct({
  url: Schema.String,
  title: Schema.NullOr(Schema.String),
  description: Schema.NullOr(Schema.String),
  imageUrl: Schema.NullOr(Schema.String),
  embedUrl: Schema.NullOr(Schema.String),
});

export type LinkedinCard = typeof LinkedinCard.Type;

export type LinkedinCardPin = Assert<Pinned<typeof LinkedinCard, GeneratedLinkedinCard>>;

/** A Fizzy card link; fetch its preview per viewer. Server fill order: 7. */
export const FizzyCardRef = Schema.Struct({
  fizzyCardId: FizzyCardId,
  accountId: Schema.String,
  number: Schema.Int,
  url: Schema.String,
});

export type FizzyCardRef = typeof FizzyCardRef.Type;

export type FizzyCardRefPin = Assert<Pinned<typeof FizzyCardRef, GeneratedFizzyCardRef>>;

export const FizzyCardStatus = Schema.Literals(["closed", "postponed", "column", "triage"]);

export type FizzyCardStatusPin = Assert<Pinned<typeof FizzyCardStatus, GeneratedFizzyCardStatus>>;

export const FizzyAssignee = Schema.Struct({
  name: Schema.String,
  avatarUrl: Schema.NullOr(Schema.String),
});

export type FizzyAssigneePin = Assert<Pinned<typeof FizzyAssignee, GeneratedFizzyAssignee>>;

const fizzyCardFields = {
  title: Schema.String,
  url: Schema.String,
  boardName: Schema.NullOr(Schema.String),
  status: FizzyCardStatus,
  columnName: Schema.NullOr(Schema.String),
  assignees: Schema.Array(FizzyAssignee),
  hasMoreAssignees: Schema.Boolean,
  tags: Schema.Array(Schema.String),
  stepsTotal: Schema.Int,
  stepsCompleted: Schema.Int,
  lastActiveAt: Schema.NullOr(Timestamp),
};

export const FizzyCard = Schema.Struct(fizzyCardFields);

export type FizzyCard = typeof FizzyCard.Type;

export type FizzyCardPin = Assert<Pinned<typeof FizzyCard, GeneratedFizzyCard>>;

/** `GET /api/v1/fizzy/cards/:id/card?messageId=`: the card as the viewer's Fizzy account sees it. */
export const FizzyCardPreview = Schema.Union([
  Schema.Struct({ state: Schema.Literal("not_connected") }),
  Schema.Struct({ state: Schema.Literal("not_found") }),
  Schema.Struct({ state: Schema.Literal("loading") }),
  Schema.Struct({ state: Schema.Literal("failed"), message: Schema.String }),
  Schema.Struct({ state: Schema.Literal("loaded"), ...fizzyCardFields }),
]);

export type FizzyCardPreview = typeof FizzyCardPreview.Type;

export type FizzyCardPreviewPin = Assert<
  Pinned<typeof FizzyCardPreview, GeneratedFizzyCardPreview>
>;

/**
 * One card under a message, in the classic slot order. LinkedIn and link cards are absent while
 * `embedsSuppressed`; a kind the server doesn't fill yet is absent too.
 */
export const MessageCard = Schema.Union([
  Schema.Struct({ kind: Schema.Literal("github"), data: GithubCardRef }),
  Schema.Struct({ kind: Schema.Literal("x"), data: XPostCard }),
  Schema.Struct({ kind: Schema.Literal("event"), data: EventCard }),
  Schema.Struct({ kind: Schema.Literal("fizzy"), data: FizzyCardRef }),
  Schema.Struct({ kind: Schema.Literal("linkedin"), data: LinkedinCard }),
  Schema.Struct({ kind: Schema.Literal("link"), data: LinkCard }),
]);

export type MessageCard = typeof MessageCard.Type;

export type MessageCardPin = Assert<Pinned<typeof MessageCard, GeneratedMessageCard>>;

/** The `message.cards` event: replaces a message's `cards` without touching `updatedAt`. */
export const MessageCards = Schema.Struct({
  messageId: MessageId,
  roomId: RoomId,
  threadId: Schema.NullOr(ThreadId),
  cards: Schema.Array(MessageCard),
});

export type MessageCards = typeof MessageCards.Type;

export type MessageCardsPin = Assert<Pinned<typeof MessageCards, GeneratedMessageCards>>;
