/**
 * Shown lists reload for each invalidation, even one that arrives while a reload is on its way
 * (the list lands still stale), with the real load actions against a fake server.
 */
import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  Outlet,
  type RouteComponent,
  RouterProvider,
} from "@tanstack/react-router";
import { act, cleanup, render, screen, waitFor } from "@testing-library/react";
import { Effect, ManagedRuntime, type Schema } from "effect";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { NotFound } from "../api/errors.ts";
import { FakeApi, type FakeHandler, meFixture } from "../api/testing.ts";
import { AgentApprovals } from "../features/agents/agent-approvals-tab.tsx";
import { ScheduledPage } from "../features/scheduled/scheduled-page.tsx";
import type { AgentApproval } from "../gen/AgentApproval.ts";
import type { ScheduledMessage } from "../gen/ScheduledMessage.ts";
import type { SyncEvent } from "../gen/SyncEvent.ts";
import { approvalListKey, approvalListOf } from "../store/approvals.ts";
import { scheduledListOf } from "../store/scheduled.ts";
import { initialState } from "../store/state.ts";
import { mutations, store } from "../store/store.ts";
import * as approvalActions from "./approval-actions.ts";
import { actions } from "./runtime.ts";
import * as scheduledActions from "./scheduled-actions.ts";

const AGENT = 9;

/** A server's answer held until it resolves. */
type Held = PromiseWithResolvers<void>;

/** The next first-page read to hold, if any. */
interface HeldRead {
  next: Held | null;
}

/** The fake server, and the real actions run against it. */
function fakeServer() {
  const runtime = ManagedRuntime.make(FakeApi.layerClient);

  const fake = () =>
    runtime.runPromise(
      Effect.gen(function* () {
        return yield* FakeApi;
      }),
    );

  return {
    runtime,
    route: async (key: string, answer: FakeHandler) =>
      runtime.runPromise((await fake()).route(key, answer)),
    reads: async (path: string, status: string | null = null) =>
      (await runtime.runPromise((await fake()).requests)).filter(
        (request) =>
          request.method === "GET" &&
          request.path === path &&
          (status === null || request.query?.status === status),
      ).length,
  };
}

/** Answers with `page()` as the server holds it when asked, held once while `hold` is set. */
function answering<A extends Schema.Json>(page: () => A, hold: HeldRead) {
  const held = hold.next;

  hold.next = null;

  return held === null
    ? Effect.succeed(page())
    : Effect.promise(() => held.promise).pipe(Effect.map(page));
}

function renderAt(component: RouteComponent) {
  const root = createRootRoute({ component: Outlet });
  const home = createRoute({ getParentRoute: () => root, path: "/", component });

  const router = createRouter({
    routeTree: root.addChildren([home]),
    history: createMemoryHistory({ initialEntries: ["/"] }),
  });

  render(<RouterProvider router={router} />);

  return act(() => router.load());
}

beforeEach(() => {
  // jsdom has no layout: give the virtual lists a viewport.
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

      unobserve() {}

      disconnect() {}
    },
  );
  vi.spyOn(HTMLElement.prototype, "offsetParent", "get").mockImplementation(function (
    this: HTMLElement,
  ) {
    return this.parentElement;
  });
  vi.spyOn(window, "scrollTo").mockImplementation(() => undefined);
  window.matchMedia = (query: string) =>
    Object.assign(new EventTarget(), {
      matches: false,
      media: query,
      onchange: null,
      addListener: () => undefined,
      removeListener: () => undefined,
    });
  mutations.setMe(meFixture);
});

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
  store.setState(initialState, true);
});

describe("pending approvals", () => {
  const approval = (id: number): AgentApproval => ({
    id,
    agentId: AGENT,
    agentUserId: AGENT,
    roomId: 3,
    roomName: "engineering",
    action: "deploy.production",
    summary: `Request ${id}`,
    status: "pending",
    expiresAt: "2099-01-01T00:00:00.000Z",
    createdAt: "2026-10-06T16:30:00.000Z",
    decidedById: null,
    decidedAt: null,
    decisionNote: null,
    githubLogin: null,
    fizzyUserName: null,
    adminOnly: false,
    approvable: true,
    deniable: true,
    updatedAt: "2026-10-06T16:30:00.000000Z",
  });

  /** A new request publishes only its activity item. */
  const requested = (seq: number, approvalId: number): SyncEvent => ({
    seq,
    topic: "user",
    type: "activity.item",
    data: {
      item: {
        id: seq,
        eventType: "agent_approval_request",
        state: "unread",
        readAt: null,
        handledAt: null,
        createdAt: "2026-10-06T16:30:00.000Z",
        updatedAt: "2026-10-06T16:30:00.000Z",
        source: {
          sourceType: "agent_approval",
          sourceId: approvalId,
          roomId: 3,
          threadId: null,
          messageId: null,
          eventId: null,
          creatorId: AGENT,
          title: "engineering",
          body: "Deploy to production",
          occurredAt: "2026-10-06T16:30:00.000Z",
          approvalStatus: "pending",
          budgetCap: null,
          path: "/",
        },
      },
      unreadCount: seq,
      unreadRevision: seq,
      evaluatedAt: "2026-10-10T12:00:00.000000000Z",
    },
  });

  it("loads again for requests that arrive while a reload is on its way", async () => {
    const server = fakeServer();
    const hold: HeldRead = { next: null };
    let pending = [100];
    const path = `/agents/${AGENT}/approvals`;
    const list = () => approvalListOf(store.getState(), approvalListKey(AGENT, "pending"));

    await server.route(`GET ${path}`, () =>
      answering(() => ({ approvals: pending.map(approval), users: [], nextCursor: null }), hold),
    );
    vi.spyOn(actions.approvals, "load").mockImplementation((agentId, filter) =>
      server.runtime.runPromise(approvalActions.load(agentId, filter)),
    );
    vi.spyOn(actions.approvals, "loadMore").mockImplementation((agentId, filter) =>
      server.runtime.runPromise(approvalActions.loadMore(agentId, filter)),
    );

    await renderAt(() => (
      <AgentApprovals agentId={AGENT} filter="pending" onFilterChange={() => undefined} />
    ));
    await screen.findByText("Request 100");

    // 101 arrives; the reload it asks for is held.
    const held = Promise.withResolvers<void>();

    hold.next = held;
    pending = [101, 100];
    act(() => mutations.applyEvents([requested(1, 101)], Date.now()));
    await waitFor(async () => expect(await server.reads(path)).toBe(2));

    // 102 arrives while that reload is on its way, then it lands without 102.
    pending = [102, 101, 100];
    act(() => mutations.applyEvents([requested(2, 102)], Date.now()));
    await act(async () => held.resolve());
    await waitFor(async () => expect(await server.reads(path)).toBe(3));

    pending = [103, 102, 101, 100];
    act(() => mutations.applyEvents([requested(3, 103)], Date.now()));

    await screen.findByText("Request 103");
    expect(screen.getByText("Request 102")).toBeTruthy();
    await waitFor(() =>
      expect(list()).toMatchObject({ ids: [103, 102, 101, 100], stale: false, status: "ready" }),
    );

    await server.runtime.dispose();
  });
});

describe("the scheduled page", () => {
  const scheduled = (id: number): ScheduledMessage => ({
    id,
    roomId: 12,
    threadId: null,
    replyToMessageId: null,
    replyTarget: null,
    markdownSource: `Draft ${id}`,
    excerpt: `Draft ${id}`,
    sendAt: `2030-01-0${id}T12:00:00Z`,
    state: "pending",
    sendable: true,
    sentAt: null,
    sentMessageId: null,
    droppedAt: null,
    dropReason: null,
    createdAt: "2026-10-10T12:00:00Z",
    attachments: [],
  });

  it("loads again for a cancel refused while a reload is on its way", async () => {
    const server = fakeServer();
    const hold: HeldRead = { next: null };
    let pending = [1, 2, 3, 4];
    const list = () => scheduledListOf(store.getState(), "pending");

    await server.route("GET /scheduled_messages", (request) => {
      const mine = request.query?.status === "pending";

      return answering(
        () => ({
          scheduledMessages: mine ? pending.map(scheduled) : [],
          conversations: [],
          nextCursor: null,
        }),
        mine ? hold : { next: null },
      );
    });

    // Each was sent from another device meanwhile: the cancel finds it gone.
    for (const id of [1, 2, 3]) {
      await server.route(`DELETE /scheduled_messages/${id}`, () =>
        Effect.fail(new NotFound({ message: "Not found" })),
      );
    }

    vi.spyOn(actions.scheduled, "load").mockImplementation((key) =>
      server.runtime.runPromise(scheduledActions.load(key)),
    );
    vi.spyOn(actions.scheduled, "loadMore").mockImplementation((key) =>
      server.runtime.runPromise(scheduledActions.loadMore(key)),
    );

    const cancel = (id: number) =>
      act(() => server.runtime.runPromise(scheduledActions.cancel(id)).catch(() => undefined));

    const pendingReads = () => server.reads("/scheduled_messages", "pending");

    await renderAt(ScheduledPage);
    // The page shows the soonest first (jsdom lays out only the first row).
    await screen.findByText("Draft 1");

    // 1 is gone; the reload its refused cancel asks for is held.
    const held = Promise.withResolvers<void>();

    hold.next = held;
    pending = [2, 3, 4];
    await cancel(1);
    await waitFor(async () => expect(await pendingReads()).toBe(2));

    // 2 is gone too before that reload lands, with 2 still in it.
    pending = [3, 4];
    await cancel(2);
    await act(async () => held.resolve());
    await waitFor(async () => expect(await pendingReads()).toBe(3));

    pending = [4];
    await cancel(3);

    await waitFor(() => expect(list()).toMatchObject({ ids: [4], stale: false, status: "ready" }));
    await screen.findByText("Draft 4");
    expect(screen.queryByText("Draft 2")).toBeNull();

    await server.runtime.dispose();
  });
});
