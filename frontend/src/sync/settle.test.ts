import { afterEach, describe, expect, it } from "@effect/vitest";
import { Deferred, Effect, Fiber, type Schema } from "effect";
import { NotFound } from "../api/errors.ts";
import { FakeApi, pageFixture } from "../api/testing.ts";
import type { ThreadDetail } from "../gen/ThreadDetail.ts";
import type { ThreadList } from "../gen/ThreadList.ts";
import { boardPostIds } from "../store/boards.ts";
import { mutations, store } from "../store/store.ts";
import { MAX_REMOVED_THREADS, THREAD_DELETED } from "../store/threads.ts";
import { BOARD, boardDetail, boardListing, boardThread } from "../test/board-fixtures.ts";
import * as boards from "./board-actions.ts";
import { MAX_SETTLE_ATTEMPTS } from "./settle.ts";
import * as threads from "./thread-actions.ts";

afterEach(() => mutations.reset());

let seq = 0;

/** Removes `threadIds` (by default, more unrelated threads than the store remembers). */
function remove(
  threadIds: readonly number[] = Array.from(
    { length: MAX_REMOVED_THREADS + 1 },
    (_, index) => 20_000 + seq + index,
  ),
) {
  mutations.applyEvents(
    threadIds.map((threadId) => {
      seq += 1;

      return {
        seq,
        topic: `room:${BOARD}`,
        type: "thread.removed" as const,
        data: { threadId, roomId: BOARD },
      };
    }),
    0,
  );
}

const listing = (): ThreadList => ({
  threads: [{ thread: boardThread(1), membership: null }],
  users: [],
});

/**
 * Routes `key`: the first request waits until the test answers it, and `during` runs while each
 * request is in flight. Later requests answer `reply` at once.
 */
const slowFirst = <A extends Schema.Json>(
  key: string,
  reply: () => A,
  during: (call: number) => void = () => {},
) =>
  Effect.gen(function* () {
    const fake = yield* FakeApi;
    const first = yield* Deferred.make<A>();
    const entered = yield* Deferred.make<void>();
    let calls = 0;

    yield* fake.route(key, () => {
      calls += 1;
      during(calls);

      return calls === 1
        ? Effect.andThen(Deferred.succeed(entered, undefined), Deferred.await(first))
        : Effect.succeed(reply());
    });

    return { first, entered, calls: () => calls };
  });

describe("replies that can't tell whether a thread was removed", () => {
  it.effect("still loads a thread pane after 501 unrelated removals while its detail was out", () =>
    Effect.gen(function* () {
      const fake = yield* FakeApi;
      yield* fake.reply("GET /threads/1/messages", pageFixture([]));
      const detail = yield* slowFirst<ThreadDetail>("GET /threads/1", () => boardDetail());

      const load = yield* Effect.forkChild(threads.reload(1));

      yield* Deferred.await(detail.entered);
      remove();
      yield* Deferred.succeed(detail.first, boardDetail());
      yield* Fiber.join(load);

      expect(store.getState().threadPanes[1]?.status).toBe("ready");
      expect(store.getState().threads[1]?.name).toBe("Post 1");
      expect(detail.calls()).toBe(2);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("installs the pane's header as it lands, before removals during the replies", () =>
    Effect.gen(function* () {
      const fake = yield* FakeApi;
      const entered = yield* Deferred.make<void>();
      const release = yield* Deferred.make<void>();
      let details = 0;

      yield* fake.route("GET /threads/1", () => {
        details += 1;

        return Effect.succeed(boardDetail());
      });
      yield* fake.route("GET /threads/1/messages", () =>
        Effect.andThen(
          Effect.andThen(Deferred.succeed(entered, undefined), Deferred.await(release)),
          Effect.succeed(pageFixture([])),
        ),
      );

      const load = yield* Effect.forkChild(threads.reload(1));

      yield* Deferred.await(entered);

      // The header answers while the replies are still out.
      for (let turn = 0; turn < 50; turn += 1) {
        yield* Effect.yieldNow;
      }

      remove();
      yield* Deferred.succeed(release, undefined);
      yield* Fiber.join(load);

      expect(store.getState().threadPanes[1]?.status).toBe("ready");
      expect(store.getState().threads[1]?.name).toBe("Post 1");
      expect(details).toBe(1);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("says the thread was deleted when asking again finds it gone", () =>
    Effect.gen(function* () {
      const fake = yield* FakeApi;
      yield* fake.reply("GET /threads/1/messages", pageFixture([]));
      const first = yield* Deferred.make<ThreadDetail>();
      const entered = yield* Deferred.make<void>();
      let details = 0;

      yield* fake.route("GET /threads/1", () => {
        details += 1;

        return details === 1
          ? Effect.andThen(Deferred.succeed(entered, undefined), Deferred.await(first))
          : Effect.fail(new NotFound({ message: "Not found" }));
      });

      const load = yield* Effect.forkChild(threads.reload(1));

      yield* Deferred.await(entered);
      // Its own removal, then enough others that it's no longer remembered.
      remove([1]);
      remove(Array.from({ length: MAX_REMOVED_THREADS }, (_, index) => 30_000 + seq + index));
      expect(store.getState().removedThreads[1]).toBeUndefined();
      yield* Deferred.succeed(first, boardDetail());
      yield* Fiber.join(load);

      expect(details).toBe(2);
      expect(store.getState().threads[1]).toBeUndefined();
      expect(store.getState().threadPanes[1]).toMatchObject({
        status: "error",
        error: THREAD_DELETED,
      });
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("still lists a thread after 501 unrelated removals while the listing was out", () =>
    Effect.gen(function* () {
      const list = yield* slowFirst<ThreadList>(`GET /rooms/${BOARD}/threads`, listing);

      const load = yield* Effect.forkChild(threads.list(BOARD, "all"));

      yield* Deferred.await(list.entered);
      remove();
      yield* Deferred.succeed(list.first, listing());
      yield* Fiber.join(load);

      expect(store.getState().roomThreads[BOARD]).toEqual({
        filter: "all",
        ids: [1],
        status: "ready",
      });
      expect(list.calls()).toBe(2);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("still adds a new post to the board after 501 unrelated removals during its POST", () =>
    Effect.gen(function* () {
      const fake = yield* FakeApi;
      yield* fake.reply(`GET /rooms/${BOARD}/board`, boardListing([]));
      yield* fake.reply("GET /threads/1", boardDetail());
      yield* boards.open(BOARD, { status: "all", owner: "anyone", tag: "" });

      const post = yield* slowFirst<ThreadDetail>(`POST /rooms/${BOARD}/posts`, () =>
        boardDetail(),
      );

      const create = yield* Effect.forkChild(
        boards.createPost(BOARD, {
          name: "New",
          status: "planned",
          ownerId: null,
          tags: [],
          brief: "",
          clientId: "0192a3b4-0000-7000-8000-00000000c2af",
        }),
      );

      yield* Deferred.await(post.entered);
      remove();
      yield* Deferred.succeed(post.first, boardDetail());
      yield* Fiber.join(create);

      expect(boardPostIds(store.getState(), BOARD)).toEqual([1]);
      // The POST isn't repeated: the post is asked for by id.
      expect(post.calls()).toBe(1);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("doesn't ask again for a thread whose removal is remembered", () =>
    Effect.gen(function* () {
      const fake = yield* FakeApi;
      yield* fake.reply("GET /threads/1/messages", pageFixture([]));
      const detail = yield* slowFirst<ThreadDetail>("GET /threads/1", () => boardDetail());

      const load = yield* Effect.forkChild(threads.reload(1));

      yield* Deferred.await(detail.entered);
      remove([1]);
      yield* Deferred.succeed(detail.first, boardDetail());
      yield* Fiber.join(load);

      expect(store.getState().threads[1]).toBeUndefined();
      expect(store.getState().threadPanes[1]?.error).toBe("This thread was deleted.");
      expect(detail.calls()).toBe(1);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("gives up on the pane, with an error, when removals outrun every attempt", () =>
    Effect.gen(function* () {
      const fake = yield* FakeApi;
      yield* fake.reply("GET /threads/1/messages", pageFixture([]));

      const detail = yield* slowFirst<ThreadDetail>(
        "GET /threads/1",
        () => boardDetail(),
        (call) => {
          if (call > 1) remove();
        },
      );

      const load = yield* Effect.forkChild(threads.reload(1));

      yield* Deferred.await(detail.entered);
      remove();
      yield* Deferred.succeed(detail.first, boardDetail());
      yield* Fiber.join(load);

      expect(detail.calls()).toBe(MAX_SETTLE_ATTEMPTS);
      expect(store.getState().threadPanes[1]).toMatchObject({
        status: "error",
        error: threads.UNSETTLED,
      });
      expect(store.getState().threads[1]).toBeUndefined();
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("gives up on the listing, as failed, when removals outrun every attempt", () =>
    Effect.gen(function* () {
      const list = yield* slowFirst<ThreadList>(`GET /rooms/${BOARD}/threads`, listing, (call) => {
        if (call > 1) remove();
      });

      const load = yield* Effect.forkChild(threads.list(BOARD, "all"));

      yield* Deferred.await(list.entered);
      remove();
      yield* Deferred.succeed(list.first, listing());
      yield* Fiber.join(load);

      expect(list.calls()).toBe(MAX_SETTLE_ATTEMPTS);
      expect(store.getState().roomThreads[BOARD]?.status).toBe("error");
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );
});
