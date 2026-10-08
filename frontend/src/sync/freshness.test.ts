import { afterEach, describe, expect, it } from "@effect/vitest";
import { Deferred, Effect, Fiber } from "effect";
import { membership } from "../store/freshness.ts";
import { mutations, store } from "../store/store.ts";
import { withRead } from "./freshness.ts";

describe("list read lifecycle", () => {
  afterEach(() => mutations.reset());

  it.effect("releases successful and failed reads", () =>
    Effect.gen(function* () {
      const value = yield* withRead(() => Effect.succeed(42), "approved");

      expect(value).toBe(42);
      expect(store.getState().freshness.reads).toEqual({});

      yield* Effect.flip(withRead(() => Effect.fail("offline"), "approved"));

      expect(store.getState().freshness.reads).toEqual({});
    }),
  );

  it.effect("releases interrupted reads and their membership history", () =>
    Effect.gen(function* () {
      const started = yield* Deferred.make<void>();

      const loading = yield* Effect.forkChild(
        withRead(
          () => Deferred.succeed(started, undefined).pipe(Effect.andThen(Effect.never)),
          "approved",
        ),
      );

      yield* Deferred.await(started);
      store.setState(
        (state) => ({ ...state, freshness: membership(state.freshness, "approved", [], [3]) }),
        true,
      );

      expect(store.getState().freshness.deltas).toHaveLength(1);

      yield* Fiber.interrupt(loading);

      expect(store.getState().freshness.reads).toEqual({});
      expect(store.getState().freshness.deltas).toEqual([]);
    }),
  );
});
