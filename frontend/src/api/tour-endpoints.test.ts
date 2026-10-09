import { describe, expect, it } from "@effect/vitest";
import { Effect } from "effect";
import { FakeApi } from "./testing.ts";
import { completeTour, TOUR_PATH } from "./tour-endpoints.ts";

describe("completeTour", () => {
  it.effect("patches classic's tour stamp from the origin's root, through the app's client", () =>
    Effect.gen(function* () {
      const fake = yield* FakeApi;

      yield* fake.route(`PATCH ${TOUR_PATH}`, () => Effect.succeed(null));
      yield* completeTour();

      expect(TOUR_PATH).toBe("/users/me/tour");
      expect(yield* fake.requests).toEqual([
        { method: "PATCH", path: "/users/me/tour", root: true },
      ]);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );
});
