import { describe, expect, it } from "vitest";
import type { PersonalAgent } from "../../gen/PersonalAgent.ts";
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

    expect(withList(newer, list([{ ...AGENT, name: "Older" }]), older)).toBe(newer);
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
    const revoked = withTokens(first, AGENT.id, [{ ...TOKEN, state: "revoked" }]);
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
      const after = withTokens(withTokens(start(), AGENT.id, answerToA), AGENT.id, answerToB);

      expect(states(after)).toEqual(["11:revoked", "12:revoked"]);
    });

    it("keeps both revoked when B answers first", () => {
      const after = withTokens(withTokens(start(), AGENT.id, answerToB), AGENT.id, answerToA);

      expect(states(after)).toEqual(["11:revoked", "12:revoked"]);
    });

    it("keeps both revoked under a newer reload that still shows A active", () => {
      const revoked = withTokens(start(), AGENT.id, answerToA);
      const after = withList(revoked, list([{ ...AGENT, credentials: answerToB }]), nextSequence());

      expect(states(after)).toEqual(["11:revoked", "12:revoked"]);
    });
  });

  it("adds a token issued while another answer was out", () => {
    const first = withList(null, list([AGENT]), nextSequence());
    const issued = { ...TOKEN, id: 13, name: "New" };
    const issue = withTokens(first, AGENT.id, [issued]);
    const after = withTokens(issue, AGENT.id, [{ ...TOKEN, state: "revoked" }]);

    expect(states(after)).toEqual(["11:revoked", "13:active"]);
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
