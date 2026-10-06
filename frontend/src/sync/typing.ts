import { Clock, Context, Effect, Layer, Ref } from "effect";
import { SyncLink } from "./link.ts";

/** At most one `typing on` frame per conversation this often while composing. */
export const TYPING_THROTTLE_MS = 3000;

/**
 * This person's typing indicator in a conversation (`room:<id>` or `thread:<id>`):
 * `set(conv, true)` on each keystroke sends `typing on` at most once per `TYPING_THROTTLE_MS`;
 * `set(conv, false)` (sent, blurred, emptied) sends `typing off` at once if `on` went out.
 */
export class Typing extends Context.Service<
  Typing,
  { readonly set: (conv: string, on: boolean) => Effect.Effect<void> }
>()("smartfire/sync/Typing") {
  static readonly layer = Layer.effect(
    Typing,
    Effect.gen(function* () {
      const link = yield* SyncLink;
      /** When `typing on` last went out, per conversation still marked typing. */
      const lastSent = yield* Ref.make<ReadonlyMap<string, number>>(new Map());

      const set = Effect.fnUntraced(function* (conv: string, on: boolean) {
        const now = yield* Clock.currentTimeMillis;

        const shouldSend = yield* Ref.modify(
          lastSent,
          (sent): readonly [boolean, ReadonlyMap<string, number>] => {
            const last = sent.get(conv);

            if (on) {
              return last === undefined || now - last >= TYPING_THROTTLE_MS
                ? [true, new Map(sent).set(conv, now)]
                : [false, sent];
            }

            if (last === undefined) {
              return [false, sent];
            }

            const next = new Map(sent);

            next.delete(conv);

            return [true, next];
          },
        );

        if (shouldSend) {
          yield* link.send({ t: "typing", conv, on });
        }
      });

      return Typing.of({ set });
    }),
  );
}
