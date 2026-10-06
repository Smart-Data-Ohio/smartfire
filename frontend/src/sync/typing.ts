import { Clock, Context, Effect, Layer, Ref } from "effect";
import { SyncLink } from "./link.ts";

/** At most one `typing on` frame per room this often while composing. */
export const TYPING_THROTTLE_MS = 3000;

/**
 * This person's typing indicator: `set(roomId, true)` on each keystroke sends `typing on` at
 * most once per `TYPING_THROTTLE_MS`; `set(roomId, false)` (sent, blurred, emptied) sends
 * `typing off` at once if `on` went out.
 */
export class Typing extends Context.Service<
  Typing,
  { readonly set: (roomId: number, on: boolean) => Effect.Effect<void> }
>()("smartfire/sync/Typing") {
  static readonly layer = Layer.effect(
    Typing,
    Effect.gen(function* () {
      const link = yield* SyncLink;
      /** When `typing on` last went out, per room still marked typing. */
      const lastSent = yield* Ref.make<ReadonlyMap<number, number>>(new Map());

      const set = Effect.fnUntraced(function* (roomId: number, on: boolean) {
        const now = yield* Clock.currentTimeMillis;

        const shouldSend = yield* Ref.modify(
          lastSent,
          (sent): readonly [boolean, ReadonlyMap<number, number>] => {
            const last = sent.get(roomId);

            if (on) {
              return last === undefined || now - last >= TYPING_THROTTLE_MS
                ? [true, new Map(sent).set(roomId, now)]
                : [false, sent];
            }

            if (last === undefined) {
              return [false, sent];
            }

            const next = new Map(sent);

            next.delete(roomId);

            return [true, next];
          },
        );

        if (shouldSend) {
          yield* link.send({ t: "typing", conv: `room:${roomId}`, on });
        }
      });

      return Typing.of({ set });
    }),
  );
}
