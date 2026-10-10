import { afterEach, describe, expect, it } from "@effect/vitest";
import { Deferred, Effect, Fiber, Layer } from "effect";
import { TestClock } from "effect/testing";
import { Validation } from "../api/errors.ts";
import {
  FakeApi,
  meFixture,
  messageFixture,
  pageFixture,
  roomDetailFixture,
} from "../api/testing.ts";
import { mutations, store } from "../store/store.ts";
import { SyncLink } from "./link.ts";
import { Outbox } from "./outbox.ts";
import { Presence } from "./presence.ts";
import * as session from "./session.ts";
import { TestLifecycle } from "./testing.ts";
import { Topics } from "./topics.ts";
import { Typing } from "./typing.ts";

const ROOM = 12;

const OTHER = 13;

const note = (id: number) => messageFixture(id, ROOM);

const navigation = Layer.mergeAll(Outbox.layer, Topics.layer, Typing.layer, Presence.layer).pipe(
  Layer.provideMerge(Layer.mergeAll(SyncLink.layer, TestLifecycle.layer, FakeApi.layerClient)),
);

const holdFirstPage = Effect.gen(function* () {
  const api = yield* FakeApi;
  const started = yield* Deferred.make<void>();
  const release = yield* Deferred.make<void>();

  yield* api.reply(
    `GET /rooms/${ROOM}`,
    roomDetailFixture(ROOM, { firstUnreadMessageId: 5, count: 50 }),
  );
  yield* api.route(`GET /rooms/${ROOM}/messages`, (request) =>
    request.query?.around === "5"
      ? Deferred.succeed(started, undefined).pipe(
          Effect.andThen(Deferred.await(release)),
          Effect.as(pageFixture([note(4), note(5), note(6)], 3, 6)),
        )
      : Effect.succeed(pageFixture([note(9)], 8, null)),
  );

  return { started, release };
});

describe("sends during a room load", () => {
  afterEach(() => {
    mutations.reset();
    session.resetRoomVisits();
  });

  for (const opening of [
    { name: "permalink", focus: 5 },
    { name: "first unread", focus: null },
  ]) {
    it.effect(`an earlier failed send preserves the ${opening.name} on return`, () =>
      Effect.gen(function* () {
        const api = yield* FakeApi;

        mutations.setMe(meFixture);
        yield* api.reply(`GET /rooms/${ROOM}`, roomDetailFixture(ROOM));
        yield* api.reply(`GET /rooms/${ROOM}/messages`, pageFixture([note(9)], 8, null));
        yield* api.reply(`GET /rooms/${OTHER}`, roomDetailFixture(OTHER));
        yield* api.reply(`GET /rooms/${OTHER}/messages`, pageFixture([]));
        yield* api.route(`POST /rooms/${ROOM}/messages`, () =>
          Effect.fail(new Validation({ message: "Couldn't send", fields: {} })),
        );
        yield* session.openRoom(ROOM, null);

        const id = yield* session.send(ROOM, "old failed send");

        yield* TestClock.adjust("1 millis");
        expect(store.getState().pending[id]?.state).toBe("failed");

        yield* session.closeRoom(ROOM);
        yield* session.openRoom(OTHER, null);
        yield* session.closeRoom(OTHER);

        const { started, release } = yield* holdFirstPage;
        const openingRoom = yield* Effect.forkChild(session.openRoom(ROOM, opening.focus));

        yield* Deferred.await(started);
        yield* Deferred.succeed(release, undefined);
        yield* Fiber.join(openingRoom);

        expect(store.getState().timelines[ROOM]?.ids).toEqual([4, 5, 6]);
        expect(store.getState().timelines[ROOM]?.after).toBe(6);
        expect(store.getState().pending[id]?.state).toBe("failed");
        expect(
          (yield* api.requests)
            .filter(
              (request) => request.method === "GET" && request.path === `/rooms/${ROOM}/messages`,
            )
            .map((request) => request.query?.around ?? null),
        ).toEqual([null, "5"]);
      }).pipe(Effect.provide(navigation)),
    );
  }

  for (const outcome of ["pending", "confirmed", "failed"]) {
    it.effect(`a new ${outcome} send during the first page load opens at the present`, () =>
      Effect.gen(function* () {
        const api = yield* FakeApi;

        mutations.setMe(meFixture);
        yield* api.route(`POST /rooms/${ROOM}/messages`, () => {
          if (outcome === "pending") return Effect.never;

          if (outcome === "failed") {
            return Effect.fail(new Validation({ message: "Couldn't send", fields: {} }));
          }

          return Effect.succeed(messageFixture(9, ROOM, { clientMessageId: "new-send" }));
        });

        const { started, release } = yield* holdFirstPage;
        const openingRoom = yield* Effect.forkChild(session.openRoom(ROOM, null));

        yield* Deferred.await(started);
        yield* session.send(ROOM, "new send", { clientMessageId: "new-send" });
        yield* TestClock.adjust("1 millis");
        yield* Deferred.succeed(release, undefined);
        yield* Fiber.join(openingRoom);

        expect(store.getState().timelines[ROOM]?.ids).toEqual([9]);
        expect(store.getState().timelines[ROOM]?.after).toBeNull();
      }).pipe(Effect.provide(navigation)),
    );
  }

  it.effect("retrying an earlier failed send during the load opens at the present", () =>
    Effect.gen(function* () {
      const api = yield* FakeApi;
      const outbox = yield* Outbox;

      mutations.setMe(meFixture);
      yield* api.route(`POST /rooms/${ROOM}/messages`, () =>
        Effect.fail(new Validation({ message: "Couldn't send", fields: {} })),
      );

      const id = yield* session.send(ROOM, "old failed send");

      yield* TestClock.adjust("1 millis");
      expect(store.getState().pending[id]?.state).toBe("failed");

      const { started, release } = yield* holdFirstPage;
      const openingRoom = yield* Effect.forkChild(session.openRoom(ROOM, 5));

      yield* Deferred.await(started);
      yield* outbox.retry(id);
      yield* TestClock.adjust("1 millis");
      expect(store.getState().pending[id]?.state).toBe("failed");
      yield* Deferred.succeed(release, undefined);
      yield* Fiber.join(openingRoom);

      expect(store.getState().timelines[ROOM]?.ids).toEqual([9]);
      expect(store.getState().timelines[ROOM]?.after).toBeNull();
    }).pipe(Effect.provide(navigation)),
  );
});
