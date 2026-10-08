import { afterEach, describe, expect, it } from "@effect/vitest";
import { Deferred, Effect, Fiber } from "effect";
import { ServerError } from "../api/errors.ts";
import { FakeApi, messageFixture, pageFixture } from "../api/testing.ts";
import { threadDetailFixture } from "../features/work/test-fixtures.ts";
import { mutations, store } from "../store/store.ts";
import * as threads from "./thread-actions.ts";

const THREAD = 70;

const ROOM = 4;

const reply = (id: number) => messageFixture(id, ROOM, { threadId: THREAD });

const timeline = () => store.getState().threadTimelines[THREAD];

describe("thread reply paging", () => {
  afterEach(() => mutations.reset());

  it.effect("pages forward after a window replaced around an older reply", () =>
    Effect.gen(function* () {
      const fake = yield* FakeApi;
      let loadingWhileReplacing: boolean | undefined;

      yield* fake.reply(`GET /threads/${THREAD}`, threadDetailFixture(THREAD, null, null));
      yield* fake.route(`GET /threads/${THREAD}/messages`, (request) => {
        if (request.query?.around !== undefined) {
          loadingWhileReplacing = timeline()?.loadingNewer;

          return Effect.succeed(pageFixture([reply(5)], null, 5));
        }

        return Effect.succeed(pageFixture([reply(6)], 6, null));
      });

      yield* threads.reload(THREAD, 5);

      expect(loadingWhileReplacing).toBe(true);
      expect(timeline()?.loadingNewer).toBe(false);
      expect(timeline()?.ids).toEqual([5]);
      expect(timeline()?.after).toBe(5);

      yield* threads.loadNewer(THREAD);

      const requests = yield* fake.requests;

      expect(requests).toContainEqual(
        expect.objectContaining({
          path: `/threads/${THREAD}/messages`,
          query: { after: "5" },
        }),
      );
      expect(timeline()?.loadingNewer).toBe(false);
      expect(timeline()?.ids).toEqual([5, 6]);
      expect(timeline()?.after).toBeNull();
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("clears the flag when the replaced window fails", () =>
    Effect.gen(function* () {
      const fake = yield* FakeApi;

      yield* fake.reply(`GET /threads/${THREAD}`, threadDetailFixture(THREAD, null, null));
      yield* fake.route(`GET /threads/${THREAD}/messages`, () =>
        Effect.fail(new ServerError({ status: 500, message: "nope" })),
      );

      yield* threads.reload(THREAD, 5);

      expect(timeline()?.loadingNewer).toBe(false);
      expect(timeline()?.loadingOlder).toBe(false);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("clears the flag when a replace or a forward page is interrupted", () =>
    Effect.gen(function* () {
      const fake = yield* FakeApi;
      const started = yield* Deferred.make<void>();
      const gate = yield* Deferred.make<void>();

      yield* fake.reply(`GET /threads/${THREAD}`, threadDetailFixture(THREAD, null, null));
      yield* fake.route(`GET /threads/${THREAD}/messages`, () =>
        Deferred.succeed(started, undefined).pipe(
          Effect.andThen(Deferred.await(gate)),
          Effect.as(pageFixture([reply(5)], null, 5)),
        ),
      );

      const replacing = yield* Effect.forkChild(threads.reload(THREAD, 5));

      yield* Deferred.await(started);
      expect(timeline()?.loadingNewer).toBe(true);

      yield* Fiber.interrupt(replacing);
      expect(timeline()?.loadingNewer).toBe(false);

      mutations.applyThreadPage(THREAD, pageFixture([reply(5)], null, 5), "replace");

      const paging = yield* Deferred.make<void>();
      const hold = yield* Deferred.make<void>();

      yield* fake.route(`GET /threads/${THREAD}/messages`, (request) =>
        Effect.gen(function* () {
          expect(request.query).toEqual({ after: "5" });
          yield* Deferred.succeed(paging, undefined);
          yield* Deferred.await(hold);

          return pageFixture([reply(6)], 6, null);
        }),
      );

      const forwarding = yield* Effect.forkChild(threads.loadNewer(THREAD));

      yield* Deferred.await(paging);
      expect(timeline()?.loadingNewer).toBe(true);

      yield* Fiber.interrupt(forwarding);
      expect(timeline()?.loadingNewer).toBe(false);
      expect(timeline()?.ids).toEqual([5]);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("clears the older flag when a prepend is interrupted", () =>
    Effect.gen(function* () {
      const fake = yield* FakeApi;
      const started = yield* Deferred.make<void>();
      const gate = yield* Deferred.make<void>();

      mutations.applyThreadPage(THREAD, pageFixture([reply(5)], 4, null), "replace");

      yield* fake.route(`GET /threads/${THREAD}/messages`, () =>
        Deferred.succeed(started, undefined).pipe(
          Effect.andThen(Deferred.await(gate)),
          Effect.as(pageFixture([reply(4)], null, 5)),
        ),
      );

      const loading = yield* Effect.forkChild(threads.loadOlder(THREAD));

      yield* Deferred.await(started);
      expect(timeline()?.loadingOlder).toBe(true);
      expect(timeline()?.loadingNewer).toBe(false);

      yield* Fiber.interrupt(loading);
      expect(timeline()?.loadingOlder).toBe(false);
      expect(timeline()?.loadingNewer).toBe(false);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("a failure in one direction leaves the other request running", () =>
    Effect.gen(function* () {
      const fake = yield* FakeApi;
      const olderStarted = yield* Deferred.make<void>();
      const newerStarted = yield* Deferred.make<void>();
      const releaseNewer = yield* Deferred.make<void>();

      mutations.applyThreadPage(THREAD, pageFixture([reply(5)], 4, 5), "replace");

      yield* fake.route(`GET /threads/${THREAD}/messages`, (request) => {
        if (request.query?.before !== undefined) {
          return Deferred.succeed(olderStarted, undefined).pipe(
            Effect.andThen(Effect.fail(new ServerError({ status: 500, message: "nope" }))),
          );
        }

        return Deferred.succeed(newerStarted, undefined).pipe(
          Effect.andThen(Deferred.await(releaseNewer)),
          Effect.as(pageFixture([reply(6)], 5, null)),
        );
      });

      const older = yield* Effect.forkChild(threads.loadOlder(THREAD));
      const newer = yield* Effect.forkChild(threads.loadNewer(THREAD));

      yield* Deferred.await(olderStarted);
      yield* Deferred.await(newerStarted);
      yield* Fiber.join(older);

      expect(timeline()?.loadingOlder).toBe(false);
      expect(timeline()?.loadingNewer).toBe(true);
      expect(timeline()?.ids).toEqual([5]);

      const newerRequests = () =>
        fake.requests.pipe(
          Effect.map((requests) =>
            requests.filter((request) => request.query?.after !== undefined),
          ),
        );

      expect(yield* newerRequests()).toHaveLength(1);

      yield* threads.loadNewer(THREAD);

      expect(yield* newerRequests()).toHaveLength(1);

      yield* Deferred.succeed(releaseNewer, undefined);
      yield* Fiber.join(newer);

      expect(timeline()?.loadingNewer).toBe(false);
      expect(timeline()?.ids).toEqual([5, 6]);
      expect(timeline()?.after).toBeNull();
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("drops an older page that lands after a replace", () =>
    Effect.gen(function* () {
      const fake = yield* FakeApi;
      const started = yield* Deferred.make<void>();
      const release = yield* Deferred.make<void>();

      mutations.applyThreadPage(THREAD, pageFixture([reply(5)], 4, null), "replace");
      yield* fake.reply(`GET /threads/${THREAD}`, threadDetailFixture(THREAD, null, null));
      yield* fake.route(`GET /threads/${THREAD}/messages`, (request) => {
        if (request.query?.before !== undefined) {
          return Deferred.succeed(started, undefined).pipe(
            Effect.andThen(Deferred.await(release)),
            Effect.as(pageFixture([reply(1)], null, 5)),
          );
        }

        return Effect.succeed(pageFixture([reply(9)], 8, null));
      });

      const older = yield* Effect.forkChild(threads.loadOlder(THREAD));

      yield* Deferred.await(started);
      yield* threads.reload(THREAD, null);

      expect(timeline()?.ids).toEqual([9]);
      expect(timeline()?.before).toBe(8);
      expect(timeline()?.loadingOlder).toBe(false);

      yield* Deferred.succeed(release, undefined);
      yield* Fiber.join(older);

      expect(timeline()?.ids).toEqual([9]);
      expect(timeline()?.before).toBe(8);
      expect(timeline()?.loadingOlder).toBe(false);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("a page started before leaving does not clear the one issued on return", () =>
    Effect.gen(function* () {
      const fake = yield* FakeApi;
      const started = yield* Deferred.make<void>();
      const release = yield* Deferred.make<void>();
      const returned = yield* Deferred.make<void>();
      const releaseReturn = yield* Deferred.make<void>();

      mutations.applyThreadPage(THREAD, pageFixture([reply(5)], 4, null), "replace");
      yield* fake.reply(`GET /threads/${THREAD}`, threadDetailFixture(THREAD, null, null));
      yield* fake.route(`GET /threads/${THREAD}/messages`, (request) => {
        if (request.query?.before === "4") {
          return Deferred.succeed(started, undefined).pipe(
            Effect.andThen(Deferred.await(release)),
            Effect.as(pageFixture([reply(1)], null, 5)),
          );
        }

        if (request.query?.before === "8") {
          return Deferred.succeed(returned, undefined).pipe(
            Effect.andThen(Deferred.await(releaseReturn)),
            Effect.as(pageFixture([reply(8)], null, 9)),
          );
        }

        return Effect.succeed(pageFixture([reply(9)], 8, null));
      });

      const older = yield* Effect.forkChild(threads.loadOlder(THREAD));

      yield* Deferred.await(started);
      yield* threads.reload(THREAD, null);

      const again = yield* Effect.forkChild(threads.loadOlder(THREAD));

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
      expect(timeline()?.before).toBeNull();
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("interrupting a superseded older page leaves the later one loading", () =>
    Effect.gen(function* () {
      const fake = yield* FakeApi;
      const started = yield* Deferred.make<void>();
      const release = yield* Deferred.make<void>();
      const returned = yield* Deferred.make<void>();
      const releaseReturn = yield* Deferred.make<void>();

      mutations.applyThreadPage(THREAD, pageFixture([reply(5)], 4, null), "replace");
      yield* fake.reply(`GET /threads/${THREAD}`, threadDetailFixture(THREAD, null, null));
      yield* fake.route(`GET /threads/${THREAD}/messages`, (request) => {
        if (request.query?.before === "4") {
          return Deferred.succeed(started, undefined).pipe(
            Effect.andThen(Deferred.await(release)),
            Effect.as(pageFixture([reply(1)], null, 5)),
          );
        }

        if (request.query?.before === "8") {
          return Deferred.succeed(returned, undefined).pipe(
            Effect.andThen(Deferred.await(releaseReturn)),
            Effect.as(pageFixture([reply(8)], null, 9)),
          );
        }

        return Effect.succeed(pageFixture([reply(9)], 8, null));
      });

      const older = yield* Effect.forkChild(threads.loadOlder(THREAD));

      yield* Deferred.await(started);
      yield* threads.reload(THREAD, null);

      const again = yield* Effect.forkChild(threads.loadOlder(THREAD));

      yield* Deferred.await(returned);
      yield* Fiber.interrupt(older);

      expect(timeline()?.loadingOlder).toBe(true);
      expect(timeline()?.ids).toEqual([9]);
      expect(timeline()?.before).toBe(8);

      yield* Deferred.succeed(releaseReturn, undefined);
      yield* Fiber.join(again);

      expect(timeline()?.loadingOlder).toBe(false);
      expect(timeline()?.ids).toEqual([8, 9]);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );
});
