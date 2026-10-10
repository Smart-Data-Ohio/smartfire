import { afterEach, describe, expect, it } from "@effect/vitest";
import { Deferred, Effect, Fiber, Result } from "effect";
import { Validation } from "../api/errors.ts";
import { FakeApi, roomDetailFixture } from "../api/testing.ts";
import { boardPostIds } from "../store/boards.ts";
import { mutations, store } from "../store/store.ts";
import { MAX_REMOVED_THREADS } from "../store/threads.ts";
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
          tags: [],
          tagsRequired: false,
          defaultBoardTagId: null,
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
          clientId: "0192a3b4-0000-7000-8000-00000000c1ad",
        } as const;

        yield* boards.createPost(BOARD, input);
        const body = (yield* fake.requests).at(-1)?.body;
        expect(body).toMatchObject({
          clientPostId: input.clientId,
          message: { markdownSource: "Brief", clientMessageId: input.clientId },
        });
        expect(boardPostIds(store.getState(), BOARD)).toEqual([1]);
        expect(store.getState().threadPanes[1]?.work).toEqual(boardDetail().work);
        yield* boards.createPost(BOARD, { ...input, brief: " \n " });
        expect((yield* fake.requests).at(-1)?.body).toMatchObject({
          message: null,
          clientPostId: input.clientId,
        });
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
  it("doesn't refetch a pane's work when only its message count moved", () => {
    mutations.loadThreadDetail(boardDetail(), 0);
    const replied = boardThread(1);

    if (replied.work !== null) replied.work.messageCount = 5;

    const events = [{ seq: 1, topic: "room:900", type: "thread.updated", data: replied }] as const;

    mutations.applyEvents(events, 0);
    expect(store.getState().threads[1]?.work?.messageCount).toBe(5);
    expect(changedWorkPanes(store.getState(), events, ["thread:1"])).toEqual([]);
  });

  it.effect("refetches only open panes with changed work facts and keeps newer live rows", () =>
    Effect.gen(function* () {
      const fake = yield* FakeApi;
      mutations.loadThreadDetail(boardDetail(), 0);
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
              forgetRejected: () => Effect.void,
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
    mutations.loadThreadDetail(boardDetail(), 0);
    mutations.upsertThread(boardThread(1, "done", null, [], 1));
    const waiting = yield* Deferred.make<ReturnType<typeof boardDetail>>();
    const entered = yield* Deferred.make<void>();
    let calls = 0;
    yield* fake.route("GET /threads/1", () => {
      calls += 1;

      return calls === 1
        ? Effect.andThen(Deferred.succeed(entered, undefined), Deferred.await(waiting))
        : Effect.succeed(boardDetail(boardThread(1, "blocked", null, [], 2)));
    });
    const refresh = yield* Effect.forkChild(refreshWorkPane(1));
    yield* Deferred.await(entered);
    mutations.upsertThread(boardThread(1, "blocked", null, [], 2));
    yield* Deferred.succeed(waiting, boardDetail(boardThread(1, "done", null, [], 1)));
    yield* Fiber.join(refresh);
    expect(calls).toBe(2);
    expect(store.getState().threadPanes[1]?.workFacts?.status).toBe("blocked");
  }).pipe(Effect.provide(FakeApi.layerClient)),
);

it.effect("keeps a reply's newer activity when work detail lands", () =>
  Effect.gen(function* () {
    const fake = yield* FakeApi;
    const old = boardThread(1, "done", null, [], 1);
    mutations.loadThreadDetail(boardDetail(), 0);
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

it.effect("doesn't let a late save reply undo a newer change that arrived meanwhile", () =>
  Effect.gen(function* () {
    const fake = yield* FakeApi;
    mutations.loadThreadDetail(boardDetail(boardThread(1, "planned")), 0);
    const reply = yield* Deferred.make<ReturnType<typeof boardDetail>>();
    const entered = yield* Deferred.make<void>();
    yield* fake.route("PATCH /threads/1/work", () =>
      Effect.andThen(Deferred.succeed(entered, undefined), Deferred.await(reply)),
    );
    yield* fake.reply("GET /threads/1", boardDetail(boardThread(1, "blocked", 8, [], 2)));
    const save = yield* Effect.forkChild(boards.update(1, { status: "in_progress" }));
    yield* Deferred.await(entered);
    // Another member's later change lands while our reply is held up.
    mutations.applyEvents(
      [
        {
          seq: 1,
          topic: "room:900",
          type: "thread.updated",
          data: boardThread(1, "blocked", 8, [], 2),
        },
      ],
      0,
    );
    yield* Deferred.succeed(reply, boardDetail(boardThread(1, "in_progress", null, [], 1)));
    yield* Fiber.join(save);
    expect(store.getState().threads[1]?.work?.status).toBe("blocked");
    expect(store.getState().threadPanes[1]?.workFacts?.status).toBe("blocked");
    expect(store.getState().threadPanes[1]?.workFacts?.owner?.id).toBe(8);
  }).pipe(Effect.provide(FakeApi.layerClient)),
);

it.effect("installs a save reply at once when nothing changed while it was in flight", () =>
  Effect.gen(function* () {
    const fake = yield* FakeApi;
    mutations.loadThreadDetail(boardDetail(boardThread(1, "planned")), 0);
    yield* fake.reply("PATCH /threads/1/work", boardDetail(boardThread(1, "done")));
    yield* boards.update(1, { status: "done" });
    expect(store.getState().threadPanes[1]?.workFacts?.status).toBe("done");
    expect((yield* fake.requests).map((request) => request.method)).toEqual(["PATCH"]);
  }).pipe(Effect.provide(FakeApi.layerClient)),
);

it.effect("doesn't bring back a post removed while its creation was in flight", () =>
  Effect.gen(function* () {
    const fake = yield* FakeApi;
    yield* fake.reply(`GET /rooms/${BOARD}/board`, boardListing([]));
    yield* boards.open(BOARD, all);
    const reply = yield* Deferred.make<ReturnType<typeof boardDetail>>();
    const entered = yield* Deferred.make<void>();
    yield* fake.route(`POST /rooms/${BOARD}/posts`, () =>
      Effect.andThen(Deferred.succeed(entered, undefined), Deferred.await(reply)),
    );

    const create = yield* Effect.forkChild(
      boards.createPost(BOARD, {
        name: "New",
        status: "planned",
        ownerId: null,
        tags: [],
        brief: "",
        clientId: "0192a3b4-0000-7000-8000-00000000c1ae",
      }),
    );

    yield* Deferred.await(entered);
    mutations.applyEvents(
      [{ seq: 1, topic: "room:900", type: "thread.created", data: boardThread(1) }],
      0,
    );
    mutations.applyEvents(
      [{ seq: 2, topic: "room:900", type: "thread.removed", data: { threadId: 1, roomId: BOARD } }],
      0,
    );
    yield* Deferred.succeed(reply, boardDetail());
    yield* Fiber.join(create);
    expect(store.getState().threads[1]).toBeUndefined();
    expect(boardPostIds(store.getState(), BOARD)).toEqual([]);
  }).pipe(Effect.provide(FakeApi.layerClient)),
);

it.effect(
  "doesn't bring back a post whose removal was forgotten while its creation was in flight",
  () =>
    Effect.gen(function* () {
      const fake = yield* FakeApi;
      yield* fake.reply(`GET /rooms/${BOARD}/board`, boardListing([]));
      yield* boards.open(BOARD, all);
      const reply = yield* Deferred.make<ReturnType<typeof boardDetail>>();
      const entered = yield* Deferred.make<void>();
      yield* fake.route(`POST /rooms/${BOARD}/posts`, () =>
        Effect.andThen(Deferred.succeed(entered, undefined), Deferred.await(reply)),
      );

      const create = yield* Effect.forkChild(
        boards.createPost(BOARD, {
          name: "New",
          status: "planned",
          ownerId: null,
          tags: [],
          brief: "",
          clientId: "0192a3b4-0000-7000-8000-00000000c1af",
        }),
      );

      yield* Deferred.await(entered);
      mutations.applyEvents(
        [{ seq: 1, topic: "room:900", type: "thread.created", data: boardThread(1) }],
        0,
      );
      // The post goes, then enough other removals that its own is no longer remembered.
      mutations.applyEvents(
        [1, ...Array.from({ length: MAX_REMOVED_THREADS }, (_, index) => 20_000 + index)].map(
          (threadId, index) => ({
            seq: index + 2,
            topic: "room:900",
            type: "thread.removed" as const,
            data: { threadId, roomId: BOARD },
          }),
        ),
        0,
      );
      expect(store.getState().removedThreads[1]).toBeUndefined();
      yield* Deferred.succeed(reply, boardDetail());
      yield* Fiber.join(create);
      expect(store.getState().threads[1]).toBeUndefined();
      expect(boardPostIds(store.getState(), BOARD)).toEqual([]);
    }).pipe(Effect.provide(FakeApi.layerClient)),
);

it.effect("keeps a step the agent reported while a save was in flight", () =>
  Effect.gen(function* () {
    const fake = yield* FakeApi;
    mutations.loadThreadDetail(boardDetail(boardThread(1, "planned")), 0);
    const reply = yield* Deferred.make<ReturnType<typeof boardDetail>>();
    const entered = yield* Deferred.make<void>();
    yield* fake.route("PATCH /threads/1/work", () =>
      Effect.andThen(Deferred.succeed(entered, undefined), Deferred.await(reply)),
    );

    const step = {
      id: 5,
      messageId: null,
      threadId: 1,
      name: "Write the fix",
      status: "running",
      inputSummary: null,
      outputSummary: null,
      durationMs: null,
      position: 0,
      createdAt: "2026-10-07T10:00:00Z",
      updatedAt: "2026-10-07T10:00:00Z",
    } as const;

    const save = yield* Effect.forkChild(boards.update(1, { status: "in_progress" }));
    yield* Deferred.await(entered);
    mutations.applyEvents(
      [
        {
          seq: 1,
          topic: "thread:1",
          type: "agent.steps",
          data: {
            roomId: BOARD,
            messageId: null,
            threadId: 1,
            steps: [{ ...step, status: "done", updatedAt: "2026-10-07T10:03:00Z" }],
          },
        },
      ],
      0,
    );
    // The reply was read before the step finished.
    const answered = boardDetail(boardThread(1, "in_progress"));

    if (answered.work !== null) answered.work = { ...answered.work, steps: [step] };

    yield* Deferred.succeed(reply, answered);
    yield* Fiber.join(save);
    expect(store.getState().threadPanes[1]?.workFacts?.status).toBe("in_progress");
    expect(store.getState().threadPanes[1]?.work?.steps.map((s) => s.status)).toEqual(["done"]);
  }).pipe(Effect.provide(FakeApi.layerClient)),
);

it.effect("keeps the live message count when an older work detail lands", () =>
  Effect.gen(function* () {
    const fake = yield* FakeApi;
    mutations.loadThreadDetail(boardDetail(), 0);
    const waiting = yield* Deferred.make<ReturnType<typeof boardDetail>>();
    const entered = yield* Deferred.make<void>();
    yield* fake.route("GET /threads/1", () =>
      Effect.andThen(Deferred.succeed(entered, undefined), Deferred.await(waiting)),
    );
    const refresh = yield* Effect.forkChild(refreshWorkPane(1));
    yield* Deferred.await(entered);
    const replied = boardThread(1);

    if (replied.work !== null) replied.work.messageCount = 7;

    mutations.applyEvents(
      [{ seq: 1, topic: "room:900", type: "thread.updated", data: replied }],
      0,
    );
    yield* Deferred.succeed(waiting, boardDetail());
    yield* Fiber.join(refresh);
    expect(store.getState().threads[1]?.work?.messageCount).toBe(7);
  }).pipe(Effect.provide(FakeApi.layerClient)),
);

it.effect("keeps a newer held work revision when an older board snapshot lands", () =>
  Effect.gen(function* () {
    const fake = yield* FakeApi;
    mutations.loadThreadDetail(boardDetail(boardThread(1, "done", 7, ["api"], 2)), 0);
    yield* fake.reply(
      `GET /rooms/${BOARD}/board`,
      boardListing([boardThread(1, "planned", 7, ["api"], 1)]),
    );
    yield* boards.open(BOARD, all);

    expect(store.getState().threads[1]?.work).toMatchObject({
      status: "done",
      updatedAt: "2026-10-07T10:00:00.002000Z",
    });
    expect(boardPostIds(store.getState(), BOARD)).toEqual([1]);
  }).pipe(Effect.provide(FakeApi.layerClient)),
);
