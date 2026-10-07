/**
 * The card programs React reaches through `actions.cards`: poll results, votes (shown at once,
 * put back when refused), poll creation, event attendance, and the per-viewer previews of
 * GitHub pull requests, Fizzy cards and quoted messages. Every reply lands in the store; loads
 * record their failures there, writes also reject.
 */
import { Clock, Duration, Effect } from "effect";
import * as api from "../api/cards-endpoints.ts";
import type { AttendanceResponse } from "../gen/AttendanceResponse.ts";
import type { CreatePoll } from "../gen/CreatePoll.ts";
import type { FizzyCardPreview } from "../gen/FizzyCardPreview.ts";
import type { GithubPullRequestCard } from "../gen/GithubPullRequestCard.ts";
import { cardMutations } from "../store/card-mutations.ts";
import {
  attendanceKey,
  fizzyKey,
  githubKey,
  type PreviewKind,
  type PreviewValues,
  quoteKey,
  withResponse,
} from "../store/cards.ts";
import { mutations, store } from "../store/store.ts";
import { keyedSerial } from "./serial.ts";

/** How often a preview the server is still fetching is asked for again, and how long between. */
const STILL_LOADING_RETRIES = 4;

const FIRST_RETRY = Duration.seconds(2);

/** Ballots go out one at a time per poll, so the server sees them in the order they were cast. */
const votes = keyedSerial<number>();

/** Answers go out one at a time per event, so each rolls back to a copy no other is holding. */
const answers = keyedSerial<number>();

/** Fetches the viewer's results for a poll (its choice in an anonymous poll comes only here). */
export const loadPoll = Effect.fn("cards.loadPoll")(function* (roomId: number, pollId: number) {
  if (store.getState().cards.pollLoads[pollId] === "loading") {
    return;
  }

  cardMutations.setPollLoad(pollId, "loading");

  const results = yield* api
    .pollResults(roomId, pollId)
    .pipe(Effect.tapError(() => Effect.sync(() => cardMutations.setPollLoad(pollId, "error"))));

  cardMutations.applyPollResults(results);
});

/**
 * Sends the viewer's whole ballot (`[]` takes the vote back). The counts move at once; a refusal
 * (closed, or not a member any more) puts them back, unless a later ballot has replaced this one,
 * and rejects.
 */
export const vote = Effect.fn("cards.vote")(function* (
  roomId: number,
  pollId: number,
  optionIds: readonly number[],
) {
  const sent = [...optionIds];

  cardMutations.setPendingVote(pollId, sent);

  const results = yield* votes.run(
    pollId,
    api
      .votePoll(roomId, pollId, sent)
      .pipe(Effect.onError(() => Effect.sync(() => cardMutations.settleVote(pollId, sent, null)))),
  );

  cardMutations.settleVote(pollId, sent, results);
});

/**
 * Posts a question with its poll. Pass the same `clientMessageId` on a retry: the server answers
 * the question it already made. The question joins the timeline like any sent message.
 */
export const createPoll = Effect.fn("cards.createPoll")(function* (
  roomId: number,
  body: CreatePoll,
) {
  const message = yield* api.createPoll(roomId, body);

  mutations.receiveMessage(message);

  return message;
});

/**
 * Loads one preview into the store under `key`, asking again (2 s, 4 s, 8 s…) while the server
 * answers that it's still fetching it.
 */
const loadPreview = <Kind extends PreviewKind, E extends { readonly message: string }, R>(
  kind: Kind,
  key: string,
  ref: number,
  fetch: Effect.Effect<PreviewValues[Kind], E, R>,
  stillLoading: (value: PreviewValues[Kind]) => boolean,
) =>
  Effect.gen(function* () {
    if (store.getState().cards.previews[kind][key]?.status === "loading") {
      return;
    }

    cardMutations.previewLoading(kind, key, ref);

    let delay = FIRST_RETRY;

    for (let attempt = 0; ; attempt += 1) {
      const value = yield* fetch.pipe(
        Effect.tapError((error) =>
          Effect.sync(() => cardMutations.previewFailed(kind, key, ref, error.message)),
        ),
      );

      cardMutations.previewLoaded(kind, key, ref, value, yield* Clock.currentTimeMillis);

      if (!stillLoading(value) || attempt >= STILL_LOADING_RETRIES) {
        return;
      }

      yield* Effect.sleep(delay);
      delay = Duration.times(delay, 2);
    }
  });

const githubStillLoading = (card: GithubPullRequestCard) => card.state === "loading";

const fizzyStillLoading = (preview: FizzyCardPreview) => preview.state === "loading";

/** The GitHub card under a message, or (`{ threadId }`) the discussion thread's header. */
export const loadGithub = Effect.fn("cards.loadGithub")(function* (
  roomId: number,
  pullRequestId: number,
  scope: api.GithubCardScope,
) {
  yield* loadPreview(
    "github",
    githubKey(roomId, pullRequestId, scope),
    pullRequestId,
    api.githubCard(roomId, pullRequestId, scope),
    githubStillLoading,
  );
});

/** The Fizzy card under a message (the fetch also asks the server to refresh it). */
export const loadFizzy = Effect.fn("cards.loadFizzy")(function* (
  roomId: number,
  fizzyCardId: number,
  messageId: number,
) {
  yield* loadPreview(
    "fizzy",
    fizzyKey(roomId, fizzyCardId, messageId),
    fizzyCardId,
    api.fizzyCard(roomId, fizzyCardId, messageId),
    fizzyStillLoading,
  );
});

/** A quoted message from another room, if the viewer may see it. */
export const loadQuote = Effect.fn("cards.loadQuote")(function* (
  roomId: number,
  referenceId: number,
) {
  yield* loadPreview(
    "quotes",
    quoteKey(roomId, referenceId),
    referenceId,
    api.quoteCard(roomId, referenceId),
    () => false,
  );
});

/** The viewer's response to an event and its counts. */
export const loadAttendance = Effect.fn("cards.loadAttendance")(function* (
  roomId: number,
  eventId: number,
) {
  yield* loadPreview(
    "attendance",
    attendanceKey(eventId),
    eventId,
    api.eventAttendance(roomId, eventId),
    () => false,
  );
});

/**
 * Answers an event (with `applyToFuture`, every later occurrence too). Shown at once; a refusal
 * puts the previous answer back, if nothing newer has replaced the guess since, and rejects.
 */
export const respond = Effect.fn("cards.respond")(function* (
  roomId: number,
  eventId: number,
  response: AttendanceResponse,
  applyToFuture: boolean,
) {
  const key = attendanceKey(eventId);

  yield* answers.run(
    eventId,
    Effect.gen(function* () {
      const before = store.getState().cards.previews.attendance[key];
      const shown = before?.value ?? null;
      const guess = shown === null ? null : withResponse(shown, response);

      if (guess !== null) {
        cardMutations.setAttendance(guess, yield* Clock.currentTimeMillis);
      }

      const reply = yield* api.respondToEvent(roomId, eventId, response, applyToFuture).pipe(
        Effect.onError(() =>
          Effect.sync(() => {
            const held = store.getState().cards.previews.attendance[key]?.value ?? null;

            if (shown !== null && guess !== null && held === guess) {
              cardMutations.setAttendance(shown, before?.fetchedAt ?? 0);
            }
          }),
        ),
      );

      cardMutations.setAttendance(reply, yield* Clock.currentTimeMillis);
    }),
  );
});
