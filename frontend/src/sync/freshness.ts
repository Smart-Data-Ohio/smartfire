import { Effect } from "effect";
import { mutations } from "../store/store.ts";

/** Every request releases its ticket on success, failure and interruption. */
export const withRead = <A, E, R>(
  request: (ticket: number) => Effect.Effect<A, E, R>,
  list: string | null = null,
) =>
  Effect.gen(function* () {
    const ticket = mutations.startRead(list);
    let rejected = false;

    const value = yield* request(ticket).pipe(
      Effect.ensuring(
        Effect.sync(() => {
          rejected = mutations.finishRead(ticket);
        }),
      ),
    );

    return { value, rejected };
  });

const replacing = new Set<string>();

/** The action owns retries; overlapping rejections coalesce into one replacement per key. */
export const readFresh = <E, R>(
  key: string,
  request: (ticket: number) => Effect.Effect<void, E, R>,
) =>
  Effect.gen(function* () {
    const first = yield* withRead(request, key);

    if (!first.rejected || replacing.has(key)) {
      return;
    }

    replacing.add(key);

    yield* Effect.gen(function* () {
      let rejected = true;

      while (rejected) {
        rejected = (yield* withRead(request, key)).rejected;
      }
    }).pipe(Effect.ensuring(Effect.sync(() => replacing.delete(key))));
  });
