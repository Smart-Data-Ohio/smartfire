/**
 * The writes the card actions make, each one `setState` (so one React commit), over the pure
 * reducers in `cards.ts`. Apart from `store.ts`'s `mutations` so the cards slice stays in its
 * own files.
 */
import type { EventAttendance } from "../gen/EventAttendance.ts";
import type { GithubPullRequestActions } from "../gen/GithubPullRequestActions.ts";
import type { PollResults } from "../gen/PollResults.ts";
import * as cards from "./cards.ts";
import type { LoadStatus } from "./model.ts";
import type { State } from "./state.ts";
import { store } from "./store.ts";

const apply = (change: (state: State) => State) => store.setState(change, true);

/** Every write to the cards slice (and to the polls on messages). */
export const cardMutations = {
  applyPollResults: (results: PollResults) =>
    apply((state) => cards.applyPollResults(state, results)),
  setPollLoad: (pollId: number, status: LoadStatus) =>
    apply((state) => cards.setPollLoad(state, pollId, status)),
  setPendingVote: (pollId: number, optionIds: readonly number[]) =>
    apply((state) => cards.setPendingVote(state, pollId, optionIds)),
  /** The vote `sent` was answered (`results`) or refused (`null`). */
  settleVote: (pollId: number, sent: readonly number[], results: PollResults | null) =>
    apply((state) => cards.settleVote(state, pollId, sent, results)),
  previewLoading: (kind: cards.PreviewKind, key: string, ref: number) =>
    apply((state) => cards.previewLoading(state, kind, key, ref)),
  previewLoaded: <Kind extends cards.PreviewKind>(
    kind: Kind,
    key: string,
    ref: number,
    value: cards.PreviewValues[Kind],
    now: number,
    generation: number,
  ) => apply((state) => cards.previewLoaded(state, kind, key, ref, value, now, generation)),
  previewFailed: (
    kind: cards.PreviewKind,
    key: string,
    ref: number,
    error: string,
    generation: number,
  ) => apply((state) => cards.previewFailed(state, kind, key, ref, error, generation)),
  /**
   * Starts a shared `/actions` read. `null` when one for this `reason` is already in flight:
   * the caller joins it instead of fetching again.
   */
  beginGithubActions: (key: string, reason: string): number | null => {
    const held = store.getState().cards.githubActions[key];

    if (held?.status === "loading" && held.reason === reason) {
      return null;
    }

    const token = (held?.token ?? 0) + 1;

    apply((state) => cards.beginGithubActions(state, key, reason, token));

    return token;
  },
  settleGithubActions: (key: string, token: number, value: GithubPullRequestActions | null) =>
    apply((state) => cards.settleGithubActions(state, key, token, value)),
  /** A discussion started: the card's preview loads again, and `/actions` follows that load. */
  invalidateGithub: (pullRequestId: number) =>
    apply((state) => cards.invalidateGithub(state, pullRequestId)),
  /** Shows an answer over the fetched attendance until its reply. */
  setPendingAnswer: (eventId: number, answer: cards.PendingAnswer) =>
    apply((state) => cards.setPendingAnswer(state, eventId, answer)),
  /** The answer was replied to (`reply`) or refused (`null`). */
  settleAnswer: (
    eventId: number,
    answer: cards.PendingAnswer,
    reply: EventAttendance | null,
    now: number,
  ) => apply((state) => cards.settleAnswer(state, eventId, answer, reply, now)),
};
