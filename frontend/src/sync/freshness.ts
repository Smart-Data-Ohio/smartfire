import { Effect } from "effect";
import { mutations } from "../store/store.ts";

/**
 * Only list membership needs a client read history. Every request releases its ticket on
 * success, failure and interruption.
 */
export const withRead = <A, E, R>(
  request: (ticket: number) => Effect.Effect<A, E, R>,
  list: string,
) =>
  Effect.gen(function* () {
    const ticket = mutations.startRead(list);

    return yield* request(ticket).pipe(
      Effect.ensuring(Effect.sync(() => mutations.finishRead(ticket))),
    );
  });
