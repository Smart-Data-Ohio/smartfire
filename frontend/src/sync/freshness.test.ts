import { afterEach, describe, expect, it } from "@effect/vitest";
import { Deferred, Effect, Fiber } from "effect";
import {
  factsFixture,
  threadDetailFixture,
  workDetailFixture,
} from "../features/work/test-fixtures.ts";
import { membership } from "../store/freshness.ts";
import { mutations, store } from "../store/store.ts";
import { readFresh, withRead } from "./freshness.ts";

const detail = (status: "blocked" | "done") =>
  threadDetailFixture(7, factsFixture({ status }), workDetailFixture());

describe("fresh read lifecycle", () => {
  afterEach(() => mutations.reset());

  it.effect("coalesces overlapping rejected reads into one replacement per key", () =>
    Effect.gen(function* () {
      mutations.loadThreadDetail(detail("blocked"));

      const started = yield* Deferred.make<void>();
      const gate = yield* Deferred.make<void>();
      const replacementStarted = yield* Deferred.make<void>();
      const replacementGate = yield* Deferred.make<void>();
      let calls = 0;

      const read = readFresh("work:7", (ticket) =>
        Effect.gen(function* () {
          calls += 1;

          if (calls <= 2) {
            if (calls === 2) {
              yield* Deferred.succeed(started, undefined);
            }

            yield* Deferred.await(gate);
            mutations.loadThreadDetail(detail("blocked"), ticket);
          } else {
            yield* Deferred.succeed(replacementStarted, undefined);
            yield* Deferred.await(replacementGate);
            mutations.loadThreadDetail(detail("done"), ticket);
          }
        }),
      );

      const first = yield* Effect.forkChild(read);
      const second = yield* Effect.forkChild(read);

      yield* Deferred.await(started);
      mutations.putWorkFacts(7, factsFixture({ status: "done" }));
      yield* Deferred.succeed(gate, undefined);
      yield* Deferred.await(replacementStarted);
      yield* Effect.yieldNow;

      expect(calls).toBe(3);
      expect(store.getState().threads[7]?.work?.status).toBe("done");

      yield* Deferred.succeed(replacementGate, undefined);
      yield* Fiber.join(first);
      yield* Fiber.join(second);

      expect(calls).toBe(3);
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
      store.setState((state) => ({
        ...state,
        freshness: membership(state.freshness, "approved", [], [3]),
      }));

      expect(store.getState().freshness.deltas).toHaveLength(1);

      yield* Fiber.interrupt(loading);

      expect(store.getState().freshness.reads).toEqual({});
      expect(store.getState().freshness.deltas).toEqual([]);
    }),
  );
});
