import { afterEach, describe, expect, it } from "@effect/vitest";
import { Deferred, Effect, Fiber } from "effect";
import { Forbidden, ServerError } from "../api/errors.ts";
import { FakeApi, userFixture } from "../api/testing.ts";
import { threadFixture } from "../features/threads/test-fixtures.ts";
import {
  agentFixture,
  factsFixture,
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
          threadDetailFixture(THREAD, factsFixture({ status: "done" }), workDetailFixture()),
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
      mutations.upsertThread(threadFixture(THREAD, { work: factsFixture({ status: "planned" }) }));
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

  it.effect("refetch once per burst when live facts move past the held ones", () =>
    Effect.gen(function* () {
      seed();
      mutations.upsertThread(threadFixture(THREAD, { work: factsFixture({ status: "blocked" }) }));

      expect(workDetailStale(store.getState(), THREAD)).toBe(true);

      const fake = yield* FakeApi;
      const gate = yield* Deferred.make<void>();

      yield* fake.route(`GET /threads/${THREAD}`, () =>
        Deferred.await(gate).pipe(
          Effect.as(
            threadDetailFixture(THREAD, factsFixture({ status: "blocked" }), workDetailFixture()),
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
});
