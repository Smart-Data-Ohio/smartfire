/**
 * Shown lists reload for each invalidation, even one that arrives while a reload is on its way
 * (the list lands still stale) or after the list was reopened under StrictMode while a reload was
 * on its way, with the real load actions against a fake server.
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
import { StrictMode } from "react";
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

/** Answers with `page` as the server held it when asked, held once while `hold` is set. */
function answering<A extends Schema.Json>(page: A, hold: HeldRead) {
  const held = hold.next;

  hold.next = null;

  return held === null
    ? Effect.succeed(page)
    : Effect.promise(() => held.promise).pipe(Effect.as(page));
}

/** Renders `component` as the app's page, under StrictMode when `strict`. */
function renderAt(component: RouteComponent, strict = false) {
  const root = createRootRoute({ component: Outlet });
  const home = createRoute({ getParentRoute: () => root, path: "/", component });

  const router = createRouter({
    routeTree: root.addChildren([home]),
    history: createMemoryHistory({ initialEntries: ["/"] }),
  });

  render(<RouterProvider router={router} />, { wrapper: strict ? StrictMode : undefined });

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

  const path = `/agents/${AGENT}/approvals`;
  const list = () => approvalListOf(store.getState(), approvalListKey(AGENT, "pending"));

  const Pending = () => (
    <AgentApprovals agentId={AGENT} filter="pending" onFilterChange={() => undefined} />
  );

  /** The server holds `pending.ids`, newest first; the hooks load through the real actions. */
  async function serve() {
    const server = fakeServer();
    const hold: HeldRead = { next: null };
    const pending = { ids: [100] };

    await server.route(`GET ${path}`, () =>
      answering({ approvals: pending.ids.map(approval), users: [], nextCursor: null }, hold),
    );
    vi.spyOn(actions.approvals, "load").mockImplementation((agentId, filter) =>
      server.runtime.runPromise(approvalActions.load(agentId, filter)),
    );
    vi.spyOn(actions.approvals, "loadMore").mockImplementation((agentId, filter) =>
      server.runtime.runPromise(approvalActions.loadMore(agentId, filter)),
    );

    return { server, hold, pending };
  }

  it("loads again for requests that arrive while a reload is on its way", async () => {
    const { server, hold, pending } = await serve();

    await renderAt(Pending);
    await screen.findByText("Request 100");

    // 101 arrives; the reload it asks for is held.
    const held = Promise.withResolvers<void>();

    hold.next = held;
    pending.ids = [101, 100];
    act(() => mutations.applyEvents([requested(1, 101)], Date.now()));
    await waitFor(async () => expect(await server.reads(path)).toBe(2));

    // 102 arrives while that reload is on its way, then it lands without 102.
    pending.ids = [102, 101, 100];
    act(() => mutations.applyEvents([requested(2, 102)], Date.now()));
    await act(async () => held.resolve());
    await waitFor(async () => expect(await server.reads(path)).toBe(3));

    pending.ids = [103, 102, 101, 100];
    act(() => mutations.applyEvents([requested(3, 103)], Date.now()));

    await screen.findByText("Request 103");
    expect(screen.getByText("Request 102")).toBeTruthy();
    await waitFor(() =>
      expect(list()).toMatchObject({ ids: [103, 102, 101, 100], stale: false, status: "ready" }),
    );

    await server.runtime.dispose();
  });

  it("keeps refreshing once reopened under StrictMode while its refresh was on its way", async () => {
    const { server, hold, pending } = await serve();

    await renderAt(Pending, true);
    await screen.findByText("Request 100");
    cleanup();

    // Reopened: StrictMode's rehearsed leave retires the refresh it started, which is held.
    const held = Promise.withResolvers<void>();
    const before = await server.reads(path);

    hold.next = held;
    pending.ids = [101, 100];
    await renderAt(Pending, true);
    await waitFor(async () => expect(await server.reads(path)).toBeGreaterThan(before));
    await act(async () => held.resolve());

    // The reopening still refreshes it.
    await waitFor(() => expect(list()).toMatchObject({ ids: [101, 100], firstPageLoading: false }));

    pending.ids = [102, 101, 100];
    act(() => mutations.applyEvents([requested(1, 102)], Date.now()));

    await screen.findByText("Request 102");
    expect(screen.getByText("Request 101")).toBeTruthy();
    await waitFor(() =>
      expect(list()).toMatchObject({
        ids: [102, 101, 100],
        stale: false,
        firstPageLoading: false,
        loadingMore: false,
      }),
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

  const list = () => scheduledListOf(store.getState(), "pending");

  /** The server holds `pending.ids` (each cancel finds its message already sent elsewhere). */
  async function serve() {
    const server = fakeServer();
    const hold: HeldRead = { next: null };
    const pending = { ids: [1, 2, 3, 4] };

    await server.route("GET /scheduled_messages", (request) => {
      const mine = request.query?.status === "pending";

      return answering(
        {
          scheduledMessages: mine ? pending.ids.map(scheduled) : [],
          conversations: [],
          nextCursor: null,
        },
        mine ? hold : { next: null },
      );
    });

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

    return { server, hold, pending, cancel, pendingReads };
  }

  it("loads again for a cancel refused while a reload is on its way", async () => {
    const { server, hold, pending, cancel, pendingReads } = await serve();

    await renderAt(ScheduledPage);
    // The page shows the soonest first (jsdom lays out only the first row).
    await screen.findByText("Draft 1");

    // 1 is gone; the reload its refused cancel asks for is held.
    const held = Promise.withResolvers<void>();

    hold.next = held;
    pending.ids = [2, 3, 4];
    await cancel(1);
    await waitFor(async () => expect(await pendingReads()).toBe(2));

    // 2 is gone too before that reload lands, with 2 still in it.
    pending.ids = [3, 4];
    await cancel(2);
    await act(async () => held.resolve());
    await waitFor(async () => expect(await pendingReads()).toBe(3));

    pending.ids = [4];
    await cancel(3);

    await waitFor(() => expect(list()).toMatchObject({ ids: [4], stale: false, status: "ready" }));
    await screen.findByText("Draft 4");
    expect(screen.queryByText("Draft 2")).toBeNull();

    await server.runtime.dispose();
  });

  it("keeps refreshing once reopened under StrictMode while its refresh was on its way", async () => {
    const { server, hold, pending, cancel, pendingReads } = await serve();

    await renderAt(ScheduledPage, true);
    await screen.findByText("Draft 1");
    cleanup();

    // Gone while the page was closed: reopened, it reloads, and that reload is held.
    pending.ids = [2, 3, 4];
    await cancel(1);

    const held = Promise.withResolvers<void>();
    const before = await pendingReads();

    hold.next = held;
    await renderAt(ScheduledPage, true);
    await waitFor(async () => expect(await pendingReads()).toBeGreaterThan(before));
    await act(async () => held.resolve());

    pending.ids = [3, 4];
    await cancel(2);

    await waitFor(() =>
      expect(list()).toMatchObject({
        ids: [3, 4],
        stale: false,
        firstPageLoading: false,
        loadingMore: false,
      }),
    );
    await screen.findByText("Draft 3");

    await server.runtime.dispose();
  });
});
