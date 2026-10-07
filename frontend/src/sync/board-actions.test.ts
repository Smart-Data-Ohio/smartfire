import { afterEach, describe, expect, it } from "@effect/vitest";
import { Deferred, Effect, Fiber, Result } from "effect";
import { Validation } from "../api/errors.ts";
import { FakeApi, roomDetailFixture } from "../api/testing.ts";
import { boardPostIds } from "../store/boards.ts";
import { mutations, store } from "../store/store.ts";
import { BOARD, boardDetail, boardListing, boardThread } from "../test/board-fixtures.ts";
import * as boards from "./board-actions.ts";
import { Presence } from "./presence.ts";
import { asAction } from "./run.ts";
import { jumpToPresent, loadAround, loadOlder, openRoom } from "./session.ts";
import { Topics } from "./topics.ts";
import { changedWorkPanes, refreshWorkPane } from "./work-refresh.ts";

const all = { status: "all", owner: "anyone", tag: "" } as const;

afterEach(() => mutations.reset());

describe("board actions", () => {
  it.effect("ignores stale query responses, including all → done → all", () =>
    Effect.gen(function* () {
      const fake = yield* FakeApi;
      const first = yield* Deferred.make<ReturnType<typeof boardListing>>();
      const entered = yield* Deferred.make<void>();
      let calls = 0;
      yield* fake.route(`GET /rooms/${BOARD}/board`, () => {
        calls += 1;

        return calls === 1
          ? Effect.andThen(Deferred.succeed(entered, undefined), Deferred.await(first))
          : Effect.succeed(boardListing([boardThread(calls)]));
      });
      const old = yield* Effect.forkChild(boards.open(BOARD, all));
      yield* Deferred.await(entered);
      yield* boards.open(BOARD, { ...all, status: "done" });
      yield* boards.open(BOARD, all);
      yield* Deferred.succeed(first, boardListing([boardThread(99)]));
      yield* Fiber.join(old);
      expect(store.getState().boards[BOARD]?.query).toEqual(all);
      expect(boardPostIds(store.getState(), BOARD)).toEqual([3]);
      expect(store.getState().threads[99]).toBeUndefined();
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("loads the cumulative next page once and keeps the earlier page on error", () =>
    Effect.gen(function* () {
      const fake = yield* FakeApi;
      yield* fake.reply(`GET /rooms/${BOARD}/board`, { ...boardListing(), hasMore: true });
      yield* boards.open(BOARD, all);
      const waiting = yield* Deferred.make<ReturnType<typeof boardListing>>();
      const entered = yield* Deferred.make<void>();
      yield* fake.route(`GET /rooms/${BOARD}/board`, () =>
        Effect.andThen(Deferred.succeed(entered, undefined), Deferred.await(waiting)),
      );
      const more = yield* Effect.forkChild(boards.loadMore(BOARD));
      yield* Deferred.await(entered);
      yield* boards.loadMore(BOARD);
      expect(yield* fake.requests).toHaveLength(2);
      yield* Deferred.succeed(waiting, {
        ...boardListing([boardThread(2), boardThread(1)]),
        page: 2,
        hasMore: true,
      });
      yield* Fiber.join(more);
      expect(store.getState().boards[BOARD]?.postIds).toEqual([2, 1]);
      expect((yield* fake.requests).at(-1)?.query).toEqual({ ...all, page: "2" });
      yield* fake.route(`GET /rooms/${BOARD}/board`, () =>
        Effect.fail(new Validation({ message: "Try again", fields: {} })),
      );
      yield* boards.loadMore(BOARD);
      expect(store.getState().boards[BOARD]?.status).toBe("ready");
      expect(store.getState().boards[BOARD]?.loadingMore).toBe(false);
      expect(store.getState().boards[BOARD]?.error).toBe("Try again");
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect(
    "merges form users, creates posts with a UUID brief, and sends null for blank briefs",
    () =>
      Effect.gen(function* () {
        const fake = yield* FakeApi;
        yield* fake.reply(`GET /rooms/${BOARD}/posts/new`, {
          ownerCandidates: [],
          tagSuggestions: [],
          users: boardDetail().users,
        });
        yield* boards.postForm(BOARD);
        expect(store.getState().users[7]?.id).toBe(7);
        yield* fake.reply(`GET /rooms/${BOARD}/board`, boardListing([]));
        yield* boards.open(BOARD, all);
        yield* fake.reply(`POST /rooms/${BOARD}/posts`, boardDetail());

        const input = {
          name: "New",
          status: "planned",
          ownerId: null,
          tags: ["api"],
          brief: "Brief",
        } as const;

        yield* boards.createPost(BOARD, input);
        const body = (yield* fake.requests).at(-1)?.body;
        expect(body).toMatchObject({
          message: {
            markdownSource: "Brief",
            clientMessageId: expect.stringMatching(/^[0-9a-f-]{14}7[0-9a-f-]+$/),
          },
        });
        expect(boardPostIds(store.getState(), BOARD)).toEqual([1]);
        expect(store.getState().threadPanes[1]?.work).toEqual(boardDetail().work);
        yield* boards.createPost(BOARD, { ...input, brief: " \n " });
        expect((yield* fake.requests).at(-1)?.body).toMatchObject({ message: null });
      }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("applies returned work details and exposes validation fields through ActionError", () =>
    Effect.gen(function* () {
      const fake = yield* FakeApi;
      const detail = boardDetail(boardThread(1, "done", 7, ["api"]));
      yield* fake.reply("PATCH /threads/1/work", detail);
      yield* boards.update(1, { status: "done", tags: ["api"] });
      expect(store.getState().threadPanes[1]?.workFacts?.status).toBe("done");
      yield* fake.reply("POST /threads/1/work/handoff", detail);
      yield* boards.handoff(1, {
        receiverAgentId: 9,
        summary: "Continue",
        links: [],
        openQuestions: [],
      });
      yield* fake.route("PATCH /threads/1/work", () =>
        Effect.fail(new Validation({ message: "Bad tags", fields: { tags: ["Invalid tag"] } })),
      );
      const rejected = yield* Effect.result(asAction(boards.update(1, { tags: ["bad tag"] })));
      expect(Result.isFailure(rejected) ? rejected.failure.fields : null).toEqual({
        tags: ["Invalid tag"],
      });
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );
});

describe("work sync and board room sessions", () => {
  it.effect("refetches only open panes with changed work facts and keeps newer live rows", () =>
    Effect.gen(function* () {
      const fake = yield* FakeApi;
      mutations.loadThreadDetail(boardDetail());
      const changed = boardThread(1, "done");

      const events = [
        { seq: 1, topic: "room:900", type: "thread.updated", data: changed },
      ] as const;

      mutations.applyEvents(events, 0);
      expect(changedWorkPanes(store.getState(), events, [])).toEqual([]);
      expect(changedWorkPanes(store.getState(), events, ["thread:1"])).toEqual([1]);
      yield* fake.reply("GET /threads/1", boardDetail(changed));
      yield* refreshWorkPane(1);
      expect(store.getState().threadPanes[1]?.workFacts?.status).toBe("done");
      expect(changedWorkPanes(store.getState(), events, ["thread:1"])).toEqual([]);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect(
    "learns board kind from detail and keeps its room subscription without message requests",
    () =>
      Effect.gen(function* () {
        const fake = yield* FakeApi;
        const detail = roomDetailFixture(BOARD);
        detail.room.kind = "board";
        yield* fake.reply(`GET /rooms/${BOARD}`, detail);
        const acquired: string[] = [];
        yield* openRoom(BOARD, null).pipe(
          Effect.provideService(
            Topics,
            Topics.of({
              acquire: (topic) =>
                Effect.sync(() => {
                  acquired.push(topic);
                }),
              release: () => Effect.void,
              subscribed: Effect.succeed([]),
            }),
          ),
          Effect.provideService(
            Presence,
            Presence.of({
              enter: () => Effect.void,
              leave: () => Effect.void,
              run: Effect.void,
              noteActivity: Effect.void,
              greeting: Effect.succeed([]),
            }),
          ),
        );
        yield* loadOlder(BOARD);
        yield* loadAround(BOARD, 1);
        yield* jumpToPresent(BOARD);
        expect(acquired).toEqual([`room:${BOARD}`]);
        expect((yield* fake.requests).map((request) => request.path)).toEqual([`/rooms/${BOARD}`]);
        expect(store.getState().timelines[BOARD]?.status).toBe("idle");
      }).pipe(Effect.provide(FakeApi.layerClient)),
  );
});

it.effect("refetches again when newer work facts arrive during a detail request", () =>
  Effect.gen(function* () {
    const fake = yield* FakeApi;
    mutations.loadThreadDetail(boardDetail());
    mutations.upsertThread(boardThread(1, "done"));
    const waiting = yield* Deferred.make<ReturnType<typeof boardDetail>>();
    const entered = yield* Deferred.make<void>();
    let calls = 0;
    yield* fake.route("GET /threads/1", () => {
      calls += 1;

      return calls === 1
        ? Effect.andThen(Deferred.succeed(entered, undefined), Deferred.await(waiting))
        : Effect.succeed(boardDetail(boardThread(1, "blocked")));
    });
    const refresh = yield* Effect.forkChild(refreshWorkPane(1));
    yield* Deferred.await(entered);
    mutations.upsertThread(boardThread(1, "blocked"));
    yield* Deferred.succeed(waiting, boardDetail(boardThread(1, "done")));
    yield* Fiber.join(refresh);
    expect(calls).toBe(2);
    expect(store.getState().threadPanes[1]?.workFacts?.status).toBe("blocked");
  }).pipe(Effect.provide(FakeApi.layerClient)),
);

it.effect("keeps a reply's newer activity when work detail lands", () =>
  Effect.gen(function* () {
    const fake = yield* FakeApi;
    const old = boardThread(1, "done");
    mutations.loadThreadDetail(boardDetail());
    mutations.upsertThread(old);
    const waiting = yield* Deferred.make<ReturnType<typeof boardDetail>>();
    const entered = yield* Deferred.make<void>();
    yield* fake.route("GET /threads/1", () =>
      Effect.andThen(Deferred.succeed(entered, undefined), Deferred.await(waiting)),
    );
    const refresh = yield* Effect.forkChild(refreshWorkPane(1));
    yield* Deferred.await(entered);
    const live = { ...old, replyCount: 2, lastActivityAt: "2026-10-07T12:00:00.000Z" };
    mutations.upsertThread(live);
    yield* Deferred.succeed(waiting, boardDetail(old));
    yield* Fiber.join(refresh);
    expect(store.getState().threads[1]).toEqual(live);
    expect(store.getState().threadPanes[1]?.workFacts?.status).toBe("done");
  }).pipe(Effect.provide(FakeApi.layerClient)),
);
