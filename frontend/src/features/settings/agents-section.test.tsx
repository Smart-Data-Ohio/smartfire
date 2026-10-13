import {
  createMemoryHistory,
  createRootRoute,
  createRouter,
  RouterProvider,
} from "@tanstack/react-router";
import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { PersonalAgent } from "../../gen/PersonalAgent.ts";
import type { PersonalAgentCredential } from "../../gen/PersonalAgentCredential.ts";
import type { PersonalAgentList } from "../../gen/PersonalAgentList.ts";
import { meAgents } from "../../sync/settings.ts";
import { SecretBlock } from "./agent-parts.tsx";
import { AgentsSection } from "./agents-section.tsx";

beforeEach(() => {
  // jsdom has no matchMedia; the agent avatar asks it about the theme and reduced motion.
  window.matchMedia = (query: string) =>
    Object.assign(new EventTarget(), {
      matches: false,
      media: query,
      onchange: null,
      addListener: () => undefined,
      removeListener: () => undefined,
    });
});

afterEach(() => {
  vi.restoreAllMocks();
  sessionStorage.clear();
});

const SECRET = `sfa_${"ab".repeat(32)}`;

const TOKEN: PersonalAgentCredential = {
  id: 11,
  kind: "token",
  name: "Primary token",
  lastFour: "beef",
  createdAt: "2026-10-13T12:00:00.000Z",
  expiresAt: "2027-01-11T12:00:00.000Z",
  lastUsedAt: null,
  lastUsedIp: null,
  state: "active",
};

const AGENT: PersonalAgent = {
  id: 7,
  userId: 70,
  name: "Helper",
  iconName: "owl",
  provider: "openai",
  runtime: "codex",
  description: null,
  dailyMessageCap: 200,
  suspended: false,
  roomIds: [2],
  capabilities: ["post_messages", "read_messages"],
  lastUsedAt: null,
  credentials: [TOKEN],
};

function list(change: Partial<PersonalAgentList> = {}): PersonalAgentList {
  return {
    agents: [AGENT],
    enabled: true,
    limit: 3,
    capabilities: ["read_messages", "post_messages", "react", "manage_threads"],
    rooms: [
      { id: 1, name: "General" },
      { id: 2, name: "Design" },
    ],
    mcpUrl: "https://chat.example.com/agents/mcp",
    oauthAvailable: false,
    ...change,
  };
}

async function mount(answer: PersonalAgentList) {
  vi.spyOn(meAgents, "list").mockResolvedValue(answer);

  const root = createRootRoute({ component: AgentsSection });

  const router = createRouter({
    routeTree: root,
    history: createMemoryHistory({ initialEntries: ["/"] }),
  });

  render(<RouterProvider router={router} />);
  await act(() => router.load());
  await screen.findByRole("heading", { level: 1, name: "Agents" });
}

const next = () => fireEvent.click(screen.getByRole("button", { name: "Next" }));

describe("the cards", () => {
  it("shows an agent's client, rooms, abilities and tokens", async () => {
    await mount(list());

    const card = await screen.findByRole("article", { name: "Helper" });

    expect(within(card).getByText(/^Codex CLI · never used$/)).toBeTruthy();
    expect(within(card).getByText("Design")).toBeTruthy();
    expect(within(card).getByText("Post messages, Read messages")).toBeTruthy();
    expect(within(card).getByText("…beef")).toBeTruthy();
    expect(within(card).getByRole("button", { name: "Rotate Primary token" })).toBeTruthy();
    expect(within(card).getByRole("button", { name: "Revoke Primary token" })).toBeTruthy();
    expect(screen.getByText("1 of 3 agents")).toBeTruthy();
  });

  it("explains when personal agents are off, keeping only revoke and remove", async () => {
    await mount(list({ enabled: false }));

    expect(await screen.findByText(/Personal agents are turned off/)).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Connect an agent" })).toBeNull();
    expect(screen.queryByRole("button", { name: /^Rotate/ })).toBeNull();
    expect(screen.queryByRole("button", { name: /^Edit/ })).toBeNull();
    expect(screen.getByRole("button", { name: "Revoke Primary token" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Remove Helper" })).toBeTruthy();
  });

  it("won't start another agent at the limit", async () => {
    await mount(list({ limit: 1 }));

    const connect = await screen.findByRole("button", { name: "Connect an agent" });

    expect(connect.hasAttribute("disabled")).toBe(true);
    expect(screen.getByText(/Remove one to connect another/)).toBeTruthy();
  });

  it("rotates a token, showing the new one once with its client's setup", async () => {
    await mount(list());

    const rotate = vi.spyOn(meAgents, "rotateToken").mockResolvedValue({
      credential: { ...TOKEN, id: 12, lastFour: SECRET.slice(-4) },
      secret: SECRET,
    });

    fireEvent.click(await screen.findByRole("button", { name: "Rotate Primary token" }));
    fireEvent.click(within(screen.getByRole("dialog")).getByLabelText("Never"));
    fireEvent.click(screen.getByRole("button", { name: "Rotate token" }));

    const shown = await screen.findByRole("alertdialog", { name: "Helper's new token" });

    expect(rotate).toHaveBeenCalledWith(7, 11, "never");
    expect(within(shown).getByText(SECRET)).toBeTruthy();
    expect(within(shown).getByText(/bearer_token_env_var = "SMARTFIRE_HELPER_TOKEN"/)).toBeTruthy();
    expect(screen.getAllByText("Revoked")).toHaveLength(1);
  });
});

describe("the wizard", () => {
  it("offers only the token clients without OAuth, and the web apps with it", async () => {
    await mount(list({ agents: [] }));
    fireEvent.click(await screen.findByRole("button", { name: "Connect an agent" }));

    expect(screen.getByRole("radio", { name: /Claude Code/ })).toBeTruthy();
    expect(screen.queryByRole("radio", { name: /claude\.ai/ })).toBeNull();
    expect(screen.queryByRole("radio", { name: /ChatGPT/ })).toBeNull();
    expect(screen.queryByRole("radio", { name: /Grok/ })).toBeNull();
  });

  it("lists the web apps when the server can sign them in", async () => {
    await mount(list({ agents: [], oauthAvailable: true }));
    fireEvent.click(await screen.findByRole("button", { name: "Connect an agent" }));

    expect(screen.getByRole("radio", { name: /claude\.ai/ })).toBeTruthy();
    expect(screen.getByRole("radio", { name: /ChatGPT/ })).toBeTruthy();
    expect(screen.getByRole("radio", { name: /Grok/ })).toBeTruthy();
  });

  it("walks client, identity, access and connect, then shows the token once", async () => {
    await mount(list({ agents: [] }));

    const create = vi.spyOn(meAgents, "create").mockResolvedValue({
      agent: { ...AGENT, name: "Builder", runtime: "claude-code", roomIds: [1] },
      secret: SECRET,
    });

    fireEvent.click(await screen.findByRole("button", { name: "Connect an agent" }));
    next();
    expect(screen.getByText("Choose where you'll use the agent.")).toBeTruthy();

    fireEvent.click(screen.getByRole("radio", { name: /Claude Code/ }));
    next();
    expect(screen.getByRole("heading", { name: "Name your agent" })).toBeTruthy();
    next();
    expect(screen.getByText("Give the agent a name.")).toBeTruthy();

    fireEvent.change(screen.getByRole("textbox", { name: "Name" }), {
      target: { value: " Builder " },
    });
    fireEvent.click(screen.getByRole("radio", { name: "Brain" }));
    next();

    fireEvent.click(screen.getByRole("checkbox", { name: "General" }));
    fireEvent.click(screen.getByRole("checkbox", { name: /React/ }));
    next();

    expect(screen.getByRole("heading", { name: "Create its token" })).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Create agent" }));

    await screen.findByRole("heading", { name: "Connect Builder" });
    expect(create).toHaveBeenCalledWith({
      name: "Builder",
      iconName: "brain",
      provider: "anthropic",
      runtime: "claude-code",
      roomIds: [1],
      capabilities: ["read_messages", "post_messages", "manage_threads"],
      tokenName: null,
      expiresAt: null,
    });
    expect(screen.getByText(SECRET)).toBeTruthy();
    expect(
      screen.getByText(/claude mcp add --transport http --scope user smartfire-builder/),
    ).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Done" }));
    expect(await screen.findByRole("article", { name: "Builder" })).toBeTruthy();
    expect(screen.queryByText(SECRET)).toBeNull();
  });

  it("sends no abilities for an agent with no rooms", async () => {
    await mount(list({ agents: [] }));

    const create = vi.spyOn(meAgents, "create").mockResolvedValue({ agent: AGENT, secret: SECRET });

    fireEvent.click(await screen.findByRole("button", { name: "Connect an agent" }));
    fireEvent.click(screen.getByRole("radio", { name: /Other MCP/ }));
    next();
    fireEvent.change(screen.getByRole("textbox", { name: "Name" }), { target: { value: "Solo" } });
    next();
    next();
    fireEvent.click(screen.getByRole("button", { name: "Create agent" }));

    await waitFor(() => expect(create).toHaveBeenCalled());
    expect(create.mock.calls[0]?.[0]).toMatchObject({ roomIds: [], capabilities: [] });
  });
});

describe("SecretBlock", () => {
  it("shows the token but no setup when the server has no public address", () => {
    render(<SecretBlock secret={SECRET} client="codex" agentName="Helper" mcpUrl={null} />);

    expect(screen.getByText(SECRET)).toBeTruthy();
    expect(screen.getByText(/public address isn't configured yet/)).toBeTruthy();
    expect(screen.queryByText(/mcp_servers/)).toBeNull();
  });
});
