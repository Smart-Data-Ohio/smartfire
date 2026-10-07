import { afterEach, describe, expect, it } from "@effect/vitest";
import { Deferred, Effect, Fiber } from "effect";
import { Forbidden, ServerError } from "../api/errors.ts";
import { FakeApi, userFixture } from "../api/testing.ts";
import { threadFixture } from "../features/threads/test-fixtures.ts";
import {
  agentFixture,
  factsFixture,
  linkFixture,
  rowFixture,
  threadDetailFixture,
  workDetailFixture,
} from "../features/work/test-fixtures.ts";
import type { WorkFacts } from "../gen/WorkFacts.ts";
import { mutations, store } from "../store/store.ts";
import { workDetailStale, workListOf } from "../store/work.ts";
import * as work from "./work-actions.ts";

const THREAD = 7;

const refuse = () => Effect.fail(new Forbidden({ message: "Not allowed" }));

const factsOf = (): WorkFacts | null | undefined => store.getState().threads[THREAD]?.work;

/** The pane holds thread 7, tracked with `facts`. */
function seed(facts: WorkFacts | null = factsFixture()): void {
  mutations.reset();
  mutations.loadThreadDetail(
    threadDetailFixture(THREAD, facts, facts === null ? null : workDetailFixture()),
  );
}

function liveStatus(status: WorkFacts["status"]): void {
  mutations.applyEvents(
    [
      {
        seq: 1,
        topic: `thread:${THREAD}`,
        type: "thread.updated",
        data: threadFixture(THREAD, {
          work: factsFixture({
            status,
            updatedAt:
              status === "done" ? "2026-10-06T09:20:00.000000Z" : "2026-10-06T09:10:00.000000Z",
          }),
        }),
      },
    ],
    Date.now(),
  );
}

describe("work actions", () => {
  afterEach(() => mutations.reset());

  it.effect("show a status change at once, then land the server's detail", () =>
    Effect.gen(function* () {
      seed();

      const fake = yield* FakeApi;
      let during: WorkFacts | null | undefined;
      let writes = 0;

      yield* fake.route(`PATCH /threads/${THREAD}/work`, () => {
        during = factsOf();
        writes = store.getState().work.writes[THREAD] ?? 0;

        return Effect.succeed(
          threadDetailFixture(
            THREAD,
            factsFixture({ status: "done", updatedAt: "2026-10-06T09:20:00.000000Z" }),
            workDetailFixture(),
          ),
        );
      });

      yield* work.setStatus(THREAD, "done");

      expect(during?.status).toBe("done");
      expect(writes).toBe(1);
      expect(factsOf()?.status).toBe("done");
      expect(store.getState().work.writes[THREAD]).toBeUndefined();
      expect(workDetailStale(store.getState(), THREAD)).toBe(false);

      const [request] = yield* fake.requests;

      expect(request?.body).toEqual({ status: "done" });
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("put the facts back when the server refuses, unless an event moved them", () =>
    Effect.gen(function* () {
      seed();

      const fake = yield* FakeApi;

      yield* fake.route(`PATCH /threads/${THREAD}/work`, refuse);

      const failure = yield* Effect.flip(work.setStatus(THREAD, "blocked"));

      expect(failure.message).toBe("Not allowed");
      expect(factsOf()?.status).toBe("in_progress");

      const gate = yield* Deferred.make<void>();

      yield* fake.route(`PATCH /threads/${THREAD}/work`, () =>
        Deferred.await(gate).pipe(Effect.andThen(refuse())),
      );

      const fiber = yield* Effect.forkChild(work.setStatus(THREAD, "blocked"));

      yield* Effect.yieldNow;
      mutations.upsertThread(
        threadFixture(THREAD, {
          work: factsFixture({ status: "planned", updatedAt: "2026-10-06T09:15:00.000000Z" }),
        }),
      );
      yield* Deferred.succeed(gate, undefined);
      yield* Effect.flip(Fiber.join(fiber));

      expect(factsOf()?.status).toBe("planned");
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("stop tracking owned work with the owner cleared, and start untracked work", () =>
    Effect.gen(function* () {
      seed();

      const fake = yield* FakeApi;

      yield* fake.reply(`PATCH /threads/${THREAD}/work`, threadDetailFixture(THREAD, null, null));
      yield* work.setStatus(THREAD, null);

      expect(factsOf()).toBeNull();
      expect(store.getState().work.details[THREAD]).toBeUndefined();

      let during: WorkFacts | null | undefined;

      yield* fake.route(`PATCH /threads/${THREAD}/work`, () => {
        during = factsOf();

        return Effect.succeed(
          threadDetailFixture(
            THREAD,
            factsFixture({ status: "planned", owner: null }),
            workDetailFixture(),
          ),
        );
      });
      yield* work.setStatus(THREAD, "planned");

      const requests = yield* fake.requests;

      expect(requests.map((request) => request.body)).toEqual([
        { status: null, ownerId: null },
        { status: "planned" },
      ]);
      expect(during).toMatchObject({ status: "planned", owner: null, links: [] });
      expect(store.getState().work.details[THREAD]).toBeDefined();
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("assign at once, and record a result and a handoff from the server's reply", () =>
    Effect.gen(function* () {
      seed();

      const fake = yield* FakeApi;
      let during: WorkFacts | null | undefined;

      yield* fake.route(`PATCH /threads/${THREAD}/work`, () => {
        during = factsOf();

        return Effect.succeed(
          threadDetailFixture(THREAD, factsFixture({ owner: userFixture(3) }), workDetailFixture()),
        );
      });
      yield* work.assign(THREAD, 3);

      expect(during?.owner?.id).toBe(3);

      yield* fake.reply(
        `PATCH /threads/${THREAD}/work`,
        threadDetailFixture(
          THREAD,
          factsFixture({ resultUpdatedAt: "2026-10-06T10:00:00.000Z" }),
          workDetailFixture({ resultMarkdown: "Done", resultHtml: "<p>Done</p>" }),
        ),
      );
      yield* work.saveResult(THREAD, "Done");

      expect(store.getState().work.details[THREAD]?.resultHtml).toBe("<p>Done</p>");

      yield* fake.reply(
        `POST /threads/${THREAD}/work/handoff`,
        threadDetailFixture(THREAD, factsFixture({ owner: agentFixture() }), workDetailFixture()),
      );
      yield* work.handOff(THREAD, {
        receiverAgentId: 9,
        summary: "Over to you",
        links: [],
        openQuestions: [],
      });

      expect(factsOf()?.owner?.id).toBe(9);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  for (const observed of ["previous owner", "tied deletion", "newer deletion"] as const) {
    it.effect(`reconciles a handoff reply after a held GET observes ${observed}`, () =>
      Effect.gen(function* () {
        seed();

        const fake = yield* FakeApi;
        const handoffStarted = yield* Deferred.make<void>();
        const readStarted = yield* Deferred.make<void>();
        const handoffGate = yield* Deferred.make<void>();
        const readGate = yield* Deferred.make<void>();

        const assigned = factsFixture({
          owner: agentFixture(),
          updatedAt: "2026-10-06T09:01:00.000000Z",
        });

        const snapshot =
          observed === "previous owner"
            ? factsFixture()
            : factsFixture({
                owner: null,
                ownerActive: false,
                updatedAt:
                  observed === "tied deletion" ? assigned.updatedAt : "2026-10-06T09:02:00.000000Z",
              });

        yield* fake.route(`POST /threads/${THREAD}/work/handoff`, () =>
          Deferred.succeed(handoffStarted, undefined).pipe(
            Effect.andThen(Deferred.await(handoffGate)),
            Effect.as(threadDetailFixture(THREAD, assigned, workDetailFixture())),
          ),
        );
        yield* fake.route(`GET /threads/${THREAD}`, () =>
          Deferred.succeed(readStarted, undefined).pipe(
            Effect.andThen(Deferred.await(readGate)),
            Effect.as(threadDetailFixture(THREAD, snapshot, workDetailFixture())),
          ),
        );

        const handingOff = yield* Effect.forkChild(
          work.handOff(THREAD, {
            receiverAgentId: 9,
            summary: "Over to you",
            links: [],
            openQuestions: [],
          }),
        );

        yield* Deferred.await(handoffStarted);

        const reading = yield* Effect.forkChild(work.refresh(THREAD));

        yield* Deferred.await(readStarted);
        yield* Deferred.succeed(readGate, undefined);
        yield* Fiber.join(reading);

        expect(store.getState().work.overlays[THREAD]).toBeUndefined();
        expect(factsOf()?.owner?.id ?? null).toBe(observed === "previous owner" ? 2 : null);

        yield* Deferred.succeed(handoffGate, undefined);
        yield* Fiber.join(handingOff);

        expect(factsOf()?.owner?.id ?? null).toBe(observed === "previous owner" ? 9 : null);
        expect(factsOf()?.ownerActive).toBe(observed === "previous owner");
        expect(factsOf()?.updatedAt).toBe(
          observed === "newer deletion" ? snapshot.updatedAt : assigned.updatedAt,
        );
        expect(store.getState().work.writes[THREAD]).toBeUndefined();
      }).pipe(Effect.provide(FakeApi.layerClient)),
    );
  }

  it.effect("refetch once per burst when live facts move past the held ones", () =>
    Effect.gen(function* () {
      seed();
      mutations.upsertThread(
        threadFixture(THREAD, {
          work: factsFixture({ status: "blocked", updatedAt: "2026-10-06T09:10:00.000000Z" }),
        }),
      );

      expect(workDetailStale(store.getState(), THREAD)).toBe(true);

      const fake = yield* FakeApi;
      const gate = yield* Deferred.make<void>();

      yield* fake.route(`GET /threads/${THREAD}`, () =>
        Deferred.await(gate).pipe(
          Effect.as(
            threadDetailFixture(
              THREAD,
              factsFixture({ status: "blocked", updatedAt: "2026-10-06T09:10:00.000000Z" }),
              workDetailFixture(),
            ),
          ),
        ),
      );

      const first = yield* Effect.forkChild(work.refresh(THREAD));

      yield* Effect.yieldNow;
      yield* work.refresh(THREAD);
      yield* Deferred.succeed(gate, undefined);
      yield* Fiber.join(first);

      expect((yield* fake.requests).length).toBe(1);
      expect(workDetailStale(store.getState(), THREAD)).toBe(false);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect(
    "stop a stale detail refresh with an error when its held revision does not advance",
    () =>
      Effect.gen(function* () {
        seed();
        liveStatus("blocked");

        const fake = yield* FakeApi;
        let requests = 0;

        yield* fake.route(`GET /threads/${THREAD}`, () => {
          requests += 1;

          return requests === 1
            ? Effect.succeed(threadDetailFixture(THREAD, factsFixture(), workDetailFixture()))
            : Effect.fail(new ServerError({ status: 503, message: "Unexpected repeated request" }));
        });
        yield* work.refresh(THREAD);

        expect(requests).toBe(1);
        expect(store.getState().threadPanes[THREAD]?.error).toBe(
          "The work detail could not catch up to its latest revision",
        );
        expect(factsOf()).toMatchObject({
          status: "blocked",
          updatedAt: "2026-10-06T09:10:00.000000Z",
        });
        expect(workDetailStale(store.getState(), THREAD)).toBe(true);
      }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect(
    "keep newer links without rejecting complete detail from a held equal-revision GET",
    () =>
      Effect.gen(function* () {
        const original = factsFixture({ links: [linkFixture(1)] });
        const latest = factsFixture({ links: [linkFixture(1, { pullRequestState: "merged" })] });

        seed(original);

        const fake = yield* FakeApi;
        const started = yield* Deferred.make<void>();
        const gate = yield* Deferred.make<void>();

        yield* fake.route(`GET /threads/${THREAD}`, () =>
          Deferred.succeed(started, undefined).pipe(
            Effect.andThen(Deferred.await(gate)),
            Effect.as(
              threadDetailFixture(
                THREAD,
                original,
                workDetailFixture({ resultMarkdown: "Fetched result" }),
              ),
            ),
          ),
        );

        const loading = yield* Effect.forkChild(work.refresh(THREAD));

        yield* Deferred.await(started);
        mutations.applyEvents(
          [
            {
              seq: 1,
              topic: `thread:${THREAD}`,
              type: "thread.updated",
              data: threadFixture(THREAD, { work: latest }),
            },
          ],
          Date.now(),
        );
        yield* Deferred.succeed(gate, undefined);
        yield* Fiber.join(loading);

        expect(factsOf()?.links).toEqual(latest.links);
        expect(store.getState().work.details[THREAD]?.resultMarkdown).toBe("Fetched result");
        expect(store.getState().threadPanes[THREAD]?.error ?? null).toBeNull();
        expect(workDetailStale(store.getState(), THREAD)).toBe(false);
        expect((yield* fake.requests).length).toBe(1);
      }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("load a filter's list, keep it on a failed reload, and drop an older reply", () =>
    Effect.gen(function* () {
      mutations.reset();

      const fake = yield* FakeApi;

      yield* fake.reply("GET /work", { threads: [rowFixture(1)], users: [userFixture(2)] });
      yield* work.loadList("open");

      expect(workListOf(store.getState(), "open").rows.map((row) => row.thread.id)).toEqual([1]);
      expect(store.getState().users[2]?.name).toBe("User 2");

      yield* fake.route("GET /work", () =>
        Effect.fail(new ServerError({ status: 500, message: "Down" })),
      );
      yield* work.loadList("open");

      expect(workListOf(store.getState(), "open")).toMatchObject({
        status: "ready",
        error: "Down",
      });

      const [request] = (yield* fake.requests).slice(-1);

      expect(request?.query).toEqual({ state: "open" });

      const gate = yield* Deferred.make<void>();

      yield* fake.route("GET /work", () =>
        Deferred.await(gate).pipe(Effect.as({ threads: [rowFixture(5)], users: [] })),
      );

      const stale = yield* Effect.forkChild(work.loadList("done"));

      yield* Effect.yieldNow;
      mutations.setWorkListLoading("done");
      yield* Deferred.succeed(gate, undefined);
      yield* Fiber.join(stale);

      expect(workListOf(store.getState(), "done").rows).toEqual([]);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("merge list and detail responses into the same work key", () =>
    Effect.gen(function* () {
      seed(factsFixture({ status: "blocked", updatedAt: "2026-10-06T09:10:00.000000Z" }));

      const fake = yield* FakeApi;
      const done = rowFixture(THREAD);

      yield* fake.reply("GET /work", {
        threads: [
          {
            ...done,
            thread: {
              ...done.thread,
              work: factsFixture({ status: "done", updatedAt: "2026-10-06T09:20:00.000000Z" }),
            },
          },
        ],
        users: [],
      });
      yield* work.loadList("done");

      expect(factsOf()?.status).toBe("done");
      expect(workListOf(store.getState(), "done").rows[0]?.thread.work?.status).toBe("done");
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("refetch incomplete work detail after holding back a delayed refresh", () =>
    Effect.gen(function* () {
      seed();
      liveStatus("blocked");

      const fake = yield* FakeApi;
      const started = yield* Deferred.make<void>();
      const gate = yield* Deferred.make<void>();

      yield* fake.route(`GET /threads/${THREAD}`, () =>
        Deferred.succeed(started, undefined).pipe(
          Effect.andThen(Deferred.await(gate)),
          Effect.as(
            threadDetailFixture(
              THREAD,
              factsFixture({ status: "blocked", updatedAt: "2026-10-06T09:10:00.000000Z" }),
              workDetailFixture(),
            ),
          ),
        ),
      );

      const loading = yield* Effect.forkChild(work.refresh(THREAD));

      yield* Deferred.await(started);
      liveStatus("done");
      yield* fake.reply(
        `GET /threads/${THREAD}`,
        threadDetailFixture(
          THREAD,
          factsFixture({ status: "done", updatedAt: "2026-10-06T09:20:00.000000Z" }),
          workDetailFixture(),
        ),
      );
      yield* Deferred.succeed(gate, undefined);
      yield* Fiber.join(loading);

      expect(factsOf()?.status).toBe("done");
      expect((yield* fake.requests).length).toBe(2);
      expect(workDetailStale(store.getState(), THREAD)).toBe(false);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("keep a successful write after its echo and a late older refresh", () =>
    Effect.gen(function* () {
      seed(factsFixture({ status: "blocked", updatedAt: "2026-10-06T09:10:00.000000Z" }));

      const fake = yield* FakeApi;
      const started = yield* Deferred.make<void>();
      const gate = yield* Deferred.make<void>();

      const done = threadDetailFixture(
        THREAD,
        factsFixture({ status: "done", updatedAt: "2026-10-06T09:20:00.000000Z" }),
        workDetailFixture(),
      );

      yield* fake.route(`GET /threads/${THREAD}`, () =>
        Deferred.succeed(started, undefined).pipe(
          Effect.andThen(Deferred.await(gate)),
          Effect.as(
            threadDetailFixture(
              THREAD,
              factsFixture({ status: "blocked", updatedAt: "2026-10-06T09:10:00.000000Z" }),
              workDetailFixture(),
            ),
          ),
        ),
      );

      const loading = yield* Effect.forkChild(work.refresh(THREAD));

      yield* Deferred.await(started);
      yield* fake.route(`PATCH /threads/${THREAD}/work`, () => {
        liveStatus("done");

        return Effect.succeed(done);
      });
      yield* work.setStatus(THREAD, "done");
      yield* fake.reply(`GET /threads/${THREAD}`, done);
      yield* Deferred.succeed(gate, undefined);
      yield* Fiber.join(loading);

      expect(factsOf()?.status).toBe("done");
      expect(workDetailStale(store.getState(), THREAD)).toBe(false);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("keep newer live facts after a delayed write success", () =>
    Effect.gen(function* () {
      seed();

      const fake = yield* FakeApi;
      const started = yield* Deferred.make<void>();
      const gate = yield* Deferred.make<void>();

      yield* fake.route(`PATCH /threads/${THREAD}/work`, () =>
        Deferred.succeed(started, undefined).pipe(
          Effect.andThen(Deferred.await(gate)),
          Effect.as(
            threadDetailFixture(
              THREAD,
              factsFixture({ status: "blocked", updatedAt: "2026-10-06T09:10:00.000000Z" }),
              workDetailFixture(),
            ),
          ),
        ),
      );

      const writing = yield* Effect.forkChild(work.setStatus(THREAD, "blocked"));

      yield* Deferred.await(started);
      liveStatus("done");
      yield* Deferred.succeed(gate, undefined);
      yield* Fiber.join(writing);

      expect(factsOf()?.status).toBe("done");
      expect(workDetailStale(store.getState(), THREAD)).toBe(true);
      expect(store.getState().work.writes[THREAD]).toBeUndefined();
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("land a write's reply when only its own echo and a new reply arrive meanwhile", () =>
    Effect.gen(function* () {
      seed();

      const fake = yield* FakeApi;
      const started = yield* Deferred.make<void>();
      const gate = yield* Deferred.make<void>();
      const blocked = factsFixture({ status: "blocked", updatedAt: "2026-10-06T09:10:00.000000Z" });

      yield* fake.route(`PATCH /threads/${THREAD}/work`, () =>
        Deferred.succeed(started, undefined).pipe(
          Effect.andThen(Deferred.await(gate)),
          Effect.as(threadDetailFixture(THREAD, blocked, workDetailFixture())),
        ),
      );

      const writing = yield* Effect.forkChild(work.setStatus(THREAD, "blocked"));

      yield* Deferred.await(started);

      mutations.applyEvents(
        [1, 2].map((seq) => ({
          seq,
          topic: `thread:${THREAD}`,
          type: "thread.updated" as const,
          data: threadFixture(THREAD, { work: blocked, replyCount: seq }),
        })),
        Date.now(),
      );

      yield* Deferred.succeed(gate, undefined);
      yield* Fiber.join(writing);

      expect(factsOf()?.status).toBe("blocked");
      expect(workDetailStale(store.getState(), THREAD)).toBe(false);
      expect((yield* fake.requests).length).toBe(1);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );
});
