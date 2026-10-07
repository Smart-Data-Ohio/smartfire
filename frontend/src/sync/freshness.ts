import { Effect } from "effect";
import { reloading } from "../store/freshness.ts";
import { mutations, store } from "../store/store.ts";

/**
 * Only list membership needs a client read history. Every request releases its ticket on
 * success, failure and interruption.
 */
export const withRead = <A, E, R>(
  request: (ticket: number) => Effect.Effect<A, E, R>,
  list: string,
  reload = true,
) =>
  Effect.gen(function* () {
    const ticket = mutations.startRead(list, reload);

    return yield* request(ticket).pipe(
      Effect.ensuring(Effect.sync(() => mutations.finishRead(ticket))),
    );
  });

/**
 * Waits until no first-page (re)load of `list` is on its way. A next page asked for meanwhile
 * would page from the cursor the reload is about to replace, so it starts after it instead.
 */
export const reloadsSettled = (list: string) =>
  Effect.callback<void>((resume) => {
    if (!reloading(store.getState().freshness, list)) {
      resume(Effect.void);

      return;
    }

    const unsubscribe = store.subscribe((state) => {
      if (!reloading(state.freshness, list)) {
        unsubscribe();
        resume(Effect.void);
      }
    });

    return Effect.sync(unsubscribe);
  });
