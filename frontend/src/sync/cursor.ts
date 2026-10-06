import { Context, Effect, Layer, Option, Schema, SubscriptionRef } from "effect";
import { ResumePoint as ResumePointSchema } from "../api/schema/sync.ts";
import type { ResumePoint } from "../gen/ResumePoint.ts";

/** Where this tab's resume point is kept, so a reload resumes too. */
export const CURSOR_STORAGE_KEY = "smartfire.sync.cursor";

const decodeStored = Schema.decodeUnknownOption(Schema.fromJsonString(ResumePointSchema));

function load(): ResumePoint | null {
  try {
    const stored = sessionStorage.getItem(CURSOR_STORAGE_KEY);

    return stored === null ? null : Option.getOrNull(decodeStored(stored));
  } catch {
    // Storage can be off (privacy modes); the cursor then lives for this page only.
    return null;
  }
}

function save(point: ResumePoint): void {
  try {
    sessionStorage.setItem(CURSOR_STORAGE_KEY, JSON.stringify(point));
  } catch {
    // As in `load`: without storage the cursor is per page.
  }
}

/**
 * The resume point: the server's boot epoch and the last sequence applied here. Sent in `hello`
 * so the server can replay what was missed; persisted to sessionStorage (one per tab).
 */
export class Cursor extends Context.Service<
  Cursor,
  {
    readonly current: SubscriptionRef.SubscriptionRef<ResumePoint | null>;
    readonly get: Effect.Effect<ResumePoint | null>;
    /** Moves the cursor and persists it. */
    readonly set: (point: ResumePoint) => Effect.Effect<void>;
  }
>()("smartfire/sync/Cursor") {
  static readonly layer = Layer.effect(
    Cursor,
    Effect.gen(function* () {
      const current = yield* SubscriptionRef.make(load());

      return Cursor.of({
        current,
        get: SubscriptionRef.get(current),
        set: (point) =>
          SubscriptionRef.set(current, point).pipe(Effect.tap(Effect.sync(() => save(point)))),
      });
    }),
  );
}
