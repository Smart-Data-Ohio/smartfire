import { describe, expect, it } from "@effect/vitest";
import { Effect } from "effect";
import { FakeApi } from "./testing.ts";
import {
  addThreadAccessMember,
  removeThreadAccessMember,
  threadAccessMembers,
} from "./thread-endpoints.ts";

const members = {
  members: [{ userId: 8, addedById: null, createdAt: "2026-10-12T10:00:00.000Z" }],
  users: [],
};

describe("private-thread member endpoints", () => {
  it.effect("lists and adds access grants with the pinned decoder", () =>
    Effect.gen(function* () {
      const fake = yield* FakeApi;
      yield* fake.reply("GET /threads/88/members", members);
      yield* fake.reply("POST /threads/88/members", members);
      expect(yield* threadAccessMembers(88)).toEqual(members);
      expect(yield* addThreadAccessMember(88, { userId: 8 })).toEqual(members);
      expect(yield* fake.requests).toEqual([
        { method: "GET", path: "/threads/88/members" },
        { method: "POST", path: "/threads/88/members", body: { userId: 8 } },
      ]);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("removes a grant with DELETE and accepts an empty response", () =>
    Effect.gen(function* () {
      const fake = yield* FakeApi;
      yield* fake.reply("DELETE /threads/88/members/8", null);
      expect(yield* removeThreadAccessMember(88, 8)).toBeUndefined();
      expect((yield* fake.requests).at(-1)).toEqual({
        method: "DELETE",
        path: "/threads/88/members/8",
      });
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );
});
