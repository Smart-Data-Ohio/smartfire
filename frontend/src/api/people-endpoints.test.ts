import { describe, expect, it } from "@effect/vitest";
import { Effect } from "effect";
import { blocks, personProfile, setIgnored } from "./people-endpoints.ts";
import { FakeApi, userFixture } from "./testing.ts";

describe("personal ignore endpoints", () => {
  it.effect("reads only the viewer's ignore list", () =>
    Effect.gen(function* () {
      const fake = yield* FakeApi;
      yield* fake.reply("GET /blocks", { viewerId: 7, people: [], revision: 0 });
      expect(yield* blocks()).toEqual({ viewerId: 7, people: [], revision: 0 });
      expect((yield* fake.requests).at(-1)?.path).toBe("/blocks");
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("uses PUT and DELETE for idempotent ignore changes", () =>
    Effect.gen(function* () {
      const fake = yield* FakeApi;

      for (const ignoredByMe of [true, false]) {
        const method = ignoredByMe ? "PUT" : "DELETE";
        yield* fake.reply(`${method} /people/2/block`, {
          viewerId: 7,
          userId: 2,
          ignoredByMe,
          revision: 1,
        });
        expect(yield* setIgnored(2, ignoredByMe)).toEqual({
          viewerId: 7,
          userId: 2,
          ignoredByMe,
          revision: 1,
        });
        expect((yield* fake.requests).at(-1)?.method).toBe(method);
      }
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );
});

describe("a person's profile", () => {
  it.effect("asks for the user card's read with card=1, and the page's without it", () =>
    Effect.gen(function* () {
      const fake = yield* FakeApi;

      const profile = {
        user: userFixture(2, "Ada"),
        status: null,
        dndAllowed: null,
        emailAddress: null,
        transferUrl: null,
        transferQrSvg: null,
        canBan: false,
        ignoredByMe: false,
        canManageBot: false,
      };

      yield* fake.reply("GET /people/2", profile);
      yield* personProfile(2, true);
      expect((yield* fake.requests).at(-1)).toMatchObject({
        method: "GET",
        path: "/people/2",
        query: { card: "1" },
      });

      yield* fake.reply("GET /people/2", profile);
      yield* personProfile(2);
      expect((yield* fake.requests).at(-1)?.query).toBeUndefined();
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );
});
