import { afterEach, describe, expect, it } from "@effect/vitest";
import { Deferred, Effect, Fiber } from "effect";
import { NotFound, ServerError } from "../api/errors.ts";
import { FakeApi, userFixture } from "../api/testing.ts";
import type { AgentDirectory } from "../gen/AgentDirectory.ts";
import type { AgentDirectoryRow } from "../gen/AgentDirectoryRow.ts";
import type { AgentProfile } from "../gen/AgentProfile.ts";
import { profileOf } from "../store/agents.ts";
import { mutations, store } from "../store/store.ts";
import * as agents from "./agent-actions.ts";

function row(agentId: number): AgentDirectoryRow {
  return {
    agentId,
    userId: agentId,
    kind: "personal",
    ownerId: 2,
    status: "idle",
    statusNote: null,
    suspended: false,
    createdAt: "2026-10-01T00:00:00.000Z",
    statusChangedAt: null,
    lastSeenAt: null,
  };
}

function directory(ids: readonly number[]): AgentDirectory {
  return {
    agents: ids.map(row),
    users: [...ids.map((id) => ({ ...userFixture(id), role: "bot" as const })), userFixture(2)],
  };
}

const profile: AgentProfile = {
  agent: row(40),
  provider: "Anthropic",
  runtime: null,
  description: null,
  rooms: [],
  hiddenRoomCount: 0,
  grants: null,
  management: null,
  users: [userFixture(40), userFixture(2)],
};

describe("agent actions", () => {
  afterEach(() => mutations.reset());

  it.effect("land the directory with its users", () =>
    Effect.gen(function* () {
      const fake = yield* FakeApi;

      yield* fake.route("GET /agents", () => Effect.succeed(directory([40, 41])));
      yield* agents.loadDirectory();

      expect(store.getState().agents.directory).toMatchObject({ ids: [40, 41], status: "ready" });
      expect(store.getState().users[2]?.name).toBe("User 2");
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("drop a directory reply that a newer load replaced", () =>
    Effect.gen(function* () {
      const fake = yield* FakeApi;
      const gate = yield* Deferred.make<void>();
      let calls = 0;

      yield* fake.route("GET /agents", () => {
        calls += 1;

        return calls === 1
          ? Deferred.await(gate).pipe(Effect.as(directory([40])))
          : Effect.succeed(directory([41]));
      });

      const first = yield* Effect.forkChild(agents.loadDirectory());

      yield* Effect.yieldNow;
      yield* agents.loadDirectory();
      yield* Deferred.succeed(gate, undefined);
      yield* Fiber.join(first);

      expect(store.getState().agents.directory.ids).toEqual([41]);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("record a failed directory load as its error, without failing", () =>
    Effect.gen(function* () {
      const fake = yield* FakeApi;

      yield* fake.route("GET /agents", () =>
        Effect.fail(new ServerError({ message: "Something went wrong", status: 500 })),
      );
      yield* agents.loadDirectory();

      expect(store.getState().agents.directory).toMatchObject({
        status: "error",
        error: "Something went wrong",
      });
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("land a profile, and mark an unknown agent missing", () =>
    Effect.gen(function* () {
      const fake = yield* FakeApi;

      yield* fake.route("GET /agents/40", () => Effect.succeed(profile));
      yield* fake.route("GET /agents/99", () =>
        Effect.fail(new NotFound({ message: "Agent not found" })),
      );
      yield* agents.loadProfile(40);
      yield* agents.loadProfile(99);

      expect(profileOf(store.getState(), 40)).toMatchObject({ status: "ready", missing: false });
      expect(store.getState().agents.rows[40]?.kind).toBe("personal");
      expect(profileOf(store.getState(), 99)).toMatchObject({ status: "error", missing: true });
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );
});
