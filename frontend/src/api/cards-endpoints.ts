/**
 * The S3 card endpoints: polls (results, create, vote), event attendance (read, respond), and the
 * per-viewer previews of GitHub pull requests, Fizzy cards and quoted messages. Every path is
 * room-scoped, as the contract has it (crates/api_types/src/cards.rs). Each validates the reply
 * with its pinned schema and returns the wire value.
 */
import { Effect } from "effect";
import type { AttendanceResponse } from "../gen/AttendanceResponse.ts";
import type { CreatePoll } from "../gen/CreatePoll.ts";
import type { EventAttendance } from "../gen/EventAttendance.ts";
import type { FizzyCardPreview } from "../gen/FizzyCardPreview.ts";
import type { GithubDiscussion } from "../gen/GithubDiscussion.ts";
import type { GithubPullRequestActions } from "../gen/GithubPullRequestActions.ts";
import type { GithubPullRequestCard } from "../gen/GithubPullRequestCard.ts";
import type { GithubReviewKind } from "../gen/GithubReviewKind.ts";
import type { GithubWriteResult } from "../gen/GithubWriteResult.ts";
import type { MessageDTO } from "../gen/MessageDTO.ts";
import type { PollResults } from "../gen/PollResults.ts";
import type { QuotePreviewResult } from "../gen/QuotePreviewResult.ts";
import { call, get } from "./call.ts";
import {
  EventAttendance as EventAttendanceSchema,
  FizzyCardPreview as FizzyCardPreviewSchema,
  GithubDiscussion as GithubDiscussionSchema,
  GithubPullRequestActions as GithubPullRequestActionsSchema,
  GithubPullRequestCard as GithubPullRequestCardSchema,
  GithubWriteResult as GithubWriteResultSchema,
  PollResults as PollResultsSchema,
  QuotePreviewResult as QuotePreviewResultSchema,
} from "./schema/cards.ts";
import { MessageDTO as MessageSchema } from "./schema/message.ts";
import { wire } from "./wire.ts";

/**
 * Which GitHub card to fetch: the one under a message, or the header of the pull request's
 * discussion thread (which also lists the changed files).
 */
export type GithubCardScope = { readonly messageId: number } | { readonly threadId: number };

function githubScopeQuery(scope: GithubCardScope): Readonly<Record<string, string>> {
  return "messageId" in scope
    ? { messageId: String(scope.messageId) }
    : { threadId: String(scope.threadId) };
}

/** `GET /rooms/:roomId/polls/:id`: the poll with the viewer's own choice. */
export const pollResults = Effect.fn("api.pollResults")(function* (roomId: number, pollId: number) {
  return yield* call(get(`/rooms/${roomId}/polls/${pollId}`), wire<PollResults>(PollResultsSchema));
});

/**
 * `POST /rooms/:roomId/polls`: posts the question with its poll. Idempotent on
 * `clientMessageId`: a retry answers the question already created.
 */
export const createPoll = Effect.fn("api.createPoll")(function* (roomId: number, body: CreatePoll) {
  return yield* call(
    {
      method: "POST",
      path: `/rooms/${roomId}/polls`,
      body: { ...body, options: [...body.options] },
    },
    wire<MessageDTO>(MessageSchema),
  );
});

/** `POST /rooms/:roomId/polls/:id/vote`: replaces the viewer's whole ballot; empty retracts. */
export const votePoll = Effect.fn("api.votePoll")(function* (
  roomId: number,
  pollId: number,
  optionIds: readonly number[],
) {
  return yield* call(
    {
      method: "POST",
      path: `/rooms/${roomId}/polls/${pollId}/vote`,
      body: { optionIds: [...optionIds] },
    },
    wire<PollResults>(PollResultsSchema),
  );
});

/** `POST /rooms/:roomId/polls/:id/end`: author or administrator closes voting. */
export const endPoll = Effect.fn("api.endPoll")(function* (roomId: number, pollId: number) {
  return yield* call(
    { method: "POST", path: `/rooms/${roomId}/polls/${pollId}/end`, body: {} },
    wire<PollResults>(PollResultsSchema),
  );
});

/** `GET /rooms/:roomId/events/:id/attendance`: the viewer's response and the counts. */
export const eventAttendance = Effect.fn("api.eventAttendance")(function* (
  roomId: number,
  eventId: number,
) {
  return yield* call(
    get(`/rooms/${roomId}/events/${eventId}/attendance`),
    wire<EventAttendance>(EventAttendanceSchema),
  );
});

/** `PUT /rooms/:roomId/events/:id/attendance`: going, maybe or declined. */
export const respondToEvent = Effect.fn("api.respondToEvent")(function* (
  roomId: number,
  eventId: number,
  response: AttendanceResponse,
  applyToFuture: boolean,
) {
  return yield* call(
    {
      method: "PUT",
      path: `/rooms/${roomId}/events/${eventId}/attendance`,
      body: { response, applyToFuture },
    },
    wire<EventAttendance>(EventAttendanceSchema),
  );
});

/** `GET /rooms/:roomId/github/pull_requests/:id/card?messageId=|threadId=`. */
export const githubCard = Effect.fn("api.githubCard")(function* (
  roomId: number,
  pullRequestId: number,
  scope: GithubCardScope,
) {
  return yield* call(
    get(`/rooms/${roomId}/github/pull_requests/${pullRequestId}/card`, githubScopeQuery(scope)),
    wire<GithubPullRequestCard>(GithubPullRequestCardSchema),
  );
});

/**
 * `POST /rooms/:roomId/github/pull_requests/:id/discussion`: the classic Discuss action.
 * Creates the thread and the pull-request mapping, or returns the thread already mapped.
 */
export const discussGithub = Effect.fn("api.discussGithub")(function* (
  roomId: number,
  pullRequestId: number,
  messageId: number,
) {
  return yield* call(
    {
      method: "POST",
      path: `/rooms/${roomId}/github/pull_requests/${pullRequestId}/discussion`,
      body: { messageId },
    },
    wire<GithubDiscussion>(GithubDiscussionSchema),
  );
});

/** `GET /rooms/:roomId/github/pull_requests/:id/actions`: what this viewer can post. */
export const githubActions = Effect.fn("api.githubActions")(function* (
  roomId: number,
  pullRequestId: number,
) {
  return yield* call(
    get(`/rooms/${roomId}/github/pull_requests/${pullRequestId}/actions`),
    wire<GithubPullRequestActions>(GithubPullRequestActionsSchema),
  );
});

/** `POST /rooms/:roomId/github/pull_requests/:id/comments`: an issue comment as the viewer. */
export const commentOnGithub = Effect.fn("api.commentOnGithub")(function* (
  roomId: number,
  pullRequestId: number,
  body: string,
) {
  return yield* call(
    {
      method: "POST",
      path: `/rooms/${roomId}/github/pull_requests/${pullRequestId}/comments`,
      body: { body },
    },
    wire<GithubWriteResult>(GithubWriteResultSchema),
  );
});

/** `POST /rooms/:roomId/github/pull_requests/:id/reviews`: approve, request changes, or comment. */
export const reviewGithub = Effect.fn("api.reviewGithub")(function* (
  roomId: number,
  pullRequestId: number,
  event: GithubReviewKind,
  body: string,
) {
  return yield* call(
    {
      method: "POST",
      path: `/rooms/${roomId}/github/pull_requests/${pullRequestId}/reviews`,
      body: { event, body },
    },
    wire<GithubWriteResult>(GithubWriteResultSchema),
  );
});

/** `POST /rooms/:roomId/github/pull_requests/:id/review_requests`: comma-separated GitHub logins. */
export const requestGithubReviewers = Effect.fn("api.requestGithubReviewers")(function* (
  roomId: number,
  pullRequestId: number,
  reviewers: string,
) {
  return yield* call(
    {
      method: "POST",
      path: `/rooms/${roomId}/github/pull_requests/${pullRequestId}/review_requests`,
      body: { reviewers },
    },
    wire<GithubWriteResult>(GithubWriteResultSchema),
  );
});

/** `GET /rooms/:roomId/fizzy/cards/:id/card?messageId=`: also queues a refresh when connected. */
export const fizzyCard = Effect.fn("api.fizzyCard")(function* (
  roomId: number,
  fizzyCardId: number,
  messageId: number,
) {
  return yield* call(
    get(`/rooms/${roomId}/fizzy/cards/${fizzyCardId}/card`, { messageId: String(messageId) }),
    wire<FizzyCardPreview>(FizzyCardPreviewSchema),
  );
});

/** `GET /rooms/:roomId/message_links/:referenceId/card`: a quoted message, if the viewer sees it. */
export const quoteCard = Effect.fn("api.quoteCard")(function* (
  roomId: number,
  referenceId: number,
) {
  return yield* call(
    get(`/rooms/${roomId}/message_links/${referenceId}/card`),
    wire<QuotePreviewResult>(QuotePreviewResultSchema),
  );
});
