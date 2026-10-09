import { afterEach, describe, expect, it } from "@effect/vitest";
import { Deferred, Effect, Fiber, Layer } from "effect";
import { ServerError } from "../api/errors.ts";
import { FakeApi, messageFixture, pageFixture, roomDetailFixture } from "../api/testing.ts";
import { mutations, store } from "../store/store.ts";
import { SyncLink } from "./link.ts";
import { Presence } from "./presence.ts";
import * as session from "./session.ts";
import { TestLifecycle } from "./testing.ts";
import { Topics } from "./topics.ts";
import { Typing } from "./typing.ts";

const ROOM = 12;

const OTHER = 13;

const note = (id: number) => messageFixture(id, ROOM);

const timeline = () => store.getState().timelines[ROOM];

const navigation = Layer.mergeAll(Topics.layer, Typing.layer, Presence.layer).pipe(
  Layer.provideMerge(Layer.mergeAll(SyncLink.layer, TestLifecycle.layer, FakeApi.layerClient)),
);

describe("room timeline paging", () => {
  afterEach(() => mutations.reset());

  it.effect("doesn't page newer while the jump to the present loads", () =>
    Effect.gen(function* () {
      const fake = yield* FakeApi;
      const latestStarted = yield* Deferred.make<void>();
      const releaseLatest = yield* Deferred.make<void>();
      let newerPages = 0;

      mutations.applyPage(ROOM, pageFixture([note(5)], 4, 5), "replace");

      yield* fake.route(`GET /rooms/${ROOM}/messages`, (request) => {
        if (request.query?.after !== undefined) {
          newerPages += 1;

          return Effect.succeed(pageFixture([note(6)], 5, 6));
        }

        return Deferred.succeed(latestStarted, undefined).pipe(
          Effect.andThen(Deferred.await(releaseLatest)),
          Effect.as(pageFixture([note(9)], 8, null)),
        );
      });

      const jump = yield* Effect.forkChild(session.jumpToPresent(ROOM));

      yield* Deferred.await(latestStarted);
      yield* session.loadNewer(ROOM);

      expect(newerPages).toBe(0);
      expect(timeline()?.ids).toEqual([5]);
      expect(timeline()?.loadingNewer).toBe(false);

      yield* Deferred.succeed(releaseLatest, undefined);
      yield* Fiber.join(jump);

      expect(timeline()?.ids).toEqual([9]);
      expect(timeline()?.after).toBeNull();
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("a failure in one direction leaves the other request running", () =>
    Effect.gen(function* () {
      const fake = yield* FakeApi;
      const olderStarted = yield* Deferred.make<void>();
      const newerStarted = yield* Deferred.make<void>();
      const releaseOlder = yield* Deferred.make<void>();
      const releaseNewer = yield* Deferred.make<void>();

      mutations.applyPage(ROOM, pageFixture([note(5)], 4, 5), "replace");

      yield* fake.route(`GET /rooms/${ROOM}/messages`, (request) => {
        if (request.query?.before !== undefined) {
          return Deferred.succeed(olderStarted, undefined).pipe(
            Effect.andThen(Deferred.await(releaseOlder)),
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
      yield* Deferred.succeed(releaseOlder, undefined);
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

  it.effect("a page started before switching rooms does not clear the one issued on return", () =>
    Effect.gen(function* () {
      const fake = yield* FakeApi;
      const started = yield* Deferred.make<void>();
      const release = yield* Deferred.make<void>();
      const returned = yield* Deferred.make<void>();
      const releaseReturn = yield* Deferred.make<void>();

      mutations.applyPage(ROOM, pageFixture([note(5)], 4, null), "replace");
      yield* fake.reply(`GET /rooms/${ROOM}`, roomDetailFixture(ROOM));
      yield* fake.reply(`GET /rooms/${OTHER}`, roomDetailFixture(OTHER));
      yield* fake.reply(
        `GET /rooms/${OTHER}/messages`,
        pageFixture([messageFixture(30, OTHER)], null, null),
      );
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
      yield* session.closeRoom(ROOM);
      yield* session.openRoom(OTHER, null);
      yield* session.closeRoom(OTHER);
      yield* session.openRoom(ROOM, null);

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
    }).pipe(Effect.provide(navigation)),
  );

  it.effect("a stale failure leaves the request that replaced it running", () =>
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
            Effect.andThen(Effect.fail(new ServerError({ status: 500, message: "nope" }))),
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

  it.effect(
    "a page that arrives while a replacement is still pending leaves the other request running",
    () =>
      Effect.gen(function* () {
        const fake = yield* FakeApi;
        const olderStarted = yield* Deferred.make<void>();
        const newerStarted = yield* Deferred.make<void>();
        const replacing = yield* Deferred.make<void>();
        const releaseOlder = yield* Deferred.make<void>();
        const releaseNewer = yield* Deferred.make<void>();
        const releaseReplace = yield* Deferred.make<void>();

        mutations.applyPage(ROOM, pageFixture([note(5)], 4, 5), "replace");
        yield* fake.reply(`GET /rooms/${ROOM}`, roomDetailFixture(ROOM));
        yield* fake.route(`GET /rooms/${ROOM}/messages`, (request) => {
          if (request.query?.before !== undefined) {
            return Deferred.succeed(olderStarted, undefined).pipe(
              Effect.andThen(Deferred.await(releaseOlder)),
              Effect.as(pageFixture([note(4)], null, 5)),
            );
          }

          if (request.query?.after !== undefined) {
            return Deferred.succeed(newerStarted, undefined).pipe(
              Effect.andThen(Deferred.await(releaseNewer)),
              Effect.as(pageFixture([note(6)], 5, null)),
            );
          }

          return Deferred.succeed(replacing, undefined).pipe(
            Effect.andThen(Deferred.await(releaseReplace)),
            Effect.as(pageFixture([note(9)], 8, null)),
          );
        });

        const older = yield* Effect.forkChild(session.loadOlder(ROOM));
        const newer = yield* Effect.forkChild(session.loadNewer(ROOM));

        yield* Deferred.await(olderStarted);
        yield* Deferred.await(newerStarted);

        const replaced = yield* Effect.forkChild(session.reloadRoom(ROOM, null));

        yield* Deferred.await(replacing);
        yield* Deferred.succeed(releaseNewer, undefined);
        yield* Fiber.join(newer);

        expect(timeline()?.loadingOlder).toBe(true);
        expect(timeline()?.ids).toEqual([5, 6]);

        yield* Deferred.succeed(releaseReplace, undefined);
        yield* Fiber.join(replaced);

        expect(timeline()?.ids).toEqual([9]);
        expect(timeline()?.before).toBe(8);
        expect(timeline()?.loadingOlder).toBe(false);

        yield* Deferred.succeed(releaseOlder, undefined);
        yield* Fiber.join(older);

        expect(timeline()?.ids).toEqual([9]);
        expect(timeline()?.before).toBe(8);
        expect(timeline()?.loadingOlder).toBe(false);
        expect(timeline()?.loadingNewer).toBe(false);
      }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  for (const first of ["older", "newer"] as const) {
    it.effect(`both pages land when the ${first} one finishes first`, () =>
      Effect.gen(function* () {
        const fake = yield* FakeApi;
        const olderStarted = yield* Deferred.make<void>();
        const newerStarted = yield* Deferred.make<void>();
        const releaseOlder = yield* Deferred.make<void>();
        const releaseNewer = yield* Deferred.make<void>();

        mutations.applyPage(ROOM, pageFixture([note(5)], 4, 5), "replace");

        yield* fake.route(`GET /rooms/${ROOM}/messages`, (request) => {
          if (request.query?.before !== undefined) {
            return Deferred.succeed(olderStarted, undefined).pipe(
              Effect.andThen(Deferred.await(releaseOlder)),
              Effect.as(pageFixture([note(4)], null, 5)),
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

        if (first === "older") {
          yield* Deferred.succeed(releaseOlder, undefined);
          yield* Fiber.join(older);

          expect(timeline()?.loadingOlder).toBe(false);
          expect(timeline()?.loadingNewer).toBe(true);
          expect(timeline()?.ids).toEqual([4, 5]);

          yield* Deferred.succeed(releaseNewer, undefined);
          yield* Fiber.join(newer);
        } else {
          yield* Deferred.succeed(releaseNewer, undefined);
          yield* Fiber.join(newer);

          expect(timeline()?.loadingOlder).toBe(true);
          expect(timeline()?.loadingNewer).toBe(false);
          expect(timeline()?.ids).toEqual([5, 6]);

          yield* Deferred.succeed(releaseOlder, undefined);
          yield* Fiber.join(older);
        }

        expect(timeline()?.loadingOlder).toBe(false);
        expect(timeline()?.loadingNewer).toBe(false);
        expect(timeline()?.ids).toEqual([4, 5, 6]);
        expect(timeline()?.before).toBeNull();
        expect(timeline()?.after).toBeNull();
      }).pipe(Effect.provide(FakeApi.layerClient)),
    );
  }
});
