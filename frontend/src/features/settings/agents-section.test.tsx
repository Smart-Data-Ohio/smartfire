import {
  createMemoryHistory,
  createRootRoute,
  createRouter,
  RouterProvider,
} from "@tanstack/react-router";
import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { PersonalAgent } from "../../gen/PersonalAgent.ts";
import type { PersonalAgentCredential } from "../../gen/PersonalAgentCredential.ts";
import type { PersonalAgentList } from "../../gen/PersonalAgentList.ts";
import { ActionError } from "../../sync/run.ts";
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
  connection: null,
};

const AGENT: PersonalAgent = {
  id: 7,
  userId: 70,
  name: "Helper",
  iconName: "owl",
  iconUrl: null,
  provider: "openai",
  runtime: "codex",
  description: null,
  dailyMessageCap: 200,
  suspended: false,
  roomIds: [2],
  capabilities: ["post_messages", "read_messages"],
  lastUsedAt: null,
  createdAt: "2026-10-13T12:00:00.000Z",
  credentials: [TOKEN],
};

function list(change: Partial<PersonalAgentList> = {}): PersonalAgentList {
  return {
    agents: [AGENT],
    enabled: true,
    limit: 3,
    capabilities: ["read_messages", "post_messages", "react", "manage_threads"],
    rooms: [
      { id: 1, name: "General", private: false },
      { id: 2, name: "Design", private: false },
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

    expect(within(card).getByText(/^Codex CLI · added Oct 13, 2026 · never used$/)).toBeTruthy();
    expect(within(card).getByText("Design")).toBeTruthy();
    expect(within(card).getByText("Post messages, Read messages")).toBeTruthy();
    expect(within(card).getByText("…beef")).toBeTruthy();
    expect(within(card).getByRole("button", { name: "Rotate Primary token" })).toBeTruthy();
    expect(within(card).getByRole("button", { name: "Revoke Primary token" })).toBeTruthy();
    expect(screen.getByText("1 of 3 agents")).toBeTruthy();
  });

  it("draws the bot's icon image when it has one, and its emoji otherwise", async () => {
    await mount(
      list({
        agents: [
          AGENT,
          { ...AGENT, id: 8, userId: 80, name: "Pictured", iconUrl: "/icons/claude" },
        ],
      }),
    );

    const pictured = await screen.findByRole("article", { name: "Pictured" });

    expect(pictured.querySelector("img.agent-avatar-icon")?.getAttribute("src")).toBe(
      "/icons/claude",
    );
    expect(within(screen.getByRole("article", { name: "Helper" })).getByText("🦉")).toBeTruthy();
  });

  it("explains when personal agents are off, keeping only revoke and remove", async () => {
    await mount(list({ enabled: false }));

    expect(await screen.findByText(/Personal agents are turned off/)).toBeTruthy();
    expect(screen.getByText(/Your agents stop working until it's turned back on/)).toBeTruthy();
    expect(screen.getByText("None while personal agents are off")).toBeTruthy();
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

  it("still connects an app that signs in at the limit, and offers nothing that makes an agent", async () => {
    await mount(list({ limit: 1, oauthAvailable: true }));

    const create = vi.spyOn(meAgents, "create");
    const connect = await screen.findByRole("button", { name: "Connect an agent" });

    expect(connect.hasAttribute("disabled")).toBe(false);
    expect(screen.getByText(/Apps that sign in can still connect as one of them/)).toBeTruthy();

    fireEvent.click(connect);

    expect(screen.getAllByRole("radio").map((radio) => radio.getAttribute("value"))).toEqual([
      "claude-ai",
      "chatgpt",
      "grok",
      "claude-code",
      "codex",
    ]);
    expect(screen.getByText(/only apps that sign in are listed/)).toBeTruthy();

    // Claude Code can only sign in here: no token choice, and no naming or access steps.
    fireEvent.click(screen.getByRole("radio", { name: /Claude Code/ }));
    expect(screen.queryByRole("radio", { name: "With a token" })).toBeNull();
    next();

    expect(await screen.findByRole("heading", { name: "Connect Claude Code" })).toBeTruthy();
    expect(screen.getByText("/mcp")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Done" }));
    expect(create).not.toHaveBeenCalled();
  });

  it("drops the removed agent by its own id, not the bot user id the answer names", async () => {
    await mount(list({ agents: [AGENT, { ...AGENT, id: 70, userId: 700, name: "Other" }] }));

    const remove = vi.spyOn(meAgents, "remove").mockResolvedValue({ id: AGENT.userId });

    fireEvent.click(await screen.findByRole("button", { name: "Remove Helper" }));
    fireEvent.click(
      within(screen.getByRole("alertdialog")).getByRole("button", { name: "Remove agent" }),
    );

    await waitFor(() => expect(screen.queryByRole("article", { name: "Helper" })).toBeNull());
    expect(remove).toHaveBeenCalledWith(7);
    expect(screen.getByRole("article", { name: "Other" })).toBeTruthy();
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

const CONNECTION: PersonalAgentCredential = {
  id: 21,
  kind: "oauth",
  name: "Claude",
  lastFour: "oauth",
  createdAt: "2026-10-13T12:00:00.000Z",
  expiresAt: "2027-04-11T12:00:00.000Z",
  lastUsedAt: null,
  lastUsedIp: null,
  state: "active",
  connection: {
    clientName: "Claude",
    clientHost: "claude.ai",
    clientId: "https://claude.ai/oauth/mcp-client",
    clientBlocked: false,
    revokedReason: null,
  },
};

const REUSED: PersonalAgentCredential = {
  ...CONNECTION,
  id: 22,
  name: "Codex",
  state: "revoked",
  connection: {
    clientName: "Codex",
    clientHost: "openai.com",
    clientId: "https://openai.com/codex/client.json",
    clientBlocked: false,
    revokedReason: "tokenReused",
  },
};

describe("connections", () => {
  it("lists an agent's connections apart from its tokens, with their dates", async () => {
    await mount(
      list({ oauthAvailable: true, agents: [{ ...AGENT, credentials: [CONNECTION, TOKEN] }] }),
    );

    const card = await screen.findByRole("article", { name: "Helper" });
    const connections = within(card).getByRole("region", { name: "Helper's connections" });
    const tokens = within(card).getByRole("region", { name: "Helper's tokens" });

    expect(within(connections).getByText("Claude")).toBeTruthy();
    expect(
      within(connections).getByText(/^Connected .+2026 · expires .+2027 · never used$/),
    ).toBeTruthy();
    expect(within(connections).getByRole("button", { name: "Revoke Claude" })).toBeTruthy();
    expect(within(connections).queryByRole("button", { name: /Rotate/ })).toBeNull();
    expect(within(connections).queryByRole("button", { name: /Reconnect/ })).toBeNull();
    expect(within(tokens).getByText("…beef")).toBeTruthy();
    expect(within(tokens).queryByText("Claude")).toBeNull();
  });

  it("says no token is needed when it only connects by signing in", async () => {
    await mount(list({ oauthAvailable: true, agents: [{ ...AGENT, credentials: [CONNECTION] }] }));

    expect(await screen.findByText("No tokens. Its connections sign in instead.")).toBeTruthy();
  });

  it("explains a connection ended for a reused sign-in token, and offers Reconnect", async () => {
    await mount(list({ oauthAvailable: true, agents: [{ ...AGENT, credentials: [REUSED] }] }));

    const connections = await screen.findByRole("region", { name: "Helper's connections" });

    expect(
      within(connections).getByText(
        "Disconnected because an old sign-in token was reused. Reconnect to continue.",
      ),
    ).toBeTruthy();
    expect(within(connections).getByText("Disconnected")).toBeTruthy();
    expect(within(connections).queryByRole("button", { name: "Revoke Codex" })).toBeNull();

    fireEvent.click(within(connections).getByRole("button", { name: "Reconnect Codex" }));

    const dialog = await screen.findByRole("dialog", { name: "Reconnect Codex" });

    expect(within(dialog).getByText("https://chat.example.com/agents/mcp")).toBeTruthy();
    expect(within(dialog).getByText(/choose Helper/)).toBeTruthy();
  });

  it("gives a product known by its exact client id its own steps when reconnecting it", async () => {
    const chatgpt: PersonalAgentCredential = {
      ...CONNECTION,
      name: "ChatGPT",
      state: "expired",
      connection: {
        clientName: "ChatGPT",
        clientHost: "chatgpt.com",
        clientId: "https://chatgpt.com/oauth/client.json",
        clientBlocked: false,
        revokedReason: null,
      },
    };

    await mount(list({ oauthAvailable: true, agents: [{ ...AGENT, credentials: [chatgpt] }] }));

    expect(await screen.findByText(/Connections last 180 days/)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Reconnect ChatGPT" }));

    const dialog = await screen.findByRole("dialog", { name: "Reconnect ChatGPT" });

    expect(within(dialog).getByText(/In ChatGPT, open Settings → Apps & Connectors/)).toBeTruthy();
  });

  it("gives general steps for a client it doesn't know exactly, even on a known host", async () => {
    await mount(
      list({
        oauthAvailable: true,
        agents: [{ ...AGENT, credentials: [{ ...CONNECTION, state: "expired" }] }],
      }),
    );

    fireEvent.click(await screen.findByRole("button", { name: "Reconnect Claude" }));

    const dialog = await screen.findByRole("dialog", { name: "Reconnect Claude" });

    expect(
      within(dialog).getByText(/Sign in again from the app you connected \(Claude\)/),
    ).toBeTruthy();
    expect(within(dialog).queryByText(/Settings → Connectors/)).toBeNull();
    expect(within(dialog).getByText("https://chat.example.com/agents/mcp")).toBeTruthy();
  });

  it("offers no Reconnect without OAuth, or for an app an administrator blocked", async () => {
    const blocked: PersonalAgentCredential = {
      ...CONNECTION,
      id: 23,
      connection: {
        clientName: "Grok",
        clientHost: "grok.com",
        clientId: "https://grok.com/oauth/client.json",
        clientBlocked: true,
        revokedReason: null,
      },
      name: "Grok",
    };

    await mount(
      list({ oauthAvailable: false, agents: [{ ...AGENT, credentials: [REUSED, blocked] }] }),
    );

    const connections = await screen.findByRole("region", { name: "Helper's connections" });

    expect(within(connections).queryByRole("button", { name: /Reconnect/ })).toBeNull();
    expect(within(connections).getByText(/administrator blocked this app/)).toBeTruthy();
    expect(within(connections).getByText("Blocked")).toBeTruthy();
  });

  it("revokes a connection after asking, replacing the agent's credentials", async () => {
    await mount(list({ oauthAvailable: true, agents: [{ ...AGENT, credentials: [CONNECTION] }] }));

    const revoke = vi.spyOn(meAgents, "revokeToken").mockResolvedValue({
      credentials: [{ ...CONNECTION, state: "revoked" }],
    });

    fireEvent.click(await screen.findByRole("button", { name: "Revoke Claude" }));

    const ask = await screen.findByRole("alertdialog", { name: "Revoke Claude?" });

    expect(within(ask).getByText(/Claude stops reaching Helper at once/)).toBeTruthy();
    fireEvent.click(within(ask).getByRole("button", { name: "Revoke" }));

    await waitFor(() => expect(revoke).toHaveBeenCalledWith(7, 21));
    expect(await screen.findByRole("button", { name: "Reconnect Claude" })).toBeTruthy();
    expect(screen.getByText("Revoked")).toBeTruthy();
  });
});

describe("the wizard, signing in", () => {
  it("gives claude.ai its connector URL and steps, and makes nothing", async () => {
    await mount(list({ agents: [], oauthAvailable: true }));

    const create = vi.spyOn(meAgents, "create");

    fireEvent.click(await screen.findByRole("button", { name: "Connect an agent" }));
    fireEvent.click(screen.getByRole("radio", { name: /claude\.ai/ }));
    next();

    expect(await screen.findByRole("heading", { name: "Connect claude.ai" })).toBeTruthy();
    expect(screen.getByText("https://chat.example.com/agents/mcp")).toBeTruthy();
    expect(screen.getByText(/There's no token to keep/)).toBeTruthy();
    expect(
      screen.getAllByRole("listitem").some((item) => item.textContent?.includes("Identity")),
    ).toBe(false);

    fireEvent.click(screen.getByRole("button", { name: "Done" }));

    expect(screen.queryByRole("heading", { name: "Connect claude.ai" })).toBeNull();
    expect(create).not.toHaveBeenCalled();
  });

  it("signs Claude Code in without a token by default, or makes one when asked", async () => {
    await mount(list({ agents: [], oauthAvailable: true }));

    fireEvent.click(await screen.findByRole("button", { name: "Connect an agent" }));
    fireEvent.click(screen.getByRole("radio", { name: /Claude Code/ }));

    expect(screen.getByRole("radio", { name: "Sign in, no token" })).toHaveProperty(
      "checked",
      true,
    );
    next();

    expect(
      await screen.findByText(
        "claude mcp add --transport http --scope user smartfire https://chat.example.com/agents/mcp",
      ),
    ).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Back" }));
    fireEvent.click(screen.getByRole("radio", { name: "With a token" }));
    next();

    expect(await screen.findByRole("textbox", { name: /Name/ })).toBeTruthy();
  });

  it("gives Codex its config and login command", async () => {
    await mount(list({ agents: [], oauthAvailable: true }));

    fireEvent.click(await screen.findByRole("button", { name: "Connect an agent" }));
    fireEvent.click(screen.getByRole("radio", { name: /Codex/ }));
    next();

    expect(await screen.findByText("codex mcp login smartfire")).toBeTruthy();
    expect(screen.getByText(/url = "https:\/\/chat\.example\.com\/agents\/mcp"/)).toBeTruthy();
  });

  it("offers no sign-in choice without OAuth", async () => {
    await mount(list({ agents: [] }));

    fireEvent.click(await screen.findByRole("button", { name: "Connect an agent" }));
    fireEvent.click(screen.getByRole("radio", { name: /Claude Code/ }));

    expect(screen.queryByRole("radio", { name: "Sign in, no token" })).toBeNull();
  });
});

/** A promise settled by hand, for answers that arrive in a chosen order. */
function deferred<T>() {
  const settle = {
    resolve: (_value: T): void => undefined,
    reject: (_error: Error): void => undefined,
  };

  const promise = new Promise<T>((resolve, reject) => {
    settle.resolve = resolve;
    settle.reject = reject;
  });

  return { promise, settle };
}

const UNCONFIRMED = /Couldn't confirm the save/;

const ROOM_REFUSED = new ActionError("Validation", "Invalid", {
  roomIds: ["includes a room you're not in"],
});

const editDialog = () => screen.getByRole("dialog");

const button = (name: string) => within(editDialog()).getByRole("button", { name });

const nameField = () => within(editDialog()).getByRole("textbox", { name: "Name" });

function rename(to: string) {
  fireEvent.change(nameField(), { target: { value: to } });
}

/**
 * Opens Helper's edit dialog, renames it and saves through the real (bounded) request, whose
 * answer never comes; runs the clock past the bound. The reload that follows answers `reload`.
 */
async function timeOutRename(reload: Promise<PersonalAgentList>) {
  vi.spyOn(globalThis, "fetch").mockReturnValue(new Promise<Response>(() => undefined));

  const edit = await screen.findByRole("button", { name: "Edit Helper" });

  vi.useFakeTimers();

  try {
    fireEvent.click(edit);
    rename("Renamed");
    vi.spyOn(meAgents, "list").mockReturnValue(reload);
    fireEvent.click(button("Save"));
    await act(() => vi.advanceTimersByTimeAsync(1_000));
    expect(button("Cancel").hasAttribute("disabled")).toBe(true);
    await act(() => vi.advanceTimersByTimeAsync(15_000));
  } finally {
    vi.useRealTimers();
  }

  expect(await within(editDialog()).findByText(UNCONFIRMED)).toBeTruthy();
}

describe("editing an agent", () => {
  it("shows a rename that took even when the access save fails, and keeps it on retry", async () => {
    await mount(list());

    const update = vi.spyOn(meAgents, "update").mockResolvedValue({ ...AGENT, name: "Renamed" });

    const setAccess = vi
      .spyOn(meAgents, "setAccess")
      .mockRejectedValueOnce(ROOM_REFUSED)
      .mockResolvedValueOnce({ ...AGENT, name: "Renamed", roomIds: [], capabilities: [] });

    fireEvent.click(await screen.findByRole("button", { name: "Edit Helper" }));
    rename("Renamed");
    fireEvent.click(within(editDialog()).getByRole("checkbox", { name: "General" }));
    fireEvent.click(button("Save"));

    expect(await within(editDialog()).findByText("includes a room you're not in")).toBeTruthy();
    expect(screen.getByRole("article", { name: "Renamed" })).toBeTruthy();

    // Closed, reopened, the room taken away: the rename isn't sent again, nor undone.
    fireEvent.click(button("Cancel"));
    fireEvent.click(await screen.findByRole("button", { name: "Edit Renamed" }));
    expect(nameField()).toHaveProperty("value", "Renamed");
    fireEvent.click(within(editDialog()).getByRole("checkbox", { name: "Design" }));
    fireEvent.click(button("Save"));

    await waitFor(() => expect(setAccess).toHaveBeenCalledTimes(2));
    expect(update).toHaveBeenCalledTimes(1);
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    expect(screen.getByRole("article", { name: "Renamed" })).toBeTruthy();
  });

  it("can't be closed while a rename is out, so a retry never undoes it", async () => {
    await mount(list());

    const answer = deferred<PersonalAgent>();
    const update = vi.spyOn(meAgents, "update").mockReturnValue(answer.promise);

    const setAccess = vi
      .spyOn(meAgents, "setAccess")
      .mockRejectedValueOnce(ROOM_REFUSED)
      .mockResolvedValueOnce({ ...AGENT, name: "Renamed", roomIds: [], capabilities: [] });

    fireEvent.click(await screen.findByRole("button", { name: "Edit Helper" }));

    const dialog = editDialog();

    rename("Renamed");
    fireEvent.click(within(dialog).getByRole("checkbox", { name: "General" }));
    fireEvent.click(button("Save"));
    await waitFor(() => expect(update).toHaveBeenCalledTimes(1));

    // The rename is out: neither Cancel nor Escape closes this opening for a fresh one.
    expect(button("Cancel").hasAttribute("disabled")).toBe(true);
    fireEvent.click(button("Cancel"));
    fireEvent.keyDown(dialog, { key: "Escape" });
    expect(editDialog()).toBe(dialog);

    // The rename lands, the access save is refused, and the corrected retry keeps the name.
    await act(async () => answer.settle.resolve({ ...AGENT, name: "Renamed" }));
    expect(await within(dialog).findByText("includes a room you're not in")).toBeTruthy();
    expect(screen.getByRole("article", { name: "Renamed" })).toBeTruthy();

    fireEvent.click(within(dialog).getByRole("checkbox", { name: "Design" }));
    fireEvent.click(button("Save"));

    await waitFor(() => expect(setAccess).toHaveBeenCalledTimes(2));
    expect(update).toHaveBeenCalledTimes(1);
    expect(update).toHaveBeenCalledWith(7, { name: "Renamed", iconName: null });
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    expect(screen.getByRole("article", { name: "Renamed" })).toBeTruthy();
  });

  it("holds Save while the reload after an unanswered save is out, then shows the server's", async () => {
    await mount(list());

    const reload = deferred<PersonalAgentList>();

    await timeOutRename(reload.promise);

    // Closable at once, but not savable until the server has said what it holds.
    expect(button("Cancel").hasAttribute("disabled")).toBe(false);
    expect(button("Save").hasAttribute("disabled")).toBe(true);

    await act(async () => reload.settle.resolve(list({ agents: [{ ...AGENT, name: "Renamed" }] })));

    await waitFor(() => expect(button("Save").hasAttribute("disabled")).toBe(false));
    expect(nameField()).toHaveProperty("value", "Renamed");
    expect(screen.getByRole("article", { name: "Renamed" })).toBeTruthy();

    // The form is the server's again: saving sends nothing at all.
    const update = vi.spyOn(meAgents, "update");
    const setAccess = vi.spyOn(meAgents, "setAccess");

    fireEvent.click(button("Save"));
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    expect(update).not.toHaveBeenCalled();
    expect(setAccess).not.toHaveBeenCalled();
  });

  it("can be closed once an unanswered save ends", async () => {
    await mount(list());
    vi.spyOn(globalThis, "fetch").mockReturnValue(new Promise<Response>(() => undefined));

    const edit = await screen.findByRole("button", { name: "Edit Helper" });

    vi.useFakeTimers();

    try {
      fireEvent.click(edit);
      fireEvent.click(screen.getByRole("checkbox", { name: "General" }));
      fireEvent.click(screen.getByRole("button", { name: "Save" }));
      await act(() => vi.advanceTimersByTimeAsync(15_000));
    } finally {
      vi.useRealTimers();
    }

    expect(await screen.findByText(UNCONFIRMED)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
  });

  it("never lets an older reload put back a name saved after it was sent", async () => {
    await mount(list());

    const reload = deferred<PersonalAgentList>();

    await timeOutRename(reload.promise);

    // Closed while the reload is out, reopened, and renamed again: that one lands.
    fireEvent.click(button("Cancel"));
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    vi.spyOn(meAgents, "update").mockResolvedValue({ ...AGENT, name: "Third" });
    fireEvent.click(screen.getByRole("button", { name: "Edit Helper" }));
    rename("Third");
    fireEvent.click(button("Save"));
    expect(await screen.findByRole("article", { name: "Third" })).toBeTruthy();

    // The first reload, sent before that rename, answers last with the name it found then.
    await act(async () => reload.settle.resolve(list({ agents: [{ ...AGENT, name: "Renamed" }] })));
    expect(screen.getByRole("article", { name: "Third" })).toBeTruthy();
    expect(screen.queryByRole("article", { name: "Renamed" })).toBeNull();
  });

  it("never lets the reload reset a name typed after the unanswered save", async () => {
    await mount(list());

    const reload = deferred<PersonalAgentList>();

    await timeOutRename(reload.promise);

    // Typed while the reload is out; it answers with the first rename, which did land.
    rename("Third");
    await act(async () => reload.settle.resolve(list({ agents: [{ ...AGENT, name: "Renamed" }] })));
    await waitFor(() => expect(button("Save").hasAttribute("disabled")).toBe(false));
    expect(nameField()).toHaveProperty("value", "Third");

    // That rename lands; the access save beside it is refused.
    const update = vi
      .spyOn(meAgents, "update")
      .mockResolvedValueOnce({ ...AGENT, name: "Third" })
      .mockResolvedValueOnce({ ...AGENT, name: "Third", iconName: "brain" });

    vi.spyOn(meAgents, "setAccess").mockRejectedValueOnce(ROOM_REFUSED);
    fireEvent.click(within(editDialog()).getByRole("checkbox", { name: "General" }));
    fireEvent.click(button("Save"));
    expect(await within(editDialog()).findByText("includes a room you're not in")).toBeTruthy();
    expect(update).toHaveBeenLastCalledWith(7, { name: "Third", iconName: null });
    expect(screen.getByRole("article", { name: "Third" })).toBeTruthy();

    // Only the icon changes now: the name isn't sent, so nothing can put an older one back.
    fireEvent.click(within(editDialog()).getByRole("checkbox", { name: "General" }));
    fireEvent.click(within(editDialog()).getByRole("radio", { name: "Brain" }));
    vi.spyOn(meAgents, "setAccess").mockResolvedValueOnce({ ...AGENT, name: "Third" });
    fireEvent.click(button("Save"));
    await waitFor(() => expect(update).toHaveBeenCalledTimes(2));
    expect(update).toHaveBeenLastCalledWith(7, { name: null, iconName: "brain" });
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    expect(screen.getByRole("article", { name: "Third" })).toBeTruthy();
  });

  it("sends a name typed back after an unanswered save whose reload failed", async () => {
    await mount(list());

    const reload = deferred<PersonalAgentList>();

    await timeOutRename(reload.promise);
    await act(async () => reload.settle.reject(new Error("offline")));
    await waitFor(() => expect(button("Save").hasAttribute("disabled")).toBe(false));

    // The cache still says Helper, but the server may hold Renamed: Helper is sent, not skipped.
    const update = vi.spyOn(meAgents, "update").mockResolvedValue(AGENT);

    rename("Helper");
    fireEvent.click(button("Save"));
    await waitFor(() => expect(update).toHaveBeenCalledWith(7, { name: "Helper", iconName: null }));
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
  });

  it("still sends a name typed back after closing and reopening, until a later answer lands", async () => {
    await mount(list());

    const reload = deferred<PersonalAgentList>();

    await timeOutRename(reload.promise);
    await act(async () => reload.settle.reject(new Error("offline")));
    await waitFor(() => expect(button("Save").hasAttribute("disabled")).toBe(false));

    // Closed and reopened: the card, and the new opening, still know the save went unanswered.
    fireEvent.click(button("Cancel"));
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());

    const card = screen.getByRole("article", { name: "Helper" });

    expect(within(card).getByText(/Couldn't confirm the last save/)).toBeTruthy();
    expect(within(card).getByRole("button", { name: "Reload Helper" })).toBeTruthy();

    const update = vi.spyOn(meAgents, "update").mockResolvedValue(AGENT);

    // Typed over and back: the cache already says Helper, yet the server may hold Renamed.
    fireEvent.click(within(card).getByRole("button", { name: "Edit Helper" }));
    rename("Helpe");
    rename("Helper");
    fireEvent.click(button("Save"));
    await waitFor(() => expect(update).toHaveBeenCalledWith(7, { name: "Helper", iconName: null }));
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());

    // That answer says what the server holds: the note goes.
    expect(within(card).queryByText(/Couldn't confirm the last save/)).toBeNull();
  });

  it("drops the unconfirmed note once the card's Reload answers", async () => {
    await mount(list());

    const reload = deferred<PersonalAgentList>();

    await timeOutRename(reload.promise);
    await act(async () => reload.settle.reject(new Error("offline")));
    await waitFor(() => expect(button("Save").hasAttribute("disabled")).toBe(false));
    fireEvent.click(button("Cancel"));

    vi.spyOn(meAgents, "list").mockResolvedValue(list({ agents: [{ ...AGENT, name: "Renamed" }] }));
    fireEvent.click(screen.getByRole("button", { name: "Reload Helper" }));

    const card = await screen.findByRole("article", { name: "Renamed" });

    expect(within(card).queryByText(/Couldn't confirm the last save/)).toBeNull();
  });

  it("never lets a late answer from a closed opening close another agent's dialog", async () => {
    const other = { ...AGENT, id: 8, userId: 80, name: "Other" };

    await mount(list({ agents: [AGENT, other] }));

    const removal = deferred<{ id: number }>();
    const renamed = deferred<PersonalAgent>();

    vi.spyOn(meAgents, "remove").mockReturnValue(removal.promise);
    vi.spyOn(meAgents, "update").mockReturnValue(renamed.promise);

    // Helper's removal is out when its rename is saved.
    fireEvent.click(await screen.findByRole("button", { name: "Remove Helper" }));
    fireEvent.click(
      within(screen.getByRole("alertdialog")).getByRole("button", { name: "Remove agent" }),
    );
    fireEvent.click(screen.getByRole("button", { name: "Edit Helper" }));
    rename("Renamed");
    fireEvent.click(button("Save"));

    // The removal answers first: Helper's card and dialog go. Other's dialog opens and is typed in.
    await act(async () => removal.settle.resolve({ id: AGENT.userId }));
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    fireEvent.click(screen.getByRole("button", { name: "Edit Other" }));
    rename("Typed");

    // Helper's rename answers last: it changes nothing on screen.
    await act(async () => renamed.settle.resolve({ ...AGENT, name: "Renamed" }));
    expect(screen.getByRole("dialog", { name: "Edit Other" })).toBeTruthy();
    expect(nameField()).toHaveProperty("value", "Typed");
    expect(screen.queryByRole("article", { name: "Renamed" })).toBeNull();
    expect(screen.queryByRole("article", { name: "Helper" })).toBeNull();
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

  it("keeps the focus in the name field while it's typed, one key at a time", async () => {
    await mount(list({ agents: [] }));

    const user = userEvent.setup();

    await user.click(await screen.findByRole("button", { name: "Connect an agent" }));
    await user.click(screen.getByRole("radio", { name: /Codex CLI/ }));
    await user.click(screen.getByRole("button", { name: "Next" }));

    const field = screen.getByRole("textbox", { name: "Name" });

    await user.click(field);
    await user.type(field, "My helper");

    expect(document.activeElement).toBe(field);
    expect(field).toHaveProperty("value", "My helper");
  });

  it("marks private rooms and says the agent shows in their member lists", async () => {
    await mount(
      list({
        agents: [],
        rooms: [
          { id: 1, name: "General", private: false },
          { id: 3, name: "Launch", private: true },
        ],
      }),
    );
    fireEvent.click(await screen.findByRole("button", { name: "Connect an agent" }));
    fireEvent.click(screen.getByRole("radio", { name: /Codex CLI/ }));
    next();
    fireEvent.change(screen.getByRole("textbox", { name: "Name" }), { target: { value: "Solo" } });
    next();

    expect(screen.getByRole("checkbox", { name: "Launch Private" })).toBeTruthy();
    expect(screen.getByRole("checkbox", { name: "General" })).toBeTruthy();
    expect(screen.getByText(/everyone in it will see your agent in the member list/)).toBeTruthy();
  });

  /** Walks a fresh wizard to its last step and presses Create agent. */
  async function createSolo() {
    fireEvent.click(await screen.findByRole("button", { name: "Connect an agent" }));
    fireEvent.click(screen.getByRole("radio", { name: /Codex CLI/ }));
    next();
    fireEvent.change(screen.getByRole("textbox", { name: "Name" }), { target: { value: "Solo" } });
    next();
    next();
    fireEvent.click(screen.getByRole("button", { name: "Create agent" }));
  }

  it("shows the agent limit under the form, not on a field", async () => {
    await mount(list({ agents: [] }));
    vi.spyOn(meAgents, "create").mockRejectedValue(
      new ActionError("Validation", "Personal agent limit reached", {
        base: ["personal agent limit reached"],
      }),
    );

    await createSolo();

    expect(await screen.findByText("Personal agent limit reached.")).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Create its token" })).toBeTruthy();
  });

  it("says personal agents were turned off, and reloads the list", async () => {
    await mount(list({ agents: [] }));

    const create = vi
      .spyOn(meAgents, "create")
      .mockRejectedValue(
        new ActionError("Forbidden", "Personal agents are turned off for this workspace."),
      );

    vi.spyOn(meAgents, "list").mockResolvedValue(list({ agents: [], enabled: false }));
    await createSolo();
    await waitFor(() => expect(create).toHaveBeenCalled());

    expect(
      await screen.findByText("Personal agents are turned off for this workspace."),
    ).toBeTruthy();
    expect(await screen.findByRole("link", { name: "Workspace settings" })).toBeTruthy();
  });

  it("shows the token and the APP_URL note when the server has no public address", async () => {
    await mount(list({ agents: [], mcpUrl: null }));
    vi.spyOn(meAgents, "create").mockResolvedValue({ agent: AGENT, secret: SECRET });

    await createSolo();

    await screen.findByRole("heading", { name: "Connect Helper" });
    expect(screen.getByText(SECRET)).toBeTruthy();
    expect(screen.getByText(/administrator needs to set it \(APP_URL\)/)).toBeTruthy();
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
