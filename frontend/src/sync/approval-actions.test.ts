import { afterEach, describe, expect, it } from "@effect/vitest";
import { Deferred, Effect, Exit, Fiber } from "effect";
import { Conflict, Forbidden, NotFound, Validation } from "../api/errors.ts";
import { FakeApi, meFixture } from "../api/testing.ts";
import type { AgentApproval } from "../gen/AgentApproval.ts";
import { approvalListKey, approvalListOf } from "../store/approvals.ts";
import { isLedgerForbidden, ledgerListKey, ledgerListOf } from "../store/ledger.ts";
import { mutations, store } from "../store/store.ts";
import * as approvals from "./approval-actions.ts";
import * as ledger from "./ledger-actions.ts";

const AGENT = 9;

const pending: AgentApproval = {
  id: 100,
  agentId: AGENT,
  agentUserId: AGENT,
  roomId: 3,
  roomName: "engineering",
  action: "github.merge_pull_request",
  summary: "Merge PR #318 into main",
  status: "pending",
  expiresAt: "2026-10-08T16:30:00.000Z",
  createdAt: "2026-10-06T16:30:00.000Z",
  decidedById: null,
  decidedAt: null,
  decisionNote: null,
  githubLogin: "ember-bot",
  fizzyUserName: null,
  adminOnly: true,
  approvable: true,
  deniable: true,
  updatedAt: "2026-10-06T16:30:00.000Z",
};

/** The viewer (Ada, user 7) with the agent's pending list loaded. */
const withPending = Effect.gen(function* () {
  const fake = yield* FakeApi;

  mutations.setMe(meFixture);
  yield* fake.route(`GET /agents/${AGENT}/approvals`, () =>
    Effect.succeed({ approvals: [pending], users: [], nextCursor: null }),
  );
  yield* approvals.load(AGENT, "pending");

  return fake;
});

function pendingIds(): readonly number[] {
  return approvalListOf(store.getState(), approvalListKey(AGENT, "pending")).ids;
}

describe("approval actions", () => {
  afterEach(() => mutations.reset());

  it.effect("keep a live Approved member after a late empty reload without another GET", () =>
    Effect.gen(function* () {
      const fake = yield* FakeApi;
      const started = yield* Deferred.make<void>();
      const gate = yield* Deferred.make<void>();

      const decided: AgentApproval = {
        ...pending,
        status: "approved",
        decidedAt: "2026-10-06T16:31:00.000Z",
        updatedAt: "2026-10-06T16:31:00.000Z",
      };

      yield* fake.reply(`GET /agents/${AGENT}/approvals`, {
        approvals: [],
        users: [],
        nextCursor: null,
      });
      yield* approvals.load(AGENT, "approved");
      yield* fake.route(`GET /agents/${AGENT}/approvals`, () =>
        Deferred.succeed(started, undefined).pipe(
          Effect.andThen(Deferred.await(gate)),
          Effect.as({ approvals: [], users: [], nextCursor: null }),
        ),
      );

      const loading = yield* Effect.forkChild(approvals.load(AGENT, "approved"));

      yield* Deferred.await(started);
      mutations.applyEvents(
        [
          {
            seq: 1,
            topic: "user:7",
            type: "approval.updated",
            data: { approval: decided, users: [] },
          },
        ],
        0,
      );
      yield* fake.route(`GET /agents/${AGENT}/approvals`, () => {
        expect(approvalListOf(store.getState(), approvalListKey(AGENT, "approved")).ids).toEqual([
          100,
        ]);

        return Effect.succeed({ approvals: [decided], users: [], nextCursor: null });
      });
      yield* Deferred.succeed(gate, undefined);
      yield* Fiber.join(loading);

      expect(approvalListOf(store.getState(), approvalListKey(AGENT, "approved")).ids).toEqual([
        100,
      ]);
      expect((yield* fake.requests).length).toBe(2);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("keep Pending's last loaded row after a late All page and page before it", () =>
    Effect.gen(function* () {
      const fake = yield* FakeApi;
      const started = yield* Deferred.make<void>();
      const gate = yield* Deferred.make<void>();

      yield* fake.route(`GET /agents/${AGENT}/approvals`, () =>
        Deferred.succeed(started, undefined).pipe(
          Effect.andThen(Deferred.await(gate)),
          Effect.as({ approvals: [{ ...pending, id: 51 }], users: [], nextCursor: null }),
        ),
      );

      const loading = yield* Effect.forkChild(approvals.load(AGENT, "all"));

      yield* Deferred.await(started);
      yield* fake.reply(`GET /agents/${AGENT}/approvals`, {
        approvals: Array.from({ length: 50 }, (_, index) => ({ ...pending, id: 100 - index })),
        users: [],
        nextCursor: "51",
      });
      yield* approvals.load(AGENT, "pending");
      yield* Deferred.succeed(gate, undefined);
      yield* Fiber.join(loading);

      expect(pendingIds()).toEqual(Array.from({ length: 50 }, (_, index) => 100 - index));

      yield* fake.reply(`GET /agents/${AGENT}/approvals`, {
        approvals: [{ ...pending, id: 50 }],
        users: [],
        nextCursor: null,
      });
      yield* approvals.loadMore(AGENT, "pending");

      expect(pendingIds().slice(-2)).toEqual([51, 50]);
      expect((yield* fake.requests).at(-1)?.query?.before).toBe("51");
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  for (const refusal of [
    ...["approved", "denied", "cancelled"].map((status) => {
      const message = `Request is already ${status}`;

      return new Validation({ message, fields: { base: [message] } });
    }),
    new Validation({ message: "Request has expired", fields: { base: ["Request has expired"] } }),
    new Validation({
      message: "Decision note is too long (maximum is 200 characters)",
      fields: { base: ["Decision note is too long (maximum is 200 characters)"] },
    }),
    new Validation({ message: "A different validation refusal", fields: {} }),
    new Conflict({ message: "This request changed" }),
    new NotFound({ message: "Not found" }),
  ]) {
    it.effect(`refetch the decided copy after ${refusal._tag}: ${refusal.message}`, () =>
      Effect.gen(function* () {
        const fake = yield* withPending;

        const decided: AgentApproval = {
          ...pending,
          status: refusal.message.includes("expired") ? "expired" : "denied",
          decidedById: refusal.message.includes("expired") ? null : 4,
          updatedAt: "2026-10-06T16:31:00.000Z",
          decidedAt: refusal.message.includes("expired") ? null : "2026-10-06T16:31:00.000Z",
        };

        yield* fake.route("PATCH /agent_approvals/100", () => Effect.fail(refusal));
        yield* fake.route(`GET /agents/${AGENT}/approvals`, (request) =>
          Effect.succeed({
            approvals: request.query?.status === "pending" ? [] : [decided],
            users: [],
            nextCursor: null,
          }),
        );

        const error = yield* Effect.flip(approvals.decide(100, "approved", null));

        expect(error).toBe(refusal);
        expect(store.getState().approvals.items[100]).toEqual(decided);
        expect(pendingIds()).toEqual([]);
        expect(
          (yield* fake.requests)
            .filter((request) => request.method === "GET")
            .map((request) => request.query?.status ?? null),
        ).toEqual(["pending", null, "pending"]);
      }).pipe(Effect.provide(FakeApi.layerClient)),
    );
  }

  it.effect("show a decision at once, then land the server's copy", () =>
    Effect.gen(function* () {
      const fake = yield* withPending;
      const gate = yield* Deferred.make<void>();

      const reply: AgentApproval = {
        ...pending,
        status: "approved",
        decidedById: 7,
        decidedAt: "2026-10-06T16:31:00.000Z",
        updatedAt: "2026-10-06T16:31:00.000Z",
        decisionNote: "Ship it",
      };

      yield* fake.route("PATCH /agent_approvals/100", (request) => {
        expect(request.body).toEqual({ decision: "approved", note: "Ship it" });

        return Deferred.await(gate).pipe(Effect.as(reply));
      });

      const deciding = yield* Effect.forkChild(approvals.decide(100, "approved", "Ship it"));

      yield* Effect.yieldNow;

      expect(store.getState().approvals.items[100]).toMatchObject({
        status: "approved",
        decidedById: 7,
      });
      expect(pendingIds()).toEqual([]);

      yield* Deferred.succeed(gate, undefined);

      const decided = yield* Fiber.join(deciding);

      expect(decided.decidedAt).toBe(reply.decidedAt);
      expect(store.getState().approvals.items[100]?.decidedAt).toBe(reply.decidedAt);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("put a refused decision back, and fail with the server's error", () =>
    Effect.gen(function* () {
      const fake = yield* withPending;

      yield* fake.route("PATCH /agent_approvals/100", () =>
        Effect.fail(
          new Forbidden({ message: "Only an administrator can approve GitHub write actions" }),
        ),
      );

      const exit = yield* Effect.exit(approvals.decide(100, "approved", null));

      expect(Exit.isFailure(exit)).toBe(true);
      expect(store.getState().approvals.items[100]).toEqual(pending);
      expect(pendingIds()).toEqual([100]);

      expect((yield* fake.requests).filter((request) => request.method === "GET").length).toBe(1);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("keep a newer copy that arrived meanwhile, and reload after a 404", () =>
    Effect.gen(function* () {
      const fake = yield* withPending;
      const gate = yield* Deferred.make<void>();

      const elsewhere: AgentApproval = {
        ...pending,
        status: "denied",
        decidedById: 4,
        updatedAt: "2026-10-06T16:31:00.000Z",
      };

      yield* fake.route("PATCH /agent_approvals/100", () =>
        Deferred.await(gate).pipe(
          Effect.andThen(Effect.fail(new NotFound({ message: "Not found" }))),
        ),
      );

      const deciding = yield* Effect.forkChild(
        Effect.exit(approvals.decide(100, "approved", null)),
      );

      yield* Effect.yieldNow;
      mutations.applyApproval(elsewhere);
      yield* Deferred.succeed(gate, undefined);
      yield* Fiber.join(deciding);

      expect(store.getState().approvals.items[100]).toEqual(elsewhere);
      expect(approvalListOf(store.getState(), approvalListKey(AGENT, "pending")).stale).toBe(false);
      expect(pendingIds()).toEqual([]);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );
});

describe("ledger actions", () => {
  afterEach(() => mutations.reset());

  it.effect("mark a refused ledger forbidden, and open it again once a page lands", () =>
    Effect.gen(function* () {
      const fake = yield* FakeApi;

      yield* fake.route(`GET /agents/${AGENT}/events`, () =>
        Effect.fail(new Forbidden({ message: "Only an administrator or its owner" })),
      );
      yield* ledger.load(AGENT, "all");

      expect(isLedgerForbidden(store.getState(), AGENT)).toBe(true);

      yield* fake.route(`GET /agents/${AGENT}/events`, () =>
        Effect.succeed({ events: [], users: [], nextCursor: null }),
      );
      yield* ledger.load(AGENT, "all");

      expect(isLedgerForbidden(store.getState(), AGENT)).toBe(false);
      expect(ledgerListOf(store.getState(), ledgerListKey(AGENT, "all")).status).toBe("ready");
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );
});
