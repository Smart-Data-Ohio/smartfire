/**
 * The writes the card actions make, each one `setState` (so one React commit), over the pure
 * reducers in `cards.ts`. Apart from `store.ts`'s `mutations` so the cards slice stays in its
 * own files.
 */
import type { EventAttendance } from "../gen/EventAttendance.ts";
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
  ) => apply((state) => cards.previewLoaded(state, kind, key, ref, value, now)),
  previewFailed: (kind: cards.PreviewKind, key: string, ref: number, error: string) =>
    apply((state) => cards.previewFailed(state, kind, key, ref, error)),
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
