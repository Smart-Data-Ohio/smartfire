import { describe, expect, it } from "vitest";
import type { PersonalAgent } from "../../gen/PersonalAgent.ts";
import type { PersonalAgentConnection } from "../../gen/PersonalAgentConnection.ts";
import type { PersonalAgentCredential } from "../../gen/PersonalAgentCredential.ts";
import type { PersonalAgentList } from "../../gen/PersonalAgentList.ts";
import {
  type AgentCache,
  isUnconfirmed,
  nextSequence,
  withAgent,
  withCreated,
  withList,
  withRemoved,
  withTokens,
  withUnconfirmed,
} from "./agents-cache.ts";

const TOKEN: PersonalAgentCredential = {
  id: 11,
  kind: "token",
  name: "Primary token",
  lastFour: "beef",
  createdAt: "2026-10-13T12:00:00.000Z",
  expiresAt: null,
  lastUsedAt: null,
  lastUsedIp: null,
  state: "active",
  connection: null,
};

const AGENT: PersonalAgent = {
  id: 7,
  userId: 70,
  name: "Helper",
  iconName: "owl",
  iconUrl: null,
  botAvatar: null,
  avatarUrl: null,
  provider: null,
  runtime: "codex",
  description: null,
  dailyMessageCap: 200,
  suspended: false,
  roomIds: [],
  capabilities: [],
  lastUsedAt: null,
  createdAt: "2026-10-13T12:00:00.000Z",
  credentials: [TOKEN],
};

function list(agents: readonly PersonalAgent[]): PersonalAgentList {
  return {
    agents: [...agents],
    enabled: true,
    limit: 3,
    capabilities: [],
    rooms: [],
    mcpUrl: null,
    oauthAvailable: false,
  };
}

const names = (cache: AgentCache) => cache.list.agents.map((a) => a.name);

/** The first agent's tokens as `id:state`, in id order. */
const states = (cache: AgentCache) =>
  (cache.list.agents[0]?.credentials ?? []).map((c) => `${c.id}:${c.state}`).sort();

describe("the agents cache", () => {
  it("keeps a save that landed after a reload was sent, when the reload answers last", () => {
    const first = withList(null, list([AGENT]), nextSequence());
    const reload = nextSequence();
    const save = nextSequence();
    const saved = withAgent(first, { ...AGENT, name: "Third" }, save);

    expect(names(withList(saved, list([{ ...AGENT, name: "Renamed" }]), reload))).toEqual([
      "Third",
    ]);
  });

  it("takes a reload sent after the save, and drops a save answered after a newer one", () => {
    const first = withList(null, list([AGENT]), nextSequence());
    const older = nextSequence();
    const newer = nextSequence();
    const saved = withAgent(first, { ...AGENT, name: "Newer" }, newer);

    expect(names(withAgent(saved, { ...AGENT, name: "Older" }, older))).toEqual(["Newer"]);
    expect(names(withList(saved, list([{ ...AGENT, name: "Server" }]), nextSequence()))).toEqual([
      "Server",
    ]);
  });

  it("ignores a whole list older than the one it holds", () => {
    const older = nextSequence();
    const newer = withList(null, list([{ ...AGENT, name: "Newer" }]), nextSequence());

    const after = withList(newer, list([{ ...AGENT, name: "Older" }]), older);

    expect(names(after)).toEqual(["Newer"]);
    expect(states(after)).toEqual(states(newer));
  });

  it("never brings back a removed agent from an older reload, but drops one a newer omits", () => {
    const other = { ...AGENT, id: 8, name: "Other" };
    const first = withList(null, list([AGENT, other]), nextSequence());
    const reload = nextSequence();
    const removed = withRemoved(first, AGENT.id, nextSequence());

    expect(names(withList(removed, list([AGENT, other]), reload))).toEqual(["Other"]);
    expect(names(withList(first, list([other]), nextSequence()))).toEqual(["Other"]);
  });

  it("keeps an agent connected after a reload was sent", () => {
    const first = withList(null, list([AGENT]), nextSequence());
    const reload = nextSequence();
    const made = withCreated(first, { ...AGENT, id: 9, name: "New" }, nextSequence());

    expect(names(withList(made, list([AGENT]), reload))).toEqual(["Helper", "New"]);
  });

  it("keeps a token revoked under an older reload that shows it active", () => {
    const first = withList(null, list([AGENT]), nextSequence());
    const reload = nextSequence();
    const revoked = withTokens(first, AGENT.id, [{ ...TOKEN, state: "revoked" }], nextSequence());
    const after = withList(revoked, list([{ ...AGENT, name: "Renamed" }]), reload);

    expect(after.list.agents[0]?.name).toBe("Renamed");
    expect(after.list.agents[0]?.credentials[0]?.state).toBe("revoked");
  });

  // Revoke A is sent, then revoke B, but the server carries out B first: B's answer still shows A
  // active, and A's answer shows both revoked. Neither order of answers brings A back.
  describe("when a revocation sent second is carried out first", () => {
    const second = { ...TOKEN, id: 12, name: "Laptop" };

    const start = () =>
      withList(null, list([{ ...AGENT, credentials: [TOKEN, second] }]), nextSequence());

    const answerToA = [
      { ...TOKEN, state: "revoked" as const },
      { ...second, state: "revoked" as const },
    ];

    const answerToB = [TOKEN, { ...second, state: "revoked" as const }];

    it("keeps both revoked when A answers first", () => {
      const cache = start();
      const a = nextSequence();
      const b = nextSequence();
      const after = withTokens(withTokens(cache, AGENT.id, answerToA, a), AGENT.id, answerToB, b);

      expect(states(after)).toEqual(["11:revoked", "12:revoked"]);
    });

    it("keeps both revoked when B answers first", () => {
      const cache = start();
      const a = nextSequence();
      const b = nextSequence();
      const after = withTokens(withTokens(cache, AGENT.id, answerToB, b), AGENT.id, answerToA, a);

      expect(states(after)).toEqual(["11:revoked", "12:revoked"]);
    });

    it("keeps both revoked under a newer reload that still shows A active", () => {
      const revoked = withTokens(start(), AGENT.id, answerToA, nextSequence());
      const after = withList(revoked, list([{ ...AGENT, credentials: answerToB }]), nextSequence());

      expect(states(after)).toEqual(["11:revoked", "12:revoked"]);
    });
  });

  it("adds a token issued while another answer was out", () => {
    const first = withList(null, list([AGENT]), nextSequence());
    const issued = { ...TOKEN, id: 13, name: "New" };
    const issue = withTokens(first, AGENT.id, [issued], null);
    const after = withTokens(issue, AGENT.id, [{ ...TOKEN, state: "revoked" }], nextSequence());

    expect(states(after)).toEqual(["11:revoked", "13:active"]);
  });

  describe("a connection's details", () => {
    const DETAILS: PersonalAgentConnection = {
      clientName: "Example Assistant",
      clientHost: "client.example",
      clientId: "https://client.example/oauth/client.json",
      clientBlocked: false,
      revokedReason: null,
    };

    const CONNECTION: PersonalAgentCredential = {
      ...TOKEN,
      id: 21,
      kind: "oauth",
      name: "Example Assistant",
      lastFour: "oauth",
      connection: DETAILS,
    };

    const connected = (details: Partial<PersonalAgentConnection>): PersonalAgentCredential => ({
      ...CONNECTION,
      connection: { ...DETAILS, ...details },
    });

    const start = () =>
      withList(null, list([{ ...AGENT, credentials: [CONNECTION] }]), nextSequence());

    const first = (cache: AgentCache) => cache.list.agents[0]?.credentials[0];

    it("keeps a block a later-sent load showed when an older one answers last, both active", () => {
      const cache = start();
      const older = nextSequence();
      const newer = nextSequence();

      const blocked = withList(
        cache,
        list([{ ...AGENT, credentials: [connected({ clientBlocked: true })] }]),
        newer,
      );

      const after = withList(
        blocked,
        list([{ ...AGENT, credentials: [connected({ clientBlocked: false })] }]),
        older,
      );

      // A whole-list load older than the one held is dropped; an agent answer isn't.
      const answered = withAgent(
        blocked,
        { ...AGENT, credentials: [connected({ clientBlocked: false })] },
        older,
      );

      expect(first(after)?.connection?.clientBlocked).toBe(true);
      expect(first(answered)?.connection?.clientBlocked).toBe(true);
      expect(first(answered)?.state).toBe("active");
    });

    it("takes an unblock and last use from a later-sent answer", () => {
      const blocked = withAgent(
        start(),
        { ...AGENT, credentials: [connected({ clientBlocked: true })] },
        nextSequence(),
      );

      const used = {
        ...connected({ clientBlocked: false }),
        lastUsedAt: "2026-10-13T15:00:00.000Z",
      };

      const after = withAgent(blocked, { ...AGENT, credentials: [used] }, nextSequence());

      expect(first(after)?.connection?.clientBlocked).toBe(false);
      expect(first(after)?.lastUsedAt).toBe("2026-10-13T15:00:00.000Z");
    });

    it("keeps an older answer's revocation, with its reason, under newer active details", () => {
      const cache = start();
      const older = nextSequence();
      const newer = nextSequence();

      const used = withAgent(
        cache,
        {
          ...AGENT,
          credentials: [{ ...CONNECTION, lastUsedAt: "2026-10-13T15:00:00.000Z" }],
        },
        newer,
      );

      const reused = {
        ...connected({ revokedReason: "tokenReused" }),
        state: "revoked" as const,
      };

      const after = withTokens(used, AGENT.id, [reused], older);

      expect(first(after)?.state).toBe("revoked");
      expect(first(after)?.connection?.revokedReason).toBe("tokenReused");
      expect(first(after)?.lastUsedAt).toBe("2026-10-13T15:00:00.000Z");
    });

    // Sent A, B, C, carried out by the server in another order: C's answer still shows the
    // connection active, A's shows it revoked without a reason, B's revoked for a reused token.
    // However the answers arrive, it ends revoked with the reason.
    describe("when answers sent A, B, C arrive in any order", () => {
      const answers = {
        a: { ...CONNECTION, state: "revoked" as const },
        b: { ...connected({ revokedReason: "tokenReused" }), state: "revoked" as const },
        c: CONNECTION,
      };

      const orders = [
        ["a", "b", "c"],
        ["a", "c", "b"],
        ["b", "a", "c"],
        ["b", "c", "a"],
        ["c", "a", "b"],
        ["c", "b", "a"],
      ] as const;

      type Via = "list" | "tokens";

      /** Answer `name`, sent as `at`, landing as a whole-list load or as a token answer. */
      const land = (cache: AgentCache, name: keyof typeof answers, at: number, via: Via) =>
        via === "list"
          ? withList(cache, list([{ ...AGENT, credentials: [answers[name]] }]), at)
          : withTokens(cache, AGENT.id, [answers[name]], at);

      // How each of A, B and C lands: all as reloads, or reloads mixed with token answers.
      const mixes: readonly (readonly [Via, Via, Via])[] = [
        ["list", "list", "list"],
        ["list", "tokens", "list"],
        ["tokens", "list", "tokens"],
        ["tokens", "tokens", "list"],
        ["list", "list", "tokens"],
      ];

      for (const mix of mixes) {
        for (const order of orders) {
          it(`keeps the reason when ${mix.join("/")} answers arrive ${order.join(", ")}`, () => {
            const cache = start();
            const sent = { a: nextSequence(), b: nextSequence(), c: nextSequence() };
            const via = { a: mix[0], b: mix[1], c: mix[2] };

            const after = order.reduce(
              (current, name) => land(current, name, sent[name], via[name]),
              cache,
            );

            expect(first(after)?.state).toBe("revoked");
            expect(first(after)?.connection?.revokedReason).toBe("tokenReused");
          });
        }
      }

      for (const order of orders) {
        it(`keeps the reason when they arrive ${order.join(", ")}`, () => {
          const cache = start();
          const sent = { a: nextSequence(), b: nextSequence(), c: nextSequence() };

          const after = order.reduce(
            (current, name) => withTokens(current, AGENT.id, [answers[name]], sent[name]),
            cache,
          );

          expect(first(after)?.state).toBe("revoked");
          expect(first(after)?.connection?.revokedReason).toBe("tokenReused");
        });
      }
    });
  });

  it("keeps an agent unconfirmed until an answer to a request sent after its save lands", () => {
    const first = withList(null, list([AGENT]), nextSequence());
    const olderReload = nextSequence();
    const unanswered = withUnconfirmed(first, AGENT.id, nextSequence());

    expect(isUnconfirmed(unanswered, AGENT.id)).toBe(true);
    expect(isUnconfirmed(withList(unanswered, list([AGENT]), olderReload), AGENT.id)).toBe(true);
    expect(isUnconfirmed(withList(unanswered, list([AGENT]), nextSequence()), AGENT.id)).toBe(
      false,
    );
    expect(isUnconfirmed(withAgent(unanswered, AGENT, nextSequence()), AGENT.id)).toBe(false);
    expect(isUnconfirmed(withRemoved(unanswered, AGENT.id, nextSequence()), AGENT.id)).toBe(false);
  });
});
