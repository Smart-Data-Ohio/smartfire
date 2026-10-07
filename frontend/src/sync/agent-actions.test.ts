import { afterEach, describe, expect, it } from "@effect/vitest";
import { Deferred, Effect, Fiber } from "effect";
import { NotFound, ServerError } from "../api/errors.ts";
import { FakeApi, userFixture } from "../api/testing.ts";
import type { AgentDirectory } from "../gen/AgentDirectory.ts";
import type { AgentDirectoryRow } from "../gen/AgentDirectoryRow.ts";
import type { AgentProfile } from "../gen/AgentProfile.ts";
import { profileOf, workingPresenceAt } from "../store/agents.ts";
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
    users: [
      ...ids.map((id) => ({
        ...userFixture(id),
        role: "bot" as const,
        agent: {
          agentId: id,
          kind: "personal" as const,
          status: "idle" as const,
          suspended: false,
        },
      })),
      userFixture(2),
    ],
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
  users: directory([40]).users,
};

describe("agent actions", () => {
  afterEach(() => mutations.reset());

  for (const older of ["success", "error"] as const) {
    it.effect(`drop a stale profile ${older} after reopening loads a newer success`, () =>
      Effect.gen(function* () {
        const fake = yield* FakeApi;
        const started = yield* Deferred.make<void>();
        const gate = yield* Deferred.make<void>();

        yield* fake.route("GET /agents/40", () =>
          Deferred.succeed(started, undefined).pipe(
            Effect.andThen(Deferred.await(gate)),
            Effect.andThen(
              older === "success"
                ? Effect.succeed({ ...profile, description: "Old" })
                : Effect.fail(new NotFound({ message: "Old error" })),
            ),
          ),
        );

        const first = yield* Effect.forkChild(agents.loadProfile(40));

        yield* Deferred.await(started);
        yield* fake.reply("GET /agents/40", { ...profile, description: "New" });
        yield* agents.loadProfile(40);
        yield* Deferred.succeed(gate, undefined);
        yield* Fiber.join(first);

        expect(profileOf(store.getState(), 40)).toMatchObject({
          status: "ready",
          profile: { description: "New" },
          error: null,
          missing: false,
        });
      }).pipe(Effect.provide(FakeApi.layerClient)),
    );
  }

  it.effect("drop a stale profile success after reopening fails", () =>
    Effect.gen(function* () {
      const fake = yield* FakeApi;
      const started = yield* Deferred.make<void>();
      const gate = yield* Deferred.make<void>();

      yield* fake.route("GET /agents/40", () =>
        Deferred.succeed(started, undefined).pipe(
          Effect.andThen(Deferred.await(gate)),
          Effect.as(profile),
        ),
      );

      const first = yield* Effect.forkChild(agents.loadProfile(40));

      yield* Deferred.await(started);
      yield* fake.route("GET /agents/40", () =>
        Effect.fail(new NotFound({ message: "New error" })),
      );
      yield* agents.loadProfile(40);
      yield* Deferred.succeed(gate, undefined);
      yield* Fiber.join(first);

      expect(profileOf(store.getState(), 40)).toMatchObject({
        status: "error",
        profile: null,
        error: "New error",
        missing: true,
      });
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  for (const screen of ["directory", "profile"] as const) {
    it.effect(`keep live status and presence after a delayed ${screen} GET`, () =>
      Effect.gen(function* () {
        const fake = yield* FakeApi;
        const started = yield* Deferred.make<void>();
        const gate = yield* Deferred.make<void>();
        const path = screen === "directory" ? "GET /agents" : "GET /agents/40";

        let calls = 0;

        yield* fake.route(path, () => {
          calls += 1;

          if (calls > 1) {
            expect(store.getState().agents.rows[40]?.status).toBe("working");

            const live = {
              ...row(40),
              status: "working" as const,
              statusNote: "Live",
              suspended: true,
            };

            return Effect.succeed(
              screen === "directory"
                ? { ...directory([40]), agents: [live] }
                : { ...profile, agent: live },
            );
          }

          return Deferred.succeed(started, undefined).pipe(
            Effect.andThen(Deferred.await(gate)),
            Effect.as(screen === "directory" ? directory([40]) : profile),
          );
        });

        const loading = yield* Effect.forkChild(
          screen === "directory" ? agents.loadDirectory() : agents.loadProfile(40),
        );

        yield* Deferred.await(started);
        mutations.applyEvents(
          [
            {
              seq: 1,
              topic: "user:2",
              type: "agent.status",
              data: {
                agentId: 40,
                userId: 40,
                status: "working",
                statusNote: "Live",
                statusChangedAt: null,
                suspended: true,
                workingPresence: "Reading logs",
                workingPresenceExpiresAt: "2026-10-08T16:30:00.000Z",
              },
            },
          ],
          0,
        );
        yield* Deferred.succeed(gate, undefined);
        yield* Fiber.join(loading);

        expect(store.getState().agents.rows[40]).toMatchObject({
          status: "working",
          statusNote: "Live",
          suspended: true,
        });
        expect(store.getState().users[40]?.agent).toMatchObject({
          status: "working",
          suspended: true,
        });
        expect(workingPresenceAt(store.getState(), 40, 0)).toBe("Reading logs");

        if (screen === "profile") {
          expect(profileOf(store.getState(), 40).profile?.agent.status).toBe("working");
        }
      }).pipe(Effect.provide(FakeApi.layerClient)),
    );
  }

  for (const screen of ["directory", "profile"] as const) {
    it.effect(`let a ${screen} response at T20 beat an intervening status event at T10`, () =>
      Effect.gen(function* () {
        const fake = yield* FakeApi;
        const started = yield* Deferred.make<void>();
        const gate = yield* Deferred.make<void>();
        const path = screen === "directory" ? "GET /agents" : "GET /agents/40";
        const newer = { ...row(40), statusChangedAt: "2026-10-07T16:20:00.000Z" };

        yield* fake.route(path, () =>
          Deferred.succeed(started, undefined).pipe(
            Effect.andThen(Deferred.await(gate)),
            Effect.as(
              screen === "directory"
                ? { ...directory([40]), agents: [newer] }
                : { ...profile, agent: newer },
            ),
          ),
        );

        const loading = yield* Effect.forkChild(
          screen === "directory" ? agents.loadDirectory() : agents.loadProfile(40),
        );

        yield* Deferred.await(started);
        mutations.applyEvents(
          [
            {
              seq: 1,
              topic: "user:2",
              type: "agent.status",
              data: {
                agentId: 40,
                userId: 40,
                status: "working",
                statusNote: "Old",
                statusChangedAt: "2026-10-07T16:10:00.000Z",
                suspended: false,
                workingPresence: null,
                workingPresenceExpiresAt: null,
              },
            },
          ],
          0,
        );
        yield* Deferred.succeed(gate, undefined);
        yield* Fiber.join(loading);

        expect(store.getState().agents.rows[40]).toMatchObject({
          status: "idle",
          statusChangedAt: newer.statusChangedAt,
        });
        expect(store.getState().users[40]?.agent?.status).toBe("idle");
      }).pipe(Effect.provide(FakeApi.layerClient)),
    );
  }

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
