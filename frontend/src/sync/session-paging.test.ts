import { afterEach, describe, expect, it } from "@effect/vitest";
import { Deferred, Effect, Fiber } from "effect";
import { ServerError } from "../api/errors.ts";
import { FakeApi, messageFixture, pageFixture, roomDetailFixture } from "../api/testing.ts";
import { mutations, store } from "../store/store.ts";
import * as session from "./session.ts";

const ROOM = 12;

const note = (id: number) => messageFixture(id, ROOM);

const timeline = () => store.getState().timelines[ROOM];

describe("room timeline paging", () => {
  afterEach(() => mutations.reset());

  it.effect("a failure in one direction leaves the other request running", () =>
    Effect.gen(function* () {
      const fake = yield* FakeApi;
      const olderStarted = yield* Deferred.make<void>();
      const newerStarted = yield* Deferred.make<void>();
      const releaseNewer = yield* Deferred.make<void>();

      mutations.applyPage(ROOM, pageFixture([note(5)], 4, 5), "replace");

      yield* fake.route(`GET /rooms/${ROOM}/messages`, (request) => {
        if (request.query?.before !== undefined) {
          return Deferred.succeed(olderStarted, undefined).pipe(
            Effect.andThen(Effect.fail(new ServerError({ status: 500, message: "nope" }))),
          );
        }

        return Deferred.succeed(newerStarted, undefined).pipe(
          Effect.andThen(Deferred.await(releaseNewer)),
          Effect.as(pageFixture([note(6)], 5, null)),
        );
      });

      const older = yield* Effect.forkChild(session.loadOlder(ROOM));
      const newer = yield* Effect.forkChild(session.loadNewer(ROOM));

      yield* Deferred.await(olderStarted);
      yield* Deferred.await(newerStarted);
      yield* Fiber.join(older);

      expect(timeline()?.loadingOlder).toBe(false);
      expect(timeline()?.loadingNewer).toBe(true);
      expect(timeline()?.ids).toEqual([5]);

      yield* Deferred.succeed(releaseNewer, undefined);
      yield* Fiber.join(newer);

      expect(timeline()?.loadingNewer).toBe(false);
      expect(timeline()?.ids).toEqual([5, 6]);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("a page started before leaving does not clear the one issued on return", () =>
    Effect.gen(function* () {
      const fake = yield* FakeApi;
      const started = yield* Deferred.make<void>();
      const release = yield* Deferred.make<void>();
      const returned = yield* Deferred.make<void>();
      const releaseReturn = yield* Deferred.make<void>();

      mutations.applyPage(ROOM, pageFixture([note(5)], 4, null), "replace");
      yield* fake.reply(`GET /rooms/${ROOM}`, roomDetailFixture(ROOM));
      yield* fake.route(`GET /rooms/${ROOM}/messages`, (request) => {
        if (request.query?.before === "4") {
          return Deferred.succeed(started, undefined).pipe(
            Effect.andThen(Deferred.await(release)),
            Effect.as(pageFixture([note(1)], null, 5)),
          );
        }

        if (request.query?.before === "8") {
          return Deferred.succeed(returned, undefined).pipe(
            Effect.andThen(Deferred.await(releaseReturn)),
            Effect.as(pageFixture([note(8)], null, 9)),
          );
        }

        return Effect.succeed(pageFixture([note(9)], 8, null));
      });

      const older = yield* Effect.forkChild(session.loadOlder(ROOM));

      yield* Deferred.await(started);
      yield* session.reloadRoom(ROOM, null);

      const again = yield* Effect.forkChild(session.loadOlder(ROOM));

      yield* Deferred.await(returned);
      yield* Deferred.succeed(release, undefined);
      yield* Fiber.join(older);

      expect(timeline()?.loadingOlder).toBe(true);
      expect(timeline()?.ids).toEqual([9]);
      expect(timeline()?.before).toBe(8);

      yield* Deferred.succeed(releaseReturn, undefined);
      yield* Fiber.join(again);

      expect(timeline()?.loadingOlder).toBe(false);
      expect(timeline()?.ids).toEqual([8, 9]);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("interrupting a superseded older page leaves the later one loading", () =>
    Effect.gen(function* () {
      const fake = yield* FakeApi;
      const started = yield* Deferred.make<void>();
      const release = yield* Deferred.make<void>();
      const returned = yield* Deferred.make<void>();
      const releaseReturn = yield* Deferred.make<void>();

      mutations.applyPage(ROOM, pageFixture([note(5)], 4, null), "replace");
      yield* fake.reply(`GET /rooms/${ROOM}`, roomDetailFixture(ROOM));
      yield* fake.route(`GET /rooms/${ROOM}/messages`, (request) => {
        if (request.query?.before === "4") {
          return Deferred.succeed(started, undefined).pipe(
            Effect.andThen(Deferred.await(release)),
            Effect.as(pageFixture([note(1)], null, 5)),
          );
        }

        if (request.query?.before === "8") {
          return Deferred.succeed(returned, undefined).pipe(
            Effect.andThen(Deferred.await(releaseReturn)),
            Effect.as(pageFixture([note(8)], null, 9)),
          );
        }

        return Effect.succeed(pageFixture([note(9)], 8, null));
      });

      const older = yield* Effect.forkChild(session.loadOlder(ROOM));

      yield* Deferred.await(started);
      yield* session.reloadRoom(ROOM, null);

      const again = yield* Effect.forkChild(session.loadOlder(ROOM));

      yield* Deferred.await(returned);
      yield* Fiber.interrupt(older);

      expect(timeline()?.loadingOlder).toBe(true);
      expect(timeline()?.ids).toEqual([9]);

      yield* Deferred.succeed(releaseReturn, undefined);
      yield* Fiber.join(again);

      expect(timeline()?.ids).toEqual([8, 9]);
      expect(timeline()?.loadingOlder).toBe(false);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );
});
