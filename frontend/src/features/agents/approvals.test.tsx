import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  RouterProvider,
} from "@tanstack/react-router";
import { act, render, renderHook, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { userFixture } from "../../api/testing.ts";
import type { AgentApproval } from "../../gen/AgentApproval.ts";
import type { AgentLedgerEvent } from "../../gen/AgentLedgerEvent.ts";
import type { AgentLedgerEventType } from "../../gen/AgentLedgerEventType.ts";
import { approvalListKey } from "../../store/approvals.ts";
import { ledgerListKey } from "../../store/ledger.ts";
import { initialState } from "../../store/state.ts";
import { mutations, store } from "../../store/store.ts";
import { actions } from "../../sync/runtime.ts";
import { removeToast, toastSnapshot } from "../../ui/toast-store.ts";
import { AgentApprovals } from "./agent-approvals-tab.tsx";
import { useAgentLedger } from "./agent-hooks.ts";
import { ApprovalCard } from "./approval-card.tsx";
import {
  actsAsText,
  adminOnlyText,
  beamedApprovalId,
  decisionLine,
  shownStatus,
} from "./approval-format.ts";
import { liveDecisionAnnouncement, liveDecisions, nextWatched } from "./approval-live.ts";
import {
  externalLine,
  hopText,
  ledgerIcon,
  ledgerSentence,
  outcomeLabel,
  webhookLine,
} from "./ledger-format.ts";
import { LedgerRow } from "./ledger-row.tsx";

const NOW = Date.UTC(2026, 9, 6, 16, 30, 0);

const HOUR = 3_600_000;

function at(offset: number): string {
  return new Date(NOW + offset).toISOString();
}

function approval(change: Partial<AgentApproval> = {}): AgentApproval {
  return {
    id: 100,
    agentId: 9,
    agentUserId: 9,
    roomId: 3,
    roomName: "engineering",
    action: "deploy.production",
    summary: "Run deploy production for release 2.0.1",
    status: "pending",
    expiresAt: at(2 * 24 * HOUR),
    createdAt: at(-2 * HOUR),
    decidedById: null,
    decidedAt: null,
    decisionNote: null,
    githubLogin: null,
    fizzyUserName: null,
    adminOnly: false,
    approvable: true,
    deniable: true,
    updatedAt: "2026-10-06T09:00:00.000Z",
    ...change,
  };
}

function entry(change: Partial<AgentLedgerEvent> = {}): AgentLedgerEvent {
  return {
    id: 1,
    eventType: "mention",
    outcome: "delivered",
    createdAt: at(-HOUR),
    roomId: 3,
    roomName: "engineering",
    actorId: 2,
    messageId: 30_001,
    hop: 0,
    detail: null,
    webhookStatus: "none",
    webhookAttempts: 0,
    webhookLastError: null,
    external: null,
    handoffSummary: null,
    content: "Can you look at the deploy log?",
    ...change,
  };
}

async function inRouter(children: ReactNode) {
  const rootRoute = createRootRoute({ component: () => children });

  const routes = ["/r/$roomId", "/r/$roomId/m/$messageId"].map((path) =>
    createRoute({ getParentRoute: () => rootRoute, path }),
  );

  const router = createRouter({
    routeTree: rootRoute.addChildren(routes),
    history: createMemoryHistory({ initialEntries: ["/"] }),
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

afterEach(() => {
  store.setState(initialState, true);
  vi.restoreAllMocks();
  vi.unstubAllGlobals();

  for (const toast of toastSnapshot()) removeToast(toast.id);
});

describe("S4 client contracts", () => {
  it("shows a base decision-note refusal through the existing toast", async () => {
    const message = "Decision note is too long (maximum is 200 characters)";

    // Give the virtual list a viewport, since jsdom has no layout or ResizeObserver.
    vi.spyOn(HTMLElement.prototype, "offsetParent", "get").mockImplementation(function (
      this: HTMLElement,
    ) {
      return this.parentElement;
    });
    vi.stubGlobal(
      "ResizeObserver",
      class implements ResizeObserver {
        private readonly callback: ResizeObserverCallback;

        constructor(callback: ResizeObserverCallback) {
          this.callback = callback;
        }

        observe(target: Element) {
          queueMicrotask(() =>
            this.callback(
              [
                {
                  target,
                  contentRect: new DOMRect(0, 0, 800, 600),
                  borderBoxSize: [],
                  contentBoxSize: [],
                  devicePixelContentBoxSize: [],
                },
              ],
              this,
            ),
          );
        }

        readonly unobserve = vi.fn();
        readonly disconnect = vi.fn();
      },
    );
    vi.spyOn(actions.approvals, "load").mockResolvedValue(undefined);
    vi.spyOn(actions.approvals, "decide").mockRejectedValue(
      Object.assign(new Error(message), { fields: { base: [message] } }),
    );
    mutations.landApprovalPage(
      approvalListKey(9, "all"),
      { approvals: [approval()], users: [], nextCursor: null },
      "replace",
    );

    await inRouter(<AgentApprovals agentId={9} filter="all" onFilterChange={() => undefined} />);
    await userEvent.click(await screen.findByRole("button", { name: "Approve" }));

    await waitFor(() =>
      expect(toastSnapshot()).toEqual([
        expect.objectContaining({ title: "Couldn't approve it", description: message }),
      ]),
    );
  });

  it("offers more ledger entries after a short page only when it has a cursor", () => {
    vi.spyOn(actions.ledger, "load").mockResolvedValue(undefined);

    const more = vi.spyOn(actions.ledger, "loadMore").mockResolvedValue(undefined);
    const key = ledgerListKey(9, "all");

    mutations.landLedgerPage(
      key,
      9,
      { events: [entry()], users: [], nextCursor: "next-page" },
      "replace",
    );

    const view = renderHook(() => useAgentLedger(9, "all"));

    expect(view.result.current.rows).toHaveLength(1);
    expect(view.result.current.hasMore).toBe(true);
    act(() => view.result.current.loadMore());
    expect(more).toHaveBeenCalledWith(9, "all");

    act(() =>
      mutations.landLedgerPage(key, 9, { events: [], users: [], nextCursor: null }, "more"),
    );

    expect(view.result.current.rows).toHaveLength(1);
    expect(view.result.current.hasMore).toBe(false);
  });
});

describe("approval wording", () => {
  it("says who decided and when, and 'someone' for a gone account", () => {
    const approved = approval({ status: "approved", decidedById: 1, decidedAt: at(-HOUR) });

    expect(decisionLine(approved, "Riel", NOW)).toBe("Approved by Riel · 1 hour ago");
    expect(decisionLine({ ...approved, status: "denied" }, null, NOW)).toBe(
      "Denied by someone · 1 hour ago",
    );
    expect(decisionLine(approval({ status: "cancelled" }), null, NOW)).toBe(
      "Cancelled by the agent",
    );
    expect(decisionLine(approval(), null, NOW)).toBe("Asked 2 hours ago · expires in 2 days");
  });

  it("reads a pending request past its expiry as expired", () => {
    const overdue = approval({ expiresAt: at(-HOUR) });

    expect(shownStatus(overdue, NOW)).toBe("expired");
    expect(decisionLine(overdue, null, NOW)).toBe("Expired 1 hour ago without a decision");
    expect(beamedApprovalId([overdue, approval({ id: 99 })], NOW)).toBe(99);
  });

  it("explains an admin-only approval, and who the action runs as", () => {
    const merge = approval({
      action: "github.merge_pull_request",
      adminOnly: true,
      approvable: false,
      githubLogin: "ember-bot",
    });

    expect(adminOnlyText(merge)).toBe("Only an administrator can approve GitHub write actions.");
    expect(adminOnlyText({ ...merge, action: "fizzy.create_card" })).toBe(
      "Only an administrator can approve Fizzy write actions.",
    );
    expect(adminOnlyText({ ...merge, approvable: true })).toBeNull();
    expect(actsAsText(merge)).toBe("Acts on GitHub as @ember-bot");
    expect(actsAsText(approval({ fizzyUserName: "Ember" }))).toBe("Acts on Fizzy as Ember");
  });
});

describe("ledger wording", () => {
  it("has a sentence and a glyph for every type, and tolerates unknown ones", () => {
    const names = { actor: "Maya", agent: "Ember" };

    expect(ledgerSentence("mention", names)).toBe("Maya mentioned Ember");
    expect(ledgerSentence("work_handed_off", { ...names, actor: null })).toBe(
      "Work was handed off to Ember",
    );
    expect(ledgerSentence("delivery_suppressed_hop_limit", names)).toBe(
      "Not delivered: too many agent hops",
    );

    // SAFETY: a newer server's type, which the wire keeps as sent.
    const unknown = "budget_threshold_crossed" as AgentLedgerEventType;

    expect(ledgerSentence(unknown, names)).toBe("Budget threshold crossed");
    expect(ledgerIcon(unknown)).toBe("circle-dot");
    expect(outcomeLabel("suppressed")).toBe("Suppressed");
  });

  it("describes the webhook, an external result and the hop", () => {
    expect(webhookLine(entry())).toBeNull();
    expect(
      webhookLine(
        entry({ webhookStatus: "failed", webhookAttempts: 3, webhookLastError: "HTTP 502" }),
      ),
    ).toBe("Webhook failed · 3 attempts · HTTP 502");
    expect(webhookLine(entry({ webhookStatus: "delivered", webhookAttempts: 1 }))).toBe(
      "Webhook delivered · 1 attempt",
    );
    expect(
      externalLine(
        entry({
          eventType: "github_action_completed",
          external: { action: "merge_pull_request", status: "succeeded", message: "Merged #318" },
        }),
      ),
    ).toBe("GitHub merge_pull_request: succeeded — Merged #318");
    expect(
      externalLine(
        entry({
          eventType: "fizzy_action_completed",
          external: { action: "create_card", status: "created", message: null },
        }),
      ),
    ).toBe("Fizzy create_card: created");
    expect(hopText(0)).toBeNull();
    expect(hopText(2)).toBe("Hop 2");
  });
});

describe("the approval card", () => {
  it("decides with an optional note", async () => {
    const onDecide = vi.fn();

    await inRouter(
      <ApprovalCard
        approval={approval()}
        now={NOW}
        motion={undefined}
        beamed
        onDecide={onDecide}
      />,
    );

    await userEvent.click(screen.getByRole("button", { name: "Add a note" }));
    await userEvent.type(screen.getByRole("textbox", { name: "Note" }), "  Go ahead ");
    await userEvent.click(screen.getByRole("button", { name: "Approve" }));

    expect(onDecide).toHaveBeenCalledWith(approval(), "approved", "Go ahead");
    expect(screen.getByRole("link", { name: "engineering" })).toBeTruthy();
  });

  it("offers only Deny with the admin-only reason, and names a hidden room", async () => {
    await inRouter(
      <ApprovalCard
        approval={approval({
          action: "github.merge_pull_request",
          adminOnly: true,
          approvable: false,
          roomName: null,
        })}
        now={NOW}
        motion={undefined}
        beamed={false}
        onDecide={() => undefined}
      />,
    );

    expect(screen.queryByRole("button", { name: "Approve" })).toBeNull();
    expect(screen.getByRole("button", { name: "Deny" })).toBeTruthy();
    expect(
      screen.getByText("Only an administrator can approve GitHub write actions."),
    ).toBeTruthy();
    expect(screen.getByText("a room you're not in")).toBeTruthy();
  });

  it("shows a decided request with who decided it and the note, and no buttons", async () => {
    store.setState({ users: { 4: userFixture(4, "Priya") } });

    await inRouter(
      <ApprovalCard
        approval={approval({
          status: "denied",
          decidedById: 4,
          decidedAt: at(-HOUR),
          decisionNote: "Not this week.",
        })}
        now={NOW}
        motion={undefined}
        beamed
        onDecide={() => undefined}
      />,
    );

    expect(screen.getByText("Denied by Priya · 1 hour ago")).toBeTruthy();
    expect(screen.getByText("Not this week.")).toBeTruthy();
    expect(screen.queryByRole("button")).toBeNull();
  });
});

describe("the ledger row", () => {
  it("links to the message it can open, with the quote", async () => {
    store.setState({ users: { 2: userFixture(2, "Maya") } });

    await inRouter(<LedgerRow event={entry()} agentName="Ember" now={NOW} />);

    expect(screen.getByText("Maya mentioned Ember")).toBeTruthy();
    expect(screen.getByText("Can you look at the deploy log?")).toBeTruthy();
    expect(screen.getByRole("link", { name: "View message" }).getAttribute("href")).toBe(
      "/r/3/m/30001",
    );
  });

  it("gates a room the viewer isn't in, and labels a handoff", async () => {
    await inRouter(
      <LedgerRow
        event={entry({
          eventType: "work_handed_off",
          roomName: null,
          content: null,
          handoffSummary: null,
          actorId: null,
        })}
        agentName="Ember"
        now={NOW}
      />,
    );

    expect(screen.getByText("a room you're not in")).toBeTruthy();
    expect(screen.getByText("Content unavailable")).toBeTruthy();
    expect(screen.queryByRole("link")).toBeNull();
  });

  it("adds the Handoff label to a summary", async () => {
    await inRouter(
      <LedgerRow
        event={entry({
          eventType: "work_handed_off",
          messageId: null,
          handoffSummary: "Staging is green.",
        })}
        agentName="Ember"
        now={NOW}
      />,
    );

    expect(screen.getByText("Handoff:")).toBeTruthy();
    expect(screen.getByText("Staging is green.", { exact: false })).toBeTruthy();
    expect(screen.queryByText("Content unavailable")).toBeNull();
  });
});

describe("decisions made elsewhere", () => {
  const pending = approval({ id: 1, status: "pending", summary: "Merge PR #318" });

  it("finds shown pending requests now decided, skipping the viewer's own", () => {
    const watched = new Map([
      [1, "pending" as const],
      [2, "pending" as const],
      [3, "approved" as const],
    ]);

    const items = {
      1: { ...pending, status: "approved" as const, decidedById: 7 },
      2: approval({ id: 2, status: "denied" }),
      3: approval({ id: 3, status: "approved" }),
    };

    expect(liveDecisions(watched, items, new Set()).map((each) => each.id)).toEqual([1, 2]);
    expect(liveDecisions(watched, items, new Set([1])).map((each) => each.id)).toEqual([2]);
    expect(liveDecisions(new Map([[1, "pending" as const]]), { 1: pending }, new Set())).toEqual(
      [],
    );
  });

  it("keeps watching a pending request that left the list before its event", () => {
    const shown = approval({ id: 2, status: "pending" });

    const previous = new Map([
      [1, "pending" as const],
      [3, "pending" as const],
    ]);

    const next = nextWatched(previous, [shown], {
      1: pending,
      2: shown,
      3: approval({ id: 3, status: "denied" }),
    });

    expect([...next]).toEqual([
      [2, "pending"],
      [1, "pending"],
    ]);
  });

  it("names one decision and its decider, and counts a burst", () => {
    const decided = { ...pending, status: "approved" as const, decidedById: 7 };
    const nameOf = (userId: number) => (userId === 7 ? "Priya Raman" : null);

    expect(liveDecisionAnnouncement([decided], nameOf)).toBe(
      "Approved by Priya Raman: Merge PR #318",
    );
    expect(
      liveDecisionAnnouncement(
        [{ ...pending, status: "expired" as const, decidedById: null }],
        nameOf,
      ),
    ).toBe("Expired: Merge PR #318");
    expect(liveDecisionAnnouncement([decided, { ...decided, id: 2 }], nameOf)).toBe(
      "2 approval requests were decided",
    );
  });
});
