import { afterEach, describe, expect, it } from "@effect/vitest";
import { Effect } from "effect";
import { Forbidden } from "../api/errors.ts";
import {
  FakeApi,
  meFixture,
  messageFixture,
  pageFixture,
  roomDetailFixture,
} from "../api/testing.ts";
import { mutations, store } from "../store/store.ts";
import * as messages from "./message-actions.ts";

const ROOM = 12;

const reaction = { content: "🎉", title: "Party popper", imageUrl: null, reactorIds: [3] };

function seed(): void {
  mutations.reset();
  mutations.setMe(meFixture);
  mutations.setRoomDetail({ ...roomDetailFixture(ROOM), pinsCount: 2 });
  mutations.applyPage(
    ROOM,
    {
      ...pageFixture([messageFixture(1, ROOM, { reactions: [reaction] })]),
      saved: [{ messageId: 1, savedItemId: 31 }],
    },
    "replace",
  );
}

const refuse = () => Effect.fail(new Forbidden({ message: "Not allowed" }));

describe("message actions", () => {
  afterEach(() => mutations.reset());

  it.effect("show a reaction at once and roll it back when the server refuses", () =>
    Effect.gen(function* () {
      seed();

      const fake = yield* FakeApi;
      let seen: readonly number[] = [];

      yield* fake.route("POST /messages/1/boosts", () => {
        seen = store.getState().messages[1]?.reactions[0]?.reactorIds ?? [];

        return refuse();
      });

      const failure = yield* Effect.flip(
        messages.toggleReaction(1, "🎉", { title: "Party popper", imageUrl: null }),
      );

      expect(failure.message).toBe("Not allowed");
      expect(seen).toEqual([3, meFixture.user.id]);
      expect(store.getState().messages[1]?.reactions).toEqual([reaction]);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("undo only its own toggle when refused, keeping reactions that arrived meanwhile", () =>
    Effect.gen(function* () {
      seed();

      const fake = yield* FakeApi;
      const thumbs = { content: "👍", title: "Thumbs up", imageUrl: null, reactorIds: [5] };

      yield* fake.route("POST /messages/1/boosts", () => {
        const held = store.getState().messages[1];

        // Someone else's reaction lands while the request is out.
        if (held !== undefined) {
          mutations.setReactions({
            messageId: 1,
            roomId: ROOM,
            threadId: null,
            reactions: [...held.reactions, thumbs],
            boosts: held.boosts,
            updatedAt: held.updatedAt,
          });
        }

        return refuse();
      });

      yield* Effect.flip(
        messages.toggleReaction(1, "🎉", { title: "Party popper", imageUrl: null }),
      );

      expect(store.getState().messages[1]?.reactions).toEqual([reaction, thumbs]);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("flip a pin at once and put it back when the server refuses", () =>
    Effect.gen(function* () {
      seed();

      const fake = yield* FakeApi;
      let during: boolean | undefined;

      yield* fake.route("POST /messages/1/pin", () => {
        during = store.getState().messages[1]?.pinned;

        return refuse();
      });

      yield* Effect.flip(messages.setPinned(1, true));

      expect(during).toBe(true);
      expect(store.getState().messages[1]?.pinned).toBe(false);
      expect(store.getState().rooms[ROOM]?.detail?.pinsCount).toBe(2);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("unsave at once, and save with the server's item id", () =>
    Effect.gen(function* () {
      seed();

      const fake = yield* FakeApi;

      yield* fake.route("DELETE /saved/31", () => Effect.succeed(null));
      yield* fake.reply("POST /saved", {
        id: 32,
        messageId: 1,
        status: "in_progress",
        remindAt: null,
        remindedAt: null,
        createdAt: "2026-10-06T11:00:00.000Z",
      });

      yield* messages.unsave(1);
      expect(store.getState().saved).toEqual({});

      yield* messages.save(1);
      expect(store.getState().saved).toEqual({ 1: 32 });
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );
});
