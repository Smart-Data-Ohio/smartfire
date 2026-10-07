import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  RouterProvider,
} from "@tanstack/react-router";
import { act, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { userFixture } from "../../api/testing.ts";
import type { AgentDirectoryRow } from "../../gen/AgentDirectoryRow.ts";
import type { AgentProfile } from "../../gen/AgentProfile.ts";
import type { AgentStep } from "../../gen/AgentStep.ts";
import type { User } from "../../gen/User.ts";
import { initialState } from "../../store/state.ts";
import { mutations, store } from "../../store/store.ts";
import { AgentBadgeFor } from "../people/agent-badge.tsx";
import { agentTone, identityOf } from "../people/agent-identity.ts";
import { filterAgents } from "./agent-directory-page.tsx";
import {
  budgetText,
  capabilityLabel,
  grantsLines,
  kindDescription,
  providerLine,
} from "./agent-format.ts";
import { AgentProfileContent } from "./agent-profile-page.tsx";
import { formatDuration, MessageSteps, summarizeSteps } from "./message-steps.tsx";
import { workingSentence } from "./working.ts";

const NOW = Date.parse("2026-10-06T12:00:00.000Z");

function agent(id: number, name: string, change: Partial<NonNullable<User["agent"]>> = {}): User {
  return {
    ...userFixture(id, name),
    role: "bot",
    agent: { agentId: id, kind: "workspace", status: "idle", suspended: false, ...change },
  };
}

function step(id: number, status: string, change: Partial<AgentStep> = {}): AgentStep {
  return {
    id,
    messageId: 1,
    threadId: null,
    name: `Step ${id}`,
    // SAFETY: the wire passes unknown statuses through; the tests feed one on purpose.
    status: status as AgentStep["status"],
    inputSummary: null,
    outputSummary: null,
    durationMs: null,
    position: id,
    createdAt: new Date(NOW).toISOString(),
    updatedAt: new Date(NOW).toISOString(),
    ...change,
  };
}

function row(agentId: number, change: Partial<AgentDirectoryRow> = {}): AgentDirectoryRow {
  return {
    agentId,
    userId: agentId,
    kind: "workspace",
    ownerId: 2,
    status: "idle",
    statusNote: null,
    suspended: false,
    createdAt: new Date(NOW - 86_400_000).toISOString(),
    statusChangedAt: null,
    lastSeenAt: null,
    ...change,
  };
}

/** `children` inside a small router with the routes the agent pages link to. */
async function inRouter(children: ReactNode) {
  const rootRoute = createRootRoute({ component: () => children });

  const routes = ["/r/$roomId", "/agents", "/agents/$agentId"].map((path) =>
    createRoute({ getParentRoute: () => rootRoute, path }),
  );

  const router = createRouter({
    routeTree: rootRoute.addChildren(routes),
    history: createMemoryHistory({ initialEntries: ["/agents/40"] }),
  });

  render(<RouterProvider router={router} />);
  await act(() => router.load());

  return router;
}

beforeEach(() => {
  // jsdom has no matchMedia; the motion helpers ask it about reduced motion.
  window.matchMedia = (query: string) => {
    const list = new EventTarget();

    return Object.assign(list, {
      matches: false,
      media: query,
      onchange: null,
      addListener: () => undefined,
      removeListener: () => undefined,
    });
  };
});

afterEach(() => store.setState(initialState, true));

describe("the agent badge", () => {
  it("is violet AGENT with its kind for an agent, and nothing for a person", () => {
    const { container, rerender } = render(<AgentBadgeFor user={agent(40, "Ada")} />);

    expect(container.textContent).toBe("Agent (workspace agent)");
    expect(container.querySelector(".agent-badge")?.getAttribute("title")).toBe("Workspace agent");

    rerender(<AgentBadgeFor user={userFixture(2)} />);
    expect(container.textContent).toBe("");
  });

  it("falls back to the plain badge for a kind it doesn't know, and says Bot without an agent row", () => {
    // SAFETY: an unknown kind as the tolerant wire would pass it through.
    const odd = agent(40, "Ada", { kind: "fleet" as "workspace" });
    const { container, rerender } = render(<AgentBadgeFor user={odd} />);

    expect(container.textContent).toBe("Agent");

    rerender(<AgentBadgeFor user={{ ...odd, agent: null }} />);
    expect(container.textContent).toBe("Bot");
  });

  it("always shows suspension and deactivation, and the live status only when asked", () => {
    const { container, rerender } = render(
      <AgentBadgeFor user={agent(40, "Ada", { suspended: true, status: "working" })} />,
    );

    expect(container.textContent).toContain("Suspended");

    rerender(<AgentBadgeFor user={{ ...agent(40, "Ada"), status: "deactivated" }} />);
    expect(container.textContent).toContain("Deactivated");

    rerender(<AgentBadgeFor user={agent(40, "Ada", { status: "working" })} />);
    expect(container.textContent).not.toContain("Working");

    rerender(<AgentBadgeFor user={agent(40, "Ada", { status: "working" })} status />);
    expect(container.textContent).toContain("Working");
  });

  it("ranks deactivation over suspension over status", () => {
    const suspended = agent(40, "Ada", { suspended: true, status: "failed" });

    expect(agentTone(identityOf(suspended))).toBe("suspended");
    expect(agentTone(identityOf({ ...suspended, status: "banned" }))).toBe("inactive");
    expect(agentTone(identityOf(agent(40, "Ada", { status: "failed" })))).toBe("failed");
    expect(agentTone(identityOf(userFixture(2)))).toBeNull();
  });
});

describe("steps on an agent message", () => {
  it("summarises the count, the running step and the failures", () => {
    const running = summarizeSteps([step(1, "done"), step(2, "running"), step(3, "pending")]);

    expect(running).toMatchObject({ count: "3 steps", failed: 0, totalMs: null });
    expect(running.running?.name).toBe("Step 2");

    const settled = summarizeSteps([
      step(1, "done", { durationMs: 400 }),
      step(2, "failed", { durationMs: 1800 }),
    ]);

    expect(settled).toMatchObject({ running: null, failed: 1, totalMs: 2200 });
  });

  it("leads with pending until every step has settled", () => {
    const { container, rerender } = render(
      <MessageSteps steps={[step(1, "pending"), step(2, "pending")]} />,
    );

    const lead = () =>
      container.querySelector(".steps-summary .step-icon")?.getAttribute("data-status");

    expect(summarizeSteps([step(1, "pending")]).settled).toBe(false);
    expect(lead()).toBe("pending");

    rerender(<MessageSteps steps={[step(1, "done"), step(2, "pending")]} />);
    expect(lead()).toBe("pending");

    rerender(<MessageSteps steps={[step(1, "done"), step(2, "done")]} />);
    expect(lead()).toBe("done");

    rerender(<MessageSteps steps={[step(1, "done"), step(2, "failed")]} />);
    expect(lead()).toBe("failed");
  });

  it("formats durations in ms below a second, else seconds", () => {
    expect(formatDuration(420)).toBe("420 ms");
    expect(formatDuration(4200)).toBe("4.2 s");
    expect(formatDuration(42_000)).toBe("42 s");
  });

  it("shows the running step live, and lists every step with In and Out when opened", async () => {
    const user = userEvent.setup();

    const { rerender } = render(
      <MessageSteps
        steps={[
          step(1, "done", { durationMs: 900, inputSummary: "pnpm check", outputSummary: "ok" }),
          step(2, "running"),
          step(3, "mystery"),
        ]}
      />,
    );

    const toggle = screen.getByRole("button", { name: /3 steps/ });

    expect(toggle.textContent).toContain("Step 2");
    expect(toggle.getAttribute("aria-expanded")).toBe("false");

    await user.click(toggle);
    expect(toggle.getAttribute("aria-expanded")).toBe("true");

    const list = screen.getByRole("list", { name: "Steps" });
    const items = within(list).getAllByRole("listitem");

    expect(items).toHaveLength(3);
    expect(items[0]?.textContent).toContain("In");
    expect(items[0]?.textContent).toContain("pnpm check");
    expect(items[2]?.textContent).toContain("Mystery");

    rerender(<MessageSteps steps={[]} />);
    expect(screen.queryByRole("button", { name: /steps/ })).toBeNull();
  });
});

describe("the agent pages' words", () => {
  it("words the kind and owner as the classic row does", () => {
    expect(kindDescription("personal", "Theo")).toBe("Personal agent of Theo");
    expect(kindDescription("workspace", "Riel")).toBe("Workspace agent, managed by Riel");
    expect(kindDescription("workspace", null)).toBe("Workspace agent, no owner recorded");
    expect(kindDescription("fleet", null)).toBe("Agent, no owner recorded");
  });

  it("words grants, budgets and the provider", () => {
    expect(
      grantsLines({
        legacy: false,
        grants: [
          { capability: "react", workspaceWide: true, roomCount: 0 },
          { capability: "post_messages", workspaceWide: false, roomCount: 3 },
        ],
      }),
    ).toEqual(["React workspace-wide", "Post messages in 3 rooms"]);
    expect(grantsLines({ legacy: false, grants: [] })).toEqual(["No active grants"]);
    expect(grantsLines({ legacy: true, grants: [] })[0]).toMatch(/^Legacy access/);
    // SAFETY: an unknown capability as the tolerant wire would pass it through.
    expect(capabilityLabel("summon_dragons" as "react")).toBe("Summon dragons");
    expect(budgetText({ cap: "board_posts", used: 4, limit: 5 })).toBe("4/5 board posts");
    expect(budgetText({ cap: "messages", used: 7, limit: null })).toBe("7 messages");
    expect(providerLine("Anthropic", "claude-agent-sdk")).toBe("Anthropic · claude-agent-sdk");
    expect(providerLine(null, " ")).toBeNull();
    expect(workingSentence(["Ember"])).toBe("Ember is working");
    expect(workingSentence(["Ember", "Scout"])).toBe("Ember and Scout are working");
  });

  it("filters the directory by kind and by the agent's or owner's name, accents folded", () => {
    const users = {
      40: agent(40, "Zoë"),
      41: agent(41, "Scout", { kind: "personal" }),
      2: userFixture(2, "Theo"),
    };

    const rows = [row(40), row(41, { kind: "personal" })];

    expect(filterAgents(rows, users, "all", "zoe").map((each) => each.agentId)).toEqual([40]);
    expect(filterAgents(rows, users, "personal", "").map((each) => each.agentId)).toEqual([41]);
    expect(filterAgents(rows, users, "all", "theo")).toHaveLength(2);
  });
});

describe("the agent profile", () => {
  const profile: AgentProfile = {
    agent: row(40, { statusNote: "Reviewing the deploy", status: "working" }),
    provider: "Anthropic",
    runtime: null,
    description: "Runs the release checklist.",
    rooms: [{ roomId: 3, name: "engineering" }],
    hiddenRoomCount: 2,
    grants: null,
    management: null,
    users: [agent(40, "Atlas", { status: "working" }), userFixture(2, "Priya")],
  };

  it("shows the header, rooms and a note in place of what only managers see", async () => {
    mutations.mergeUsers(profile.users);
    await inRouter(<AgentProfileContent profile={profile} now={NOW} />);

    expect(screen.getByRole("heading", { name: "Atlas" })).toBeDefined();
    expect(screen.getByText("Workspace agent, managed by Priya")).toBeDefined();
    expect(screen.getByText(/Reviewing the deploy/)).toBeDefined();
    expect(screen.getByText("Anthropic")).toBeDefined();
    expect(screen.getByRole("link", { name: "engineering" }).getAttribute("href")).toBe("/r/3");
    expect(screen.getByText(/2 rooms you're not in/)).toBeDefined();
    expect(screen.queryByText("Capabilities")).toBeNull();
    expect(screen.getByText(/shown to administrators and its owner/)).toBeDefined();
    expect(screen.getByRole("button", { name: "Message" })).toBeDefined();
  });

  it("shows grants, usage and the last 24 hours to managers", async () => {
    mutations.mergeUsers(profile.users);
    await inRouter(
      <AgentProfileContent
        profile={{
          ...profile,
          grants: {
            legacy: false,
            grants: [{ capability: "react", workspaceWide: true, roomCount: 0 }],
          },
          management: {
            activitySummary: { delivered: 12, acknowledged: 9, posted: 4, suppressed: 1 },
            budgetUsage: [{ cap: "messages", used: 40, limit: 50 }],
          },
        }}
        now={NOW}
      />,
    );

    expect(screen.getByText("React workspace-wide")).toBeDefined();
    expect(screen.getByRole("meter", { name: "40/50 messages" })).toBeDefined();
    expect(screen.getAllByRole("definition").map((each) => each.textContent)).toEqual([
      "12",
      "9",
      "4",
      "1",
    ]);
    expect(
      screen.getByLabelText("12 delivered, 9 acknowledged, 4 posted, 1 suppressed"),
    ).toBeDefined();
  });
});
