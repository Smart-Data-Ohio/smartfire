/**
 * Polls and cards in the store: the `asOf` ordering of a message's poll and cards, the viewer's
 * own ballots, votes on their way, and the per-viewer previews (GitHub pull requests, Fizzy
 * cards, quoted messages, event attendance). Pure functions over `State`; `card-mutations.ts`
 * wraps them in `setState`.
 *
 * Ordering (crates/api_types/src/cards.rs, **Ordering**): a message's `poll` and `cards` change
 * without its `updatedAt` moving, so each copy carries an `asOf` (`poll.asOf`, `cardsAsOf`). Per
 * message the store keeps the poll and the cards with the latest `asOf`, and on a tie the one
 * that arrived last; the rest of the message follows `updatedAt` as before. Ballots
 * (`PollBallot.asOf`) follow the same rule per poll.
 */
import type { AttendanceResponse } from "../gen/AttendanceResponse.ts";
import type { EventAttendance } from "../gen/EventAttendance.ts";
import type { FizzyCardPreview } from "../gen/FizzyCardPreview.ts";
import type { GithubPullRequestCard } from "../gen/GithubPullRequestCard.ts";
import type { MessageCard } from "../gen/MessageCard.ts";
import type { MessageCards } from "../gen/MessageCards.ts";
import type { Poll } from "../gen/Poll.ts";
import type { PollBallot } from "../gen/PollBallot.ts";
import type { PollResults } from "../gen/PollResults.ts";
import type { QuotePreviewResult } from "../gen/QuotePreviewResult.ts";
import { mergeMessageCopies } from "./agents.ts";
import type { LoadStatus, MessageDTO } from "./model.ts";
import type { State } from "./state.ts";

export type { MessageCard } from "../gen/MessageCard.ts";

export type { Poll } from "../gen/Poll.ts";

/** The viewer's own choice in a poll, as of when the server read it. */
export interface Ballot {
  readonly myOptionIds: readonly number[];
  readonly asOf: string;
}

/**
 * An event answer on its way: shown over the fetched attendance at once, dropped when its reply
 * lands or fails, unless a later answer has replaced it (compared by identity).
 */
export interface PendingAnswer {
  readonly response: AttendanceResponse;
}

/** What each preview table holds. */
export interface PreviewValues {
  readonly github: GithubPullRequestCard;
  readonly fizzy: FizzyCardPreview;
  readonly quotes: QuotePreviewResult;
  readonly attendance: EventAttendance;
}

export type PreviewKind = keyof PreviewValues;

/** One per-viewer fetch: the last good value stays while it reloads. */
export interface Preview<T> {
  readonly status: "loading" | "ready" | "error";
  readonly value: T | null;
  readonly error: string | null;
  /** The pull request, Fizzy card, reference or event id: what `message.cards` invalidates by. */
  readonly ref: number;
  /** When the value arrived (ms); 0 until one has. */
  readonly fetchedAt: number;
}

/** Previews by key (see `githubKey`, `fizzyKey`, `quoteKey`, `attendanceKey`). */
export type PreviewTables = {
  readonly [Kind in PreviewKind]: Readonly<Record<string, Preview<PreviewValues[Kind]>>>;
};

/** The cards slice of the store. */
export interface CardsState {
  /** The viewer's ballot per poll id, from results, vote replies and `poll.ballot`. */
  readonly ballots: Readonly<Record<number, Ballot>>;
  /** Ballots on their way per poll id: shown at once, dropped when the reply lands or fails. */
  readonly pendingVotes: Readonly<Record<number, readonly number[]>>;
  /** The per-viewer results fetch per poll id (anonymous polls need it for the viewer's vote). */
  readonly pollLoads: Readonly<Record<number, LoadStatus>>;
  /** Event answers on their way per event id (see `PendingAnswer`). */
  readonly pendingAnswers: Readonly<Record<number, PendingAnswer>>;
  readonly previews: PreviewTables;
}

export const emptyCards: CardsState = {
  ballots: {},
  pendingVotes: {},
  pollLoads: {},
  pendingAnswers: {},
  previews: { github: {}, fizzy: {}, quotes: {}, attendance: {} },
};

/** A ready preview older than this is fetched again when its card mounts. */
export const PREVIEW_TTL_MS = 5 * 60_000;

/** The GitHub card under a message, or the thread header's (which lists files). */
export function githubKey(
  roomId: number,
  pullRequestId: number,
  scope: { readonly messageId: number } | { readonly threadId: number },
): string {
  const where = "messageId" in scope ? `m${scope.messageId}` : `t${scope.threadId}`;

  return `${roomId}:${pullRequestId}:${where}`;
}

export function fizzyKey(roomId: number, fizzyCardId: number, messageId: number): string {
  return `${roomId}:${fizzyCardId}:m${messageId}`;
}

export function quoteKey(roomId: number, referenceId: number): string {
  return `${roomId}:${referenceId}`;
}

export function attendanceKey(eventId: number): string {
  return String(eventId);
}

// --- ordering ---------------------------------------------------------------------------------

/** The newer of two copies of a message's poll; on a tie the incoming (later) one. */
export function newerPoll(held: Poll | null, incoming: Poll | null): Poll | null {
  if (incoming === null) {
    // A poll never leaves its message; a copy without one is from before it was read.
    return held;
  }

  return held === null || incoming.asOf >= held.asOf ? incoming : held;
}

/**
 * The message to keep when `incoming` arrives and `held` is in the store: its fields from the
 * copy with the later `updatedAt` (`incomingWinsTie` says who keeps a tie, as each caller did
 * before), its agent steps merged from both, its poll and its cards each from the copy with the
 * later `asOf`. Returns `held` itself
 * when nothing changes, so selectors don't re-render.
 */
export function reconcileMessage(
  held: MessageDTO | undefined,
  incoming: MessageDTO,
  incomingWinsTie: boolean,
): MessageDTO {
  if (held === undefined) {
    return incoming;
  }

  // The newer copy's fields, with the agent steps of both merged step by step.
  const base = mergeMessageCopies(held, incoming, incomingWinsTie ? "incoming" : "held");
  const poll = newerPoll(held.poll, incoming.poll);
  const cardsFrom = incoming.cardsAsOf >= held.cardsAsOf ? incoming : held;

  if (base.poll === poll && base.cards === cardsFrom.cards) {
    return base;
  }

  return { ...base, poll, cards: cardsFrom.cards, cardsAsOf: cardsFrom.cardsAsOf };
}

function withMessage(state: State, message: MessageDTO): State {
  return { ...state, messages: { ...state.messages, [message.id]: message } };
}

function withCards(state: State, cards: CardsState): State {
  return { ...state, cards };
}

/** A poll copy (`poll.updated`, a results or vote reply) lands on its message unless older. */
export function applyPoll(state: State, poll: Poll): State {
  const held = state.messages[poll.messageId];

  if (held === undefined || newerPoll(held.poll, poll) !== poll) {
    return state;
  }

  return withMessage(state, { ...held, poll });
}

/** The viewer's ballot for a poll lands unless the held one was read later. */
export function applyBallot(
  state: State,
  pollId: number,
  myOptionIds: readonly number[],
  asOf: string,
): State {
  const held = state.cards.ballots[pollId];

  if (held !== undefined && held.asOf > asOf) {
    return state;
  }

  return withCards(state, {
    ...state.cards,
    ballots: { ...state.cards.ballots, [pollId]: { myOptionIds, asOf } },
  });
}

/** The `poll.ballot` event: the viewer voted in some tab. */
export function applyPollBallot(state: State, ballot: PollBallot): State {
  return applyBallot(state, ballot.pollId, ballot.myOptionIds, ballot.asOf);
}

/** A results or vote reply: the poll, and the viewer's ballot as of the same read. */
export function applyPollResults(state: State, results: PollResults): State {
  const withBallot = applyBallot(
    applyPoll(state, results.poll),
    results.poll.id,
    results.myOptionIds,
    results.poll.asOf,
  );

  return setPollLoad(withBallot, results.poll.id, "ready");
}

export function setPollLoad(state: State, pollId: number, status: LoadStatus): State {
  if (state.cards.pollLoads[pollId] === status) {
    return state;
  }

  return withCards(state, {
    ...state.cards,
    pollLoads: { ...state.cards.pollLoads, [pollId]: status },
  });
}

/** A vote on its way (`optionIds`), or none (`null`) once its reply lands or fails. */
export function setPendingVote(
  state: State,
  pollId: number,
  optionIds: readonly number[] | null,
): State {
  const { [pollId]: _previous, ...others } = state.cards.pendingVotes;

  return withCards(state, {
    ...state.cards,
    pendingVotes: optionIds === null ? others : { ...others, [pollId]: optionIds },
  });
}

/**
 * A vote's reply landed: the poll and ballot it carries, and its pending ballot dropped unless a
 * later vote replaced it meanwhile (that one's reply settles it).
 */
export function settleVote(
  state: State,
  pollId: number,
  sent: readonly number[],
  results: PollResults | null,
): State {
  const landed = results === null ? state : applyPollResults(state, results);

  return landed.cards.pendingVotes[pollId] === sent ? setPendingVote(landed, pollId, null) : landed;
}

/** The ids each preview table is invalidated by. */
interface PreviewRefs {
  readonly github: Set<number>;
  readonly fizzy: Set<number>;
  readonly quotes: Set<number>;
  readonly attendance: Set<number>;
}

/** The refs among `cards`. */
function previewRefs(cards: readonly MessageCard[]): PreviewRefs {
  const refs: PreviewRefs = {
    github: new Set<number>(),
    fizzy: new Set<number>(),
    quotes: new Set<number>(),
    attendance: new Set<number>(),
  };

  for (const card of cards) {
    switch (card.kind) {
      case "github":
        refs.github.add(card.data.pullRequestId);
        break;
      case "fizzy":
        refs.fizzy.add(card.data.fizzyCardId);
        break;
      case "quote":
        refs.quotes.add(card.data.referenceId);
        break;
      case "event":
        refs.attendance.add(card.data.eventId);
        break;
      default:
        break;
    }
  }

  return refs;
}

function withoutRefs<T>(
  table: Readonly<Record<string, Preview<T>>>,
  refs: ReadonlySet<number>,
): Readonly<Record<string, Preview<T>>> {
  if (refs.size === 0) {
    return table;
  }

  const kept = Object.entries(table).filter(([, preview]) => !refs.has(preview.ref));

  return kept.length === Object.keys(table).length ? table : Object.fromEntries(kept);
}

/**
 * The `message.cards` event: the message's cards are replaced unless the held ones are newer.
 * The previews its old and new cards point at are dropped, so mounted cards fetch them again (a
 * fetch finished, a pull request was refreshed, an event changed).
 */
export function applyMessageCards(state: State, change: MessageCards): State {
  const held = state.messages[change.messageId];

  if (held === undefined || held.cardsAsOf > change.asOf) {
    return state;
  }

  const refs = previewRefs([...held.cards, ...change.cards]);
  const { previews } = state.cards;

  return withCards(withMessage(state, { ...held, cards: change.cards, cardsAsOf: change.asOf }), {
    ...state.cards,
    previews: {
      github: withoutRefs(previews.github, refs.github),
      fizzy: withoutRefs(previews.fizzy, refs.fizzy),
      quotes: withoutRefs(previews.quotes, refs.quotes),
      attendance: withoutRefs(previews.attendance, refs.attendance),
    },
  });
}

// --- previews ---------------------------------------------------------------------------------

/** Replaces one preview. */
export function setPreview<Kind extends PreviewKind>(
  state: State,
  kind: Kind,
  key: string,
  preview: Preview<PreviewValues[Kind]>,
): State {
  const { previews } = state.cards;

  return withCards(state, {
    ...state.cards,
    previews: { ...previews, [kind]: { ...previews[kind], [key]: preview } },
  });
}

/** A fetch started: the last good value stays on screen. */
export function previewLoading<Kind extends PreviewKind>(
  state: State,
  kind: Kind,
  key: string,
  ref: number,
): State {
  const held: Preview<PreviewValues[Kind]> | undefined = state.cards.previews[kind][key];

  return setPreview(state, kind, key, {
    status: "loading",
    value: held?.value ?? null,
    error: null,
    ref,
    fetchedAt: held?.fetchedAt ?? 0,
  });
}

/** A fetch answered. */
export function previewLoaded<Kind extends PreviewKind>(
  state: State,
  kind: Kind,
  key: string,
  ref: number,
  value: PreviewValues[Kind],
  now: number,
): State {
  return setPreview(state, kind, key, { status: "ready", value, error: null, ref, fetchedAt: now });
}

/** A fetch failed; a value already shown stays. */
export function previewFailed<Kind extends PreviewKind>(
  state: State,
  kind: Kind,
  key: string,
  ref: number,
  error: string,
): State {
  const held: Preview<PreviewValues[Kind]> | undefined = state.cards.previews[kind][key];

  return setPreview(state, kind, key, {
    status: "error",
    value: held?.value ?? null,
    error,
    ref,
    fetchedAt: held?.fetchedAt ?? 0,
  });
}

/** Whether a card should fetch its preview now: never fetched, or stale and not on its way. */
export function needsFetch<T>(preview: Preview<T> | undefined, now: number): boolean {
  if (preview === undefined) {
    return true;
  }

  return preview.status === "ready" && now - preview.fetchedAt > PREVIEW_TTL_MS;
}

const COUNT_FIELD = {
  going: "goingCount",
  maybe: "maybeCount",
  declined: "declinedCount",
} as const;

/** The attendance with the viewer's response changed, counts and all (shown before the reply). */
export function withResponse(
  attendance: EventAttendance,
  response: AttendanceResponse,
): EventAttendance {
  if (attendance.response === response) {
    return attendance;
  }

  const next = {
    ...attendance,
    response,
    [COUNT_FIELD[response]]: attendance[COUNT_FIELD[response]] + 1,
  };

  if (attendance.response === null) {
    return next;
  }

  const previous = COUNT_FIELD[attendance.response];

  return { ...next, [previous]: Math.max(0, attendance[previous] - 1) };
}

/** An answer on its way (`answer`), or none (`null`). */
export function setPendingAnswer(
  state: State,
  eventId: number,
  answer: PendingAnswer | null,
): State {
  const { [eventId]: _previous, ...others } = state.cards.pendingAnswers;

  return withCards(state, {
    ...state.cards,
    pendingAnswers: answer === null ? others : { ...others, [eventId]: answer },
  });
}

/**
 * An answer's reply landed (`reply`, fetched at `now`) or it was refused (`null`): the reply
 * becomes the attendance, and the pending answer is dropped only if it is still `answer` (a later
 * one stays shown until its own reply).
 */
export function settleAnswer(
  state: State,
  eventId: number,
  answer: PendingAnswer,
  reply: EventAttendance | null,
  now: number,
): State {
  const landed =
    reply === null
      ? state
      : previewLoaded(state, "attendance", attendanceKey(eventId), eventId, reply, now);

  return landed.cards.pendingAnswers[eventId] === answer
    ? setPendingAnswer(landed, eventId, null)
    : landed;
}

/** The attendance as shown: the fetched one with the answer on its way, if any, applied. */
export function shownAttendance(
  attendance: EventAttendance | null,
  pending: PendingAnswer | undefined,
): EventAttendance | null {
  return attendance === null || pending === undefined
    ? attendance
    : withResponse(attendance, pending.response);
}

// --- reading polls ----------------------------------------------------------------------------

/** Whether a poll takes no more votes: closed by the server, or past `closesAt` here. */
export function pollClosed(poll: Poll, now: number): boolean {
  return (
    poll.closed ||
    poll.closedAt !== null ||
    (poll.closesAt !== null && Date.parse(poll.closesAt) <= now)
  );
}

/**
 * The viewer's choice as the server last told it: the ballot when it's as new as the poll (or
 * the poll is anonymous), otherwise the options naming the viewer among their voters. `null`
 * while unknown: an anonymous poll whose results haven't been fetched.
 */
export function viewerChoice(
  poll: Poll,
  ballot: Ballot | undefined,
  viewerId: number,
): readonly number[] | null {
  if (ballot !== undefined && (poll.anonymous || ballot.asOf >= poll.asOf)) {
    return ballot.myOptionIds;
  }

  if (!poll.anonymous) {
    return poll.options.flatMap((option) =>
      option.voterIds.includes(viewerId) ? [option.id] : [],
    );
  }

  return ballot?.myOptionIds ?? null;
}

/** What a poll card shows: the counts with the viewer's pending ballot already in them. */
export interface PollView {
  readonly poll: Poll;
  /** The viewer's choice (pending, else the server's); `null` while unknown. */
  readonly myOptionIds: readonly number[] | null;
  /** A vote is on its way. */
  readonly pending: boolean;
  readonly closed: boolean;
}

/**
 * The poll as the card shows it: `pending` (the viewer's ballot on its way) replaces their server
 * choice in the counts and voters, so a vote moves the bars at once and a refusal (which drops
 * `pending`) puts them back without touching anything that arrived meanwhile.
 */
export function pollView(
  poll: Poll,
  ballot: Ballot | undefined,
  pending: readonly number[] | undefined,
  viewerId: number,
  now: number,
): PollView {
  const server = viewerChoice(poll, ballot, viewerId);
  const closed = pollClosed(poll, now);

  if (pending === undefined) {
    return { poll, myOptionIds: server, pending: false, closed };
  }

  const before = new Set(server ?? []);
  const after = new Set(pending);

  const options = poll.options.map((option) => {
    const delta = Number(after.has(option.id)) - Number(before.has(option.id));

    if (delta === 0) {
      return option;
    }

    const voterIds = poll.anonymous
      ? option.voterIds
      : delta > 0
        ? [...option.voterIds.filter((id) => id !== viewerId), viewerId]
        : option.voterIds.filter((id) => id !== viewerId);

    return { ...option, votes: Math.max(0, option.votes + delta), voterIds };
  });

  const totalVotes = Math.max(0, poll.totalVotes + after.size - before.size);

  return { poll: { ...poll, options, totalVotes }, myOptionIds: pending, pending: true, closed };
}

// --- reading cards ----------------------------------------------------------------------------

const KNOWN_KINDS: ReadonlySet<string> = new Set([
  "drive",
  "github",
  "x",
  "event",
  "fizzy",
  "quote",
  "linkedin",
  "link",
]);

/**
 * The cards to render under a message, in the server's slot order: kinds this build knows (the
 * store keeps the wire JSON, so a newer kind is still there and is skipped here, as the
 * tolerant schema's `unknown` card is), without LinkedIn and link previews while the author
 * suppressed embeds (the server leaves them out too).
 */
export function visibleCards(message: MessageDTO): readonly MessageCard[] {
  return message.cards.filter(
    (card) =>
      KNOWN_KINDS.has(card.kind) &&
      !(message.embedsSuppressed && (card.kind === "linkedin" || card.kind === "link")),
  );
}
