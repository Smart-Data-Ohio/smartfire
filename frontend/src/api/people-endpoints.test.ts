import { describe, expect, it } from "@effect/vitest";
import { Effect } from "effect";
import { blocks, setIgnored } from "./people-endpoints.ts";
import { FakeApi } from "./testing.ts";

describe("personal ignore endpoints", () => {
  it.effect("reads only the viewer's ignore list", () =>
    Effect.gen(function* () {
      const fake = yield* FakeApi;
      yield* fake.reply("GET /blocks", { people: [], revision: 0 });
      expect(yield* blocks()).toEqual({ people: [], revision: 0 });
      expect((yield* fake.requests).at(-1)?.path).toBe("/blocks");
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("uses PUT and DELETE for idempotent ignore changes", () =>
    Effect.gen(function* () {
      const fake = yield* FakeApi;

      for (const ignoredByMe of [true, false]) {
        const method = ignoredByMe ? "PUT" : "DELETE";
        yield* fake.reply(`${method} /people/2/block`, { userId: 2, ignoredByMe, revision: 1 });
        expect(yield* setIgnored(2, ignoredByMe)).toEqual({ userId: 2, ignoredByMe, revision: 1 });
        expect((yield* fake.requests).at(-1)?.method).toBe(method);
      }
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );
});
