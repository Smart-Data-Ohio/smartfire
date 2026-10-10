import { afterEach, describe, expect, it } from "@effect/vitest";
import { Deferred, Effect, Fiber } from "effect";
import { users } from "../api/endpoints.ts";
import { FakeApi, userFixture } from "../api/testing.ts";
import type { User } from "../gen/User.ts";
import { mutations, store } from "../store/store.ts";

const USER = 7;

function expired(): User {
  return {
    ...userFixture(USER),
    role: "bot",
    hasAvatar: true,
    customStatus: null,
    avatarIcon: null,
    agent: { agentId: 9, kind: "workspace", status: "working", suspended: true },
  };
}

function captured(): User {
  return {
    ...userFixture(USER),
    role: "bot",
    hasAvatar: false,
    customStatus: { emoji: "🌴", text: "Away", expiresAt: "2026-10-07T10:00:00.000Z" },
    avatarIcon: {
      name: "robot",
      title: "Robot",
      kind: "emoji",
      character: "🤖",
      imageUrl: null,
      animated: false,
      stillUrl: null,
    },
    agent: { agentId: 9, kind: "workspace", status: "idle", suspended: false },
  };
}

const read = users([USER]).pipe(
  Effect.tap((list) => Effect.sync(() => mutations.mergeUsers(list.users))),
);

describe("user presentation observations across API requests", () => {
  afterEach(() => mutations.reset());

  it.effect("keeps newly read expiry, avatar and badge fields after a delayed tied response", () =>
    Effect.gen(function* () {
      const fake = yield* FakeApi;
      const started = yield* Deferred.make<void>();
      const gate = yield* Deferred.make<void>();

      yield* fake.route("GET /users", () =>
        Deferred.succeed(started, undefined).pipe(
          Effect.andThen(Deferred.await(gate)),
          Effect.as({ users: [captured()] }),
        ),
      );

      const loading = yield* Effect.forkChild(read);

      yield* Deferred.await(started);
      yield* fake.reply("GET /users", { users: [expired()] });
      yield* read;
      yield* Deferred.succeed(gate, undefined);
      yield* Fiber.join(loading);

      expect(store.getState().users[USER]).toEqual(expired());
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("keeps an identical newer read against a delayed tied ABA response", () =>
    Effect.gen(function* () {
      const fake = yield* FakeApi;
      const started = yield* Deferred.make<void>();
      const gate = yield* Deferred.make<void>();

      mutations.mergeUsers([expired()]);
      yield* fake.route("GET /users", () =>
        Deferred.succeed(started, undefined).pipe(
          Effect.andThen(Deferred.await(gate)),
          Effect.as({ users: [captured()] }),
        ),
      );

      const loading = yield* Effect.forkChild(read);

      yield* Deferred.await(started);
      yield* fake.reply("GET /users", { users: [expired()] });
      yield* read;
      yield* Deferred.succeed(gate, undefined);
      yield* Fiber.join(loading);

      expect(store.getState().users[USER]).toEqual(expired());
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("keeps newer edited status fields when a newer request returns an older row", () =>
    Effect.gen(function* () {
      const fake = yield* FakeApi;

      const current = {
        ...expired(),
        updatedAt: "2026-10-07T10:01:00.000000Z",
        customStatus: { emoji: "💻", text: "Working", expiresAt: null },
      };

      mutations.mergeUsers([current]);
      yield* fake.reply("GET /users", {
        users: [{ ...expired(), updatedAt: "2026-10-07T10:00:00.000000Z" }],
      });
      yield* read;

      expect(store.getState().users[USER]?.customStatus).toEqual(current.customStatus);
      expect(store.getState().users[USER]?.updatedAt).toBe(current.updatedAt);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("keeps newer icon identity when a newer request returns an older user row", () =>
    Effect.gen(function* () {
      const fake = yield* FakeApi;

      const current: User = {
        ...captured(),
        updatedAt: "2026-10-07T10:01:00.000000Z",
        avatarIcon: {
          name: "rocket",
          title: "Rocket",
          kind: "emoji",
          character: "🚀",
          imageUrl: null,
          animated: false,
          stillUrl: null,
        },
      };

      mutations.mergeUsers([current]);
      yield* fake.reply("GET /users", {
        users: [{ ...captured(), updatedAt: "2026-10-07T10:00:00.000000Z" }],
      });
      yield* read;

      expect(store.getState().users[USER]?.avatarIcon).toEqual(current.avatarIcon);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("keeps the canonical human role without an older bot's badge", () =>
    Effect.gen(function* () {
      const fake = yield* FakeApi;
      const current = { ...userFixture(USER), updatedAt: "2026-10-07T10:01:00.000000Z" };

      mutations.mergeUsers([current]);
      yield* fake.reply("GET /users", {
        users: [{ ...captured(), updatedAt: "2026-10-07T10:00:00.000000Z" }],
      });
      yield* read;

      expect(store.getState().users[USER]).toMatchObject({ role: "member", agent: null });
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("keeps an event's fields when a response from an earlier request lands", () =>
    Effect.gen(function* () {
      const fake = yield* FakeApi;
      const started = yield* Deferred.make<void>();
      const gate = yield* Deferred.make<void>();

      yield* fake.route("GET /users", () =>
        Deferred.succeed(started, undefined).pipe(
          Effect.andThen(Deferred.await(gate)),
          Effect.as({ users: [captured()] }),
        ),
      );

      const loading = yield* Effect.forkChild(read);

      yield* Deferred.await(started);
      mutations.applyEvents(
        [
          {
            seq: 1,
            topic: "user:7",
            type: "approval.updated",
            data: {
              users: [expired()],
              approval: {
                id: 1,
                agentId: 9,
                agentUserId: USER,
                roomId: null,
                roomName: null,
                action: "messages.post",
                summary: "Request",
                status: "pending",
                expiresAt: "2026-10-08T10:00:00.000Z",
                createdAt: "2026-10-07T10:00:00.000Z",
                decidedById: null,
                decidedAt: null,
                decisionNote: null,
                githubLogin: null,
                fizzyUserName: null,
                adminOnly: false,
                approvable: true,
                deniable: true,
                updatedAt: "2026-10-07T10:00:00.000000Z",
              },
            },
          },
        ],
        0,
      );
      yield* Deferred.succeed(gate, undefined);
      yield* Fiber.join(loading);

      expect(store.getState().users[USER]).toEqual(expired());
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );
});
