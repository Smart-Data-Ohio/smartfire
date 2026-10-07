import { afterEach, describe, expect, it } from "@effect/vitest";
import { Deferred, Effect, Fiber } from "effect";
import { Forbidden, ServerError } from "../api/errors.ts";
import { FakeApi, pageFixture, rowVersionFixture, userFixture } from "../api/testing.ts";
import { messageFixture, threadFixture } from "../features/threads/test-fixtures.ts";
import {
  factsFixture,
  threadDetailFixture,
  workDetailFixture,
} from "../features/work/test-fixtures.ts";
import type { AgentApproval } from "../gen/AgentApproval.ts";
import type { ThreadDetail } from "../gen/ThreadDetail.ts";
import type { WorkFacts } from "../gen/WorkFacts.ts";
import { approvalListKey, approvalListOf } from "../store/approvals.ts";
import { mutations, store } from "../store/store.ts";
import { workDetailStale } from "../store/work.ts";
import * as approvals from "./approval-actions.ts";
import * as threads from "./thread-actions.ts";
import * as work from "./work-actions.ts";

const THREAD = 7;

const NOW = Date.UTC(2026, 9, 6, 9, 15);

function facts(offset: number, change: Partial<WorkFacts> = {}): WorkFacts {
  return factsFixture({ updatedAt: rowVersionFixture(NOW + offset), ...change });
}

function detail(incoming: WorkFacts): ThreadDetail {
  return threadDetailFixture(THREAD, incoming, workDetailFixture());
}

function seed(): void {
  mutations.loadThreadDetail(detail(facts(0, { status: "planned" })));
}

describe("S4 revisions through held network responses", () => {
  afterEach(() => mutations.reset());

  for (const newestRequest of ["first", "second"] as const) {
    it.effect(
      `keep the newest server work when the ${newestRequest} overlapping request lands first`,
      () =>
        Effect.gen(function* () {
          seed();

          const fake = yield* FakeApi;
          const firstStarted = yield* Deferred.make<void>();
          const secondStarted = yield* Deferred.make<void>();
          const firstGate = yield* Deferred.make<void>();
          const secondGate = yield* Deferred.make<void>();
          const newest = facts(2, { status: "done" });
          const oldest = facts(1, { status: "blocked" });
          let calls = 0;

          yield* fake.reply(`GET /threads/${THREAD}/messages`, pageFixture([]));
          yield* fake.route(`GET /threads/${THREAD}`, () => {
            calls += 1;

            if (calls > 2) {
              return Effect.succeed(detail(oldest));
            }

            const first = calls === 1;
            const incoming = first === (newestRequest === "first") ? newest : oldest;

            return Deferred.succeed(first ? firstStarted : secondStarted, undefined).pipe(
              Effect.andThen(Deferred.await(first ? firstGate : secondGate)),
              Effect.as(detail(incoming)),
            );
          });

          const first = yield* Effect.forkChild(threads.reload(THREAD));

          yield* Deferred.await(firstStarted);

          const second = yield* Effect.forkChild(threads.reload(THREAD));

          yield* Deferred.await(secondStarted);
          yield* Deferred.succeed(newestRequest === "first" ? firstGate : secondGate, undefined);
          yield* Fiber.join(newestRequest === "first" ? first : second);
          yield* Deferred.succeed(newestRequest === "first" ? secondGate : firstGate, undefined);
          yield* Fiber.join(newestRequest === "first" ? second : first);

          expect(store.getState().threads[THREAD]?.work).toEqual(newest);
          expect(
            (yield* fake.requests).filter((request) => request.path === `/threads/${THREAD}`),
          ).toHaveLength(2);
        }).pipe(Effect.provide(FakeApi.layerClient)),
    );
  }

  it.effect(
    "keep an optimistic status through an older GET started during the write and after its reply",
    () =>
      Effect.gen(function* () {
        seed();

        const fake = yield* FakeApi;
        const writeStarted = yield* Deferred.make<void>();
        const readStarted = yield* Deferred.make<void>();
        const writeGate = yield* Deferred.make<void>();
        const readGate = yield* Deferred.make<void>();
        const confirmed = facts(2, { status: "done" });

        yield* fake.reply(`GET /threads/${THREAD}/messages`, pageFixture([]));
        yield* fake.route(`PATCH /threads/${THREAD}/work`, () =>
          Deferred.succeed(writeStarted, undefined).pipe(
            Effect.andThen(Deferred.await(writeGate)),
            Effect.as(detail(confirmed)),
          ),
        );
        yield* fake.route(`GET /threads/${THREAD}`, () =>
          Deferred.succeed(readStarted, undefined).pipe(
            Effect.andThen(Deferred.await(readGate)),
            Effect.as(detail(facts(0, { status: "planned", ownerActive: false }))),
          ),
        );

        const writing = yield* Effect.forkChild(work.setStatus(THREAD, "done"));

        yield* Deferred.await(writeStarted);

        const reading = yield* Effect.forkChild(threads.reload(THREAD));

        yield* Deferred.await(readStarted);
        yield* Deferred.succeed(readGate, undefined);
        yield* Fiber.join(reading);

        expect(store.getState().threads[THREAD]?.work?.status).toBe("done");
        expect(store.getState().threads[THREAD]?.work?.ownerActive).toBe(false);

        yield* Deferred.succeed(writeGate, undefined);
        yield* Fiber.join(writing);

        expect(store.getState().threads[THREAD]?.work).toEqual(confirmed);
        expect(store.getState().work.writes[THREAD]).toBeUndefined();
      }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("keep a successful work reply after its echo and a late older refresh", () =>
    Effect.gen(function* () {
      seed();

      const fake = yield* FakeApi;
      const readStarted = yield* Deferred.make<void>();
      const readGate = yield* Deferred.make<void>();
      const confirmed = facts(2, { status: "done" });

      yield* fake.reply(`GET /threads/${THREAD}/messages`, pageFixture([]));
      yield* fake.route(`GET /threads/${THREAD}`, () =>
        Deferred.succeed(readStarted, undefined).pipe(
          Effect.andThen(Deferred.await(readGate)),
          Effect.as(detail(facts(0, { status: "planned" }))),
        ),
      );
      yield* fake.route(`PATCH /threads/${THREAD}/work`, () => {
        mutations.applyEvents(
          [
            {
              seq: 1,
              topic: `thread:${THREAD}`,
              type: "thread.updated",
              data: threadFixture(THREAD, { work: confirmed }),
            },
          ],
          NOW,
        );

        return Effect.succeed(detail(confirmed));
      });

      const reading = yield* Effect.forkChild(threads.reload(THREAD));

      yield* Deferred.await(readStarted);
      yield* work.setStatus(THREAD, "done");
      yield* Deferred.succeed(readGate, undefined);
      yield* Fiber.join(reading);

      expect(store.getState().threads[THREAD]?.work).toEqual(confirmed);
      expect(
        (yield* fake.requests).filter((request) => request.path === `/threads/${THREAD}`),
      ).toHaveLength(1);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("roll back a refused status while keeping eligibility learned from an older GET", () =>
    Effect.gen(function* () {
      seed();

      const fake = yield* FakeApi;
      const writeStarted = yield* Deferred.make<void>();
      const writeGate = yield* Deferred.make<void>();

      yield* fake.route(`PATCH /threads/${THREAD}/work`, () =>
        Deferred.succeed(writeStarted, undefined).pipe(
          Effect.andThen(Deferred.await(writeGate)),
          Effect.andThen(Effect.fail(new Forbidden({ message: "Status change refused" }))),
        ),
      );
      yield* fake.reply(
        `GET /threads/${THREAD}`,
        detail(facts(0, { status: "planned", ownerActive: false })),
      );

      const writing = yield* Effect.forkChild(work.setStatus(THREAD, "done"));

      yield* Deferred.await(writeStarted);
      yield* work.refresh(THREAD);

      expect(store.getState().threads[THREAD]?.work).toMatchObject({
        status: "done",
        ownerActive: false,
      });

      yield* Deferred.succeed(writeGate, undefined);

      const refused = yield* Effect.flip(Fiber.join(writing));

      expect(refused.message).toBe("Status change refused");
      expect(store.getState().threads[THREAD]?.work).toMatchObject({
        status: "planned",
        ownerActive: false,
        updatedAt: rowVersionFixture(NOW),
      });
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("keep newer eligibility when a held refresh returns the same work revision", () =>
    Effect.gen(function* () {
      seed();

      const fake = yield* FakeApi;
      const readStarted = yield* Deferred.make<void>();
      const readGate = yield* Deferred.make<void>();

      yield* fake.route(`GET /threads/${THREAD}`, () =>
        Deferred.succeed(readStarted, undefined).pipe(
          Effect.andThen(Deferred.await(readGate)),
          Effect.as(detail(facts(0, { status: "planned", ownerActive: true }))),
        ),
      );

      const reading = yield* Effect.forkChild(work.refresh(THREAD));

      yield* Deferred.await(readStarted);
      mutations.loadThreadDetail(detail(facts(0, { status: "planned", ownerActive: false })));
      yield* Deferred.succeed(readGate, undefined);
      yield* Fiber.join(reading);

      expect(store.getState().threads[THREAD]?.work).toMatchObject({
        status: "planned",
        ownerActive: false,
        updatedAt: rowVersionFixture(NOW),
      });
      expect((yield* fake.requests).length).toBe(1);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect(
    "keep retrying incomplete detail while confirmed revisions advance during each read",
    () =>
      Effect.gen(function* () {
        seed();

        const fake = yield* FakeApi;
        const confirmed = facts(8, { status: "done" });
        let calls = 0;

        mutations.upsertThread(threadFixture(THREAD, { work: facts(2, { status: "done" }) }));
        yield* fake.route(`GET /threads/${THREAD}`, () => {
          calls += 1;

          if (calls <= 6) {
            mutations.upsertThread(
              threadFixture(THREAD, { work: facts(calls + 2, { status: "done" }) }),
            );
          }

          return Effect.succeed(
            detail(calls <= 6 ? facts(calls + 1, { status: "planned" }) : confirmed),
          );
        });
        yield* work.refresh(THREAD);

        expect(calls).toBe(7);
        expect(store.getState().threads[THREAD]?.work).toEqual(confirmed);
        expect(workDetailStale(store.getState(), THREAD)).toBe(false);
      }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("surface a detail refresh error while keeping the newer server work", () =>
    Effect.gen(function* () {
      seed();

      const fake = yield* FakeApi;
      const confirmed = facts(2, { status: "done" });

      mutations.upsertThread(threadFixture(THREAD, { work: confirmed }));
      yield* fake.route(`GET /threads/${THREAD}`, () =>
        Effect.fail(new ServerError({ status: 503, message: "Detail unavailable" })),
      );
      yield* work.refresh(THREAD);

      expect(store.getState().threadPanes[THREAD]?.error).toBe("Detail unavailable");
      expect(store.getState().threads[THREAD]?.work).toEqual(confirmed);
      expect(workDetailStale(store.getState(), THREAD)).toBe(true);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("keep tracked work through an untracked GET captured during a remote ABA change", () =>
    Effect.gen(function* () {
      seed();

      const fake = yield* FakeApi;
      const readStarted = yield* Deferred.make<void>();
      const readGate = yield* Deferred.make<void>();

      yield* fake.reply(`GET /threads/${THREAD}/messages`, pageFixture([]));
      yield* fake.route(`GET /threads/${THREAD}`, () =>
        Deferred.succeed(readStarted, undefined).pipe(
          Effect.andThen(Deferred.await(readGate)),
          Effect.as(threadDetailFixture(THREAD, null, null)),
        ),
      );

      const reading = yield* Effect.forkChild(threads.reload(THREAD));

      yield* Deferred.await(readStarted);
      mutations.upsertThread(threadFixture(THREAD, { work: facts(2, { status: "planned" }) }));
      yield* Deferred.succeed(readGate, undefined);
      yield* Fiber.join(reading);

      expect(store.getState().threads[THREAD]?.work).toMatchObject({
        status: "planned",
        updatedAt: rowVersionFixture(NOW + 2),
      });
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect(
    "keep untracked work through a tracked GET captured during an inverse remote ABA change",
    () =>
      Effect.gen(function* () {
        mutations.loadThreadDetail(threadDetailFixture(THREAD, null, null));

        const fake = yield* FakeApi;
        const readStarted = yield* Deferred.make<void>();
        const readGate = yield* Deferred.make<void>();

        yield* fake.reply(`GET /threads/${THREAD}/messages`, pageFixture([]));
        yield* fake.route(`GET /threads/${THREAD}`, () =>
          Deferred.succeed(readStarted, undefined).pipe(
            Effect.andThen(Deferred.await(readGate)),
            Effect.as(detail(facts(1, { status: "planned" }))),
          ),
        );

        const reading = yield* Effect.forkChild(threads.reload(THREAD));

        yield* Deferred.await(readStarted);
        mutations.upsertThread(threadFixture(THREAD, { work: null }));
        yield* Deferred.succeed(readGate, undefined);
        yield* Fiber.join(reading);

        expect(store.getState().threads[THREAD]?.work).toBeNull();
        expect(store.getState().work.details[THREAD]).toBeUndefined();
      }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("keep tracked work when a held thread creation retry replies with untracked work", () =>
    Effect.gen(function* () {
      const fake = yield* FakeApi;
      const createStarted = yield* Deferred.make<void>();
      const createGate = yield* Deferred.make<void>();
      const confirmed = facts(2, { status: "planned" });

      yield* fake.route("POST /rooms/4/threads", () =>
        Deferred.succeed(createStarted, undefined).pipe(
          Effect.andThen(Deferred.await(createGate)),
          Effect.as({
            detail: threadDetailFixture(THREAD, null, null),
            message: messageFixture(100, { threadId: THREAD }),
          }),
        ),
      );

      const creating = yield* Effect.forkChild(threads.create(4, 70, "First reply"));

      yield* Deferred.await(createStarted);
      mutations.upsertThread(threadFixture(THREAD, { work: confirmed }));
      yield* Deferred.succeed(createGate, undefined);

      const threadId = yield* Fiber.join(creating);

      expect(threadId).toBe(THREAD);
      expect(store.getState().threads[THREAD]?.work).toEqual(confirmed);
      expect((yield* fake.requests).length).toBe(1);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("keep tracked work when a held thread update replies with untracked work", () =>
    Effect.gen(function* () {
      mutations.loadThreadDetail(threadDetailFixture(THREAD, null, null));

      const fake = yield* FakeApi;
      const updateStarted = yield* Deferred.make<void>();
      const updateGate = yield* Deferred.make<void>();
      const confirmed = facts(2, { status: "planned" });
      const reply = threadDetailFixture(THREAD, null, null);

      yield* fake.route(`PATCH /threads/${THREAD}`, () =>
        Deferred.succeed(updateStarted, undefined).pipe(
          Effect.andThen(Deferred.await(updateGate)),
          Effect.as({ ...reply, thread: { ...reply.thread, name: "Renamed" } }),
        ),
      );

      const updating = yield* Effect.forkChild(
        threads.update(THREAD, { name: "Renamed", status: null }),
      );

      yield* Deferred.await(updateStarted);
      mutations.upsertThread(threadFixture(THREAD, { work: confirmed }));
      yield* Deferred.succeed(updateGate, undefined);
      yield* Fiber.join(updating);

      expect(store.getState().threads[THREAD]).toMatchObject({ name: "Renamed", work: confirmed });
      expect((yield* fake.requests).length).toBe(1);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("keep a new tracking overlay when an earlier GET captured transient tracked work", () =>
    Effect.gen(function* () {
      mutations.loadThreadDetail(threadDetailFixture(THREAD, null, null));

      const fake = yield* FakeApi;
      const readStarted = yield* Deferred.make<void>();
      const readGate = yield* Deferred.make<void>();
      const writeStarted = yield* Deferred.make<void>();
      const writeGate = yield* Deferred.make<void>();
      const confirmed = facts(2, { status: "planned", owner: null, ownerActive: false });

      yield* fake.reply(`GET /threads/${THREAD}/messages`, pageFixture([]));
      yield* fake.route(`GET /threads/${THREAD}`, () =>
        Deferred.succeed(readStarted, undefined).pipe(
          Effect.andThen(Deferred.await(readGate)),
          Effect.as(detail(facts(1, { status: "blocked" }))),
        ),
      );
      yield* fake.route(`PATCH /threads/${THREAD}/work`, () =>
        Deferred.succeed(writeStarted, undefined).pipe(
          Effect.andThen(Deferred.await(writeGate)),
          Effect.as(detail(confirmed)),
        ),
      );

      const reading = yield* Effect.forkChild(threads.reload(THREAD));

      yield* Deferred.await(readStarted);

      const writing = yield* Effect.forkChild(work.setStatus(THREAD, "planned"));

      yield* Deferred.await(writeStarted);
      yield* Deferred.succeed(readGate, undefined);
      yield* Fiber.join(reading);

      expect(store.getState().threads[THREAD]?.work).toMatchObject({
        status: "planned",
        owner: null,
      });

      yield* Deferred.succeed(writeGate, undefined);
      yield* Fiber.join(writing);

      expect(store.getState().threads[THREAD]?.work).toEqual(confirmed);
      expect(store.getState().work.writes[THREAD]).toBeUndefined();
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect(
    "keep equal-event eligibility through a held ABA refresh while filling revisioned fields",
    () =>
      Effect.gen(function* () {
        mutations.loadThreadDetail(detail(facts(0, { ownerActive: false })));

        const fake = yield* FakeApi;
        const readStarted = yield* Deferred.make<void>();
        const readGate = yield* Deferred.make<void>();

        yield* fake.route(`GET /threads/${THREAD}`, () =>
          Deferred.succeed(readStarted, undefined).pipe(
            Effect.andThen(Deferred.await(readGate)),
            Effect.as(
              detail(facts(0, { ownerActive: true, runUrl: "https://example.test/run/held" })),
            ),
          ),
        );

        const reading = yield* Effect.forkChild(work.refresh(THREAD));

        yield* Deferred.await(readStarted);
        mutations.upsertThread(threadFixture(THREAD, { work: facts(0, { ownerActive: false }) }));
        yield* Deferred.succeed(readGate, undefined);
        yield* Fiber.join(reading);

        expect(store.getState().threads[THREAD]?.work).toMatchObject({
          ownerActive: false,
          runUrl: "https://example.test/run/held",
          updatedAt: rowVersionFixture(NOW),
        });
        expect((yield* fake.requests).length).toBe(1);
      }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("keep an optimistic owner's eligibility separate from the previous owner's GET", () =>
    Effect.gen(function* () {
      seed();

      const fake = yield* FakeApi;
      const writeStarted = yield* Deferred.make<void>();
      const writeGate = yield* Deferred.make<void>();
      const confirmed = facts(2, { status: "planned", owner: userFixture(3), ownerActive: true });

      yield* fake.route(`PATCH /threads/${THREAD}/work`, () =>
        Deferred.succeed(writeStarted, undefined).pipe(
          Effect.andThen(Deferred.await(writeGate)),
          Effect.as(detail(confirmed)),
        ),
      );
      yield* fake.reply(
        `GET /threads/${THREAD}`,
        detail(facts(0, { status: "planned", ownerActive: false })),
      );

      const writing = yield* Effect.forkChild(work.assign(THREAD, 3));

      yield* Deferred.await(writeStarted);
      yield* work.refresh(THREAD);

      expect(store.getState().threads[THREAD]?.work).toMatchObject({
        owner: { id: 3 },
        ownerActive: true,
      });

      yield* Deferred.succeed(writeGate, undefined);
      yield* Fiber.join(writing);

      expect(store.getState().threads[THREAD]?.work).toEqual(confirmed);
      expect(store.getState().work.writes[THREAD]).toBeUndefined();
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect(
    "keep an Approved row after an equal-membership event and a late empty ABA reload",
    () =>
      Effect.gen(function* () {
        const fake = yield* FakeApi;
        const readStarted = yield* Deferred.make<void>();
        const readGate = yield* Deferred.make<void>();

        const approved: AgentApproval = {
          id: 1,
          agentId: 9,
          agentUserId: 9,
          roomId: 4,
          roomName: "engineering",
          action: "messages.post",
          summary: "Post the result",
          status: "approved",
          expiresAt: new Date(NOW + 3_600_000).toISOString(),
          createdAt: new Date(NOW - 1000).toISOString(),
          decidedById: 1,
          decidedAt: new Date(NOW).toISOString(),
          decisionNote: null,
          githubLogin: null,
          fizzyUserName: null,
          adminOnly: false,
          approvable: true,
          deniable: true,
          updatedAt: rowVersionFixture(NOW),
        };

        const newest = { ...approved, updatedAt: rowVersionFixture(NOW + 2) };

        yield* fake.reply("GET /agents/9/approvals", {
          approvals: [approved],
          users: [],
          nextCursor: null,
        });
        yield* approvals.load(9, "approved");
        yield* fake.route("GET /agents/9/approvals", () =>
          Deferred.succeed(readStarted, undefined).pipe(
            Effect.andThen(Deferred.await(readGate)),
            Effect.as({ approvals: [], users: [], nextCursor: null }),
          ),
        );

        const reading = yield* Effect.forkChild(approvals.load(9, "approved"));

        yield* Deferred.await(readStarted);
        mutations.applyEvents(
          [
            {
              seq: 1,
              topic: "user:1",
              type: "approval.updated",
              data: { approval: newest, users: [] },
            },
          ],
          NOW,
        );
        yield* Deferred.succeed(readGate, undefined);
        yield* Fiber.join(reading);

        expect(store.getState().approvals.items[1]).toEqual(newest);
        expect(approvalListOf(store.getState(), approvalListKey(9, "approved")).ids).toEqual([1]);
        expect((yield* fake.requests).length).toBe(2);
      }).pipe(Effect.provide(FakeApi.layerClient)),
  );
});
