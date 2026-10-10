import { describe, expect, it } from "@effect/vitest";
import { Effect } from "effect";
import { search } from "./search-endpoints.ts";
import { FakeApi } from "./testing.ts";

const empty = {
  query: "launch",
  chips: [],
  messages: [],
  users: [],
  conversations: [],
  nextCursor: null,
  sections: [],
};

describe("typed search parameters", () => {
  it.effect("sends IDs, intersecting media filters and the selected sort with the cursor", () =>
    Effect.gen(function* () {
      const fake = yield* FakeApi;

      yield* fake.reply("GET /search", empty);
      yield* search("launch", "cursor", {
        authorId: 2,
        channelId: 7,
        has: ["mention", "image"],
        mentionsMe: true,
        sort: "oldest",
      });

      expect((yield* fake.requests).at(-1)?.query).toEqual({
        q: "launch",
        before: "cursor",
        authorId: "2",
        channelId: "7",
        has: "mention,image",
        mentionsMe: "true",
        sort: "oldest",
      });
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );
});
