import { describe, expect, it } from "vitest";
import type { PersonalAgent } from "../../gen/PersonalAgent.ts";
import type { PersonalAgentCredential } from "../../gen/PersonalAgentCredential.ts";
import type { PersonalAgentList } from "../../gen/PersonalAgentList.ts";
import { ActionError } from "../../sync/run.ts";
import {
  abilityLabel,
  agentIcon,
  baseProblem,
  canCreate,
  clientChoices,
  emptyDraft,
  expiresAtOf,
  iconOf,
  parseDraft,
  roomNames,
  serializeDraft,
  sortedTokens,
  stepProblem,
  tokenMeta,
  turnedOff,
  usage,
  withClient,
} from "./agents-format.ts";

const CAPABILITIES = ["read_messages", "post_messages", "react", "manage_threads"];

function list(change: Partial<PersonalAgentList> = {}): PersonalAgentList {
  return {
    agents: [],
    enabled: true,
    limit: 3,
    capabilities: CAPABILITIES,
    rooms: [
      { id: 1, name: "General", private: false },
      { id: 2, name: "Design", private: false },
    ],
    mcpUrl: "https://chat.example.com/agents/mcp",
    oauthAvailable: false,
    ...change,
  };
}

const AGENT: PersonalAgent = {
  id: 1,
  userId: 50,
  name: "Helper",
  iconName: null,
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
  credentials: [],
};

function token(change: Partial<PersonalAgentCredential>): PersonalAgentCredential {
  return {
    id: 1,
    kind: "token",
    name: "Primary token",
    lastFour: "abcd",
    createdAt: "2026-10-13T12:00:00.000Z",
    expiresAt: "2027-01-11T12:00:00.000Z",
    lastUsedAt: null,
    lastUsedIp: null,
    state: "active",
    ...change,
  };
}

describe("clients", () => {
  it("offers the token clients, and the web apps only with OAuth", () => {
    expect(clientChoices(false).map((c) => c.label)).toEqual([
      "Claude Code",
      "Codex CLI",
      "Cursor",
      "Other MCP or API client",
    ]);
    expect(clientChoices(true).map((c) => c.value)).toEqual([
      "claude-ai",
      "chatgpt",
      "grok",
      "claude-code",
      "codex",
      "cursor",
      "other",
    ]);
  });

  it("takes the client's icon unless one was picked by hand", () => {
    const [claude, codex] = clientChoices(false);

    if (claude === undefined || codex === undefined) throw new Error("no clients");

    const picked = withClient(emptyDraft(CAPABILITIES), claude);

    expect(picked).toMatchObject({ client: "claude-code", iconName: "sparkles" });
    expect(withClient(picked, codex).iconName).toBe("robot");
    expect(withClient({ ...picked, iconName: "owl" }, codex).iconName).toBe("owl");
  });
});

describe("the wizard's steps", () => {
  const draft = emptyDraft(CAPABILITIES);

  it("starts with every ability and no rooms, for 90 days", () => {
    expect(draft).toEqual({
      client: null,
      name: "",
      iconName: null,
      roomIds: [],
      capabilities: CAPABILITIES,
      expiry: "90d",
    });
    expect(expiresAtOf("90d")).toBeNull();
    expect(expiresAtOf("never")).toBe("never");
  });

  it("asks for a client, then a name, then an ability for any rooms", () => {
    expect(stepProblem("client", draft)).toBe("Choose where you'll use the agent.");
    expect(stepProblem("client", { ...draft, client: "codex" })).toBeNull();
    expect(stepProblem("identity", { ...draft, name: "  " })).toBe("Give the agent a name.");
    expect(stepProblem("identity", { ...draft, name: "Helper" })).toBeNull();
    expect(stepProblem("access", { ...draft, capabilities: [] })).toBeNull();
    expect(stepProblem("access", { ...draft, roomIds: [1], capabilities: [] })).toBe(
      "Choose at least one ability for its rooms.",
    );
    expect(stepProblem("connect", draft)).toBeNull();
  });

  it("keeps a draft across a Google confirmation, and refuses one it doesn't know", () => {
    const kept = {
      ...draft,
      client: "cursor" as const,
      name: "Helper, the second",
      iconName: "owl",
      roomIds: [2, 1],
      capabilities: ["react"],
      expiry: "never" as const,
    };

    expect(parseDraft(serializeDraft(kept))).toEqual(kept);
    expect(parseDraft(serializeDraft({ ...kept, iconName: null, roomIds: [] }))).toEqual({
      ...kept,
      iconName: null,
      roomIds: [],
    });
    expect(parseDraft("client=nope&expiry=90d")).toBeNull();
    expect(parseDraft("client=codex&expiry=forever")).toBeNull();
    expect(parseDraft("client=codex&expiry=90d&roomIds=1,x,-2")?.roomIds).toEqual([1]);
  });
});

describe("the cards", () => {
  it("counts agents against the limit; administrators have none", () => {
    const full = list({ limit: 1, agents: [AGENT] });

    expect(usage(list())).toBe("0 of 3 agents");
    expect(canCreate(list())).toBe(true);
    expect(canCreate(full)).toBe(false);
    expect(usage(list({ limit: null }))).toBeNull();
    expect(canCreate(list({ limit: null, agents: [AGENT] }))).toBe(true);
    expect(canCreate(list({ enabled: false }))).toBe(false);
  });

  it("names an agent's rooms, dropping any it can no longer reach", () => {
    expect(roomNames(list(), [2, 1, 99])).toEqual(["General", "Design"]);
  });

  it("names abilities, spelling out one it doesn't know", () => {
    expect(abilityLabel("manage_threads")).toBe("Manage threads");
    expect(abilityLabel("future_power")).toBe("future power");
  });

  it("draws a known icon as its emoji, and anything else as the drawn bot", () => {
    expect(iconOf("owl")).toMatchObject({ kind: "emoji", character: "🦉" });
    expect(iconOf("claude")).toBeNull();
    expect(iconOf(null)).toBeNull();
  });

  it("shows the bot's icon image when the server resolved one, else its emoji", () => {
    expect(agentIcon({ ...AGENT, iconName: "claude", iconUrl: "/icons/claude" })).toMatchObject({
      character: null,
      imageUrl: "/icons/claude",
    });
    expect(agentIcon({ ...AGENT, iconName: "owl" })).toMatchObject({ character: "🦉" });
    expect(agentIcon(AGENT)).toBeNull();
  });

  it("lists tokens newest first, revoked ones last", () => {
    const sorted = sortedTokens([
      token({ id: 1 }),
      token({ id: 3, state: "revoked" }),
      token({ id: 2 }),
    ]);

    expect(sorted.map((t) => t.id)).toEqual([2, 1, 3]);
  });

  it("says when a token was made, when it expires and when it was used", () => {
    const ago = () => "5 minutes ago";

    expect(tokenMeta(token({}), ago)).toMatch(/^Created .+2026 · expires .+2027 · never used$/);
    expect(tokenMeta(token({ expiresAt: null, lastUsedAt: "2026-10-13T12:05:00Z" }), ago)).toMatch(
      / · never expires · last used 5 minutes ago$/,
    );
    expect(tokenMeta(token({ state: "expired" }), ago)).toContain(" · expired ");
    expect(tokenMeta(token({ state: "revoked" }), ago)).not.toContain("expire");
  });
});

describe("refusals", () => {
  it("reads the agent limit as a form-level sentence", () => {
    const limit = new ActionError("Validation", "Personal agent limit reached", {
      base: ["personal agent limit reached"],
    });

    expect(baseProblem(limit)).toBe("Personal agent limit reached.");
    expect(
      baseProblem(new ActionError("Validation", "Invalid", { name: ["can't be blank"] })),
    ).toBeNull();
    expect(baseProblem(new Error("boom"))).toBeNull();
  });

  it("knows a 403 means personal agents are turned off", () => {
    expect(
      turnedOff(new ActionError("Forbidden", "Personal agents are turned off for this workspace.")),
    ).toBe(true);
    expect(turnedOff(new ActionError("NotFound", "Not found"))).toBe(false);
  });
});
