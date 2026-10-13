import { describe, expect, it } from "vitest";
import type { PersonalAgent } from "../../gen/PersonalAgent.ts";
import type { PersonalAgentCredential } from "../../gen/PersonalAgentCredential.ts";
import type { PersonalAgentList } from "../../gen/PersonalAgentList.ts";
import {
  nextSequence,
  withAgent,
  withCreated,
  withList,
  withRemoved,
  withTokens,
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

const names = (cache: { list: PersonalAgentList }) => cache.list.agents.map((a) => a.name);

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

  it("keeps a newer token change under an older reload's profile", () => {
    const first = withList(null, list([AGENT]), nextSequence());
    const reload = nextSequence();

    const revoked = withTokens(first, AGENT.id, [{ ...TOKEN, state: "revoked" }], nextSequence());

    const after = withList(revoked, list([{ ...AGENT, name: "Renamed" }]), reload);

    expect(after.list.agents[0]?.name).toBe("Renamed");
    expect(after.list.agents[0]?.credentials[0]?.state).toBe("revoked");
  });

  it("never lets an older revocation's list show a token a newer one revoked as active", () => {
    const second = { ...TOKEN, id: 12, name: "Laptop" };
    const both = [TOKEN, second];
    const first = withList(null, list([{ ...AGENT, credentials: both }]), nextSequence());
    const revokeFirst = nextSequence();
    const revokeSecond = nextSequence();

    // The second revocation answers first: both gone by then.
    const ended = withTokens(
      first,
      AGENT.id,
      [
        { ...TOKEN, state: "revoked" },
        { ...second, state: "revoked" },
      ],
      revokeSecond,
    );

    // The first answers last, from before the second: only its own token revoked.
    const after = withTokens(
      ended,
      AGENT.id,
      [{ ...TOKEN, state: "revoked" }, second],
      revokeFirst,
    );

    const states = after.list.agents[0]?.credentials.map((c) => `${c.id}:${c.state}`);

    expect(states?.sort()).toEqual(["11:revoked", "12:revoked"]);
  });

  it("adds a token issued while an older answer was out", () => {
    const first = withList(null, list([AGENT]), nextSequence());
    const revoke = nextSequence();
    const issued = { ...TOKEN, id: 13, name: "New" };
    const issue = withTokens(first, AGENT.id, [issued], nextSequence());
    const after = withTokens(issue, AGENT.id, [{ ...TOKEN, state: "revoked" }], revoke);

    expect(after.list.agents[0]?.credentials.map((c) => c.id).sort()).toEqual([11, 13]);
    expect(after.list.agents[0]?.credentials.find((c) => c.id === 11)?.state).toBe("revoked");
  });
});
