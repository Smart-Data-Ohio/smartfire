import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  Outlet,
  RouterProvider,
} from "@tanstack/react-router";
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import type { ScheduledMessage } from "../../gen/ScheduledMessage.ts";
import type { SyncEvent } from "../../gen/SyncEvent.ts";
import type { Thread } from "../../gen/Thread.ts";
import type { ScheduledListKey } from "../../store/scheduled.ts";
import { scheduledListOf } from "../../store/scheduled.ts";
import { initialState } from "../../store/state.ts";
import { mutations, store } from "../../store/store.ts";
import { actions } from "../../sync/runtime.ts";
import { ScheduledPage } from "./scheduled-page.tsx";

const item = {
  id: 4,
  roomId: 12,
  threadId: null,
  replyToMessageId: null,
  replyTarget: null,
  markdownSource: "Files for review",
  excerpt: "Files for review",
  sendAt: "2030-01-01T12:00:00Z",
  state: "pending",
  sendable: true,
  sentAt: null,
  sentMessageId: null,
  droppedAt: null,
  dropReason: null,
  createdAt: "2026-10-10T12:00:00Z",
  attachments: [
    {
      signedId: "one",
      attachment: {
        filename: "report.pdf",
        contentType: "application/pdf",
        byteSize: 5,
        width: null,
        height: null,
        preview: "file",
        url: "/report",
        downloadUrl: "/report?download",
        thumbnailUrl: null,
      },
    },
  ],
} satisfies ScheduledMessage;

beforeEach(() => {
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
                contentRect: new DOMRect(0, 0, 390, 600),
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
});

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
  store.setState(initialState, true);
});

function renderPage() {
  const root = createRootRoute({ component: Outlet });
  const home = createRoute({ getParentRoute: () => root, path: "/", component: ScheduledPage });

  const router = createRouter({
    routeTree: root.addChildren([home]),
    history: createMemoryHistory({ initialEntries: ["/"] }),
  });

  render(<RouterProvider router={router} />);

  return router;
}

it("shows a scheduled file on the page and sends that draft now", async () => {
  vi.spyOn(actions.scheduled, "load").mockResolvedValue(undefined);
  const send = vi.spyOn(actions.scheduled, "sendNow").mockResolvedValue("sent");
  mutations.landScheduledPage(
    "pending",
    { scheduledMessages: [item], conversations: [], nextCursor: null },
    "replace",
  );
  mutations.landScheduledPage(
    "past",
    { scheduledMessages: [], conversations: [], nextCursor: null },
    "replace",
  );
  const root = createRootRoute({ component: Outlet });
  const home = createRoute({ getParentRoute: () => root, path: "/", component: ScheduledPage });

  const router = createRouter({
    routeTree: root.addChildren([home]),
    history: createMemoryHistory({ initialEntries: ["/"] }),
  });

  render(<RouterProvider router={router} />);
  await act(() => router.load());
  await screen.findByText("report.pdf");
  fireEvent.click(screen.getByRole("button", { name: "Send now" }));
  await waitFor(() => expect(send).toHaveBeenCalledWith(4));
});

const secret: Thread = {
  id: 88,
  roomId: 12,
  parentMessageId: 70,
  creatorId: 2,
  name: "Just us",
  status: "active",
  visibility: "private",
  replyCount: 1,
  lastActivityAt: "2026-10-12T10:00:00.000Z",
  autoArchiveAfterMinutes: 4320,
  createdAt: "2026-10-12T09:00:00.000Z",
  work: null,
};

const reply = {
  ...item,
  id: 5,
  threadId: secret.id,
  replyToMessageId: 71,
  replyTarget: {
    messageId: 71,
    roomId: 12,
    threadId: secret.id,
    creatorId: 2,
    authorName: "Maya",
    roomLabel: "general",
    excerpt: "Only us",
    createdAt: "2026-10-12T09:30:00.000Z",
  },
  markdownSource: "My reply",
  excerpt: "My reply",
  attachments: [],
} satisfies ScheduledMessage;

const removed: SyncEvent = {
  seq: 1,
  topic: "user",
  type: "thread.removed",
  data: { threadId: secret.id, roomId: 12 },
};

const granted: SyncEvent = { seq: 2, topic: "user", type: "thread.updated", data: secret };

/**
 * `actions.scheduled.load` as the server answers it: the reply sendable and quoting its message,
 * as it lists it again once access returns, unless `down` says the server answers 503.
 */
function serveScheduled(down: () => boolean) {
  return vi.spyOn(actions.scheduled, "load").mockImplementation(async (key: ScheduledListKey) => {
    mutations.setScheduledListLoading(key, false);

    const { generation } = scheduledListOf(store.getState(), key);

    if (down()) {
      mutations.setScheduledListFailed(key, "Service unavailable", generation);

      return;
    }

    const page = {
      scheduledMessages: key === "pending" ? [reply] : [],
      conversations: [],
      nextCursor: null,
    };

    mutations.landScheduledPage(key, page, "replace", generation);
  });
}

it.each([
  ["separate batches", [[removed], [granted]]],
  ["one batch", [[removed, granted]]],
] as const)(
  "sends a reply now again once the viewer is added back (%s)",
  async (_delivery, batches) => {
    const load = serveScheduled(() => false);
    const router = renderPage();
    const sendNow = () => screen.getByRole("button", { name: "Send now" }).hasAttribute("disabled");

    await act(() => router.load());
    await waitFor(() => expect(sendNow()).toBe(false));
    load.mockClear();

    for (const [index, batch] of batches.entries()) {
      act(() => mutations.applyEvents(batch, Date.now()));

      // Between the removal and the re-add, the reply can't be sent and quotes nothing.
      if (index < batches.length - 1) {
        expect(sendNow()).toBe(true);
        expect(store.getState().scheduled.items[5]?.replyTarget).toBeNull();
      }
    }

    await waitFor(() => expect(load).toHaveBeenCalledWith("pending"));
    await waitFor(() => expect(sendNow()).toBe(false));
    expect(store.getState().scheduled.items[5]?.replyTarget?.excerpt).toBe("Only us");
  },
);

it.each([
  ["Try again", "retry"],
  ["reopening the page", "reopen"],
] as const)("recovers from a failed reload after the re-add by %s", async (_way, recovery) => {
  let down = false;
  const load = serveScheduled(() => down);
  const first = renderPage();
  const sendNow = () => screen.getByRole("button", { name: "Send now" }).hasAttribute("disabled");

  await act(() => first.load());
  await waitFor(() => expect(sendNow()).toBe(false));
  act(() => mutations.applyEvents([removed], Date.now()));

  // The reload the re-add asks for fails, once: nothing asks again by itself.
  down = true;
  load.mockClear();
  act(() => mutations.applyEvents([granted], Date.now()));
  await screen.findByText("Service unavailable");
  expect(load.mock.calls.filter(([key]) => key === "pending")).toHaveLength(1);
  expect(sendNow()).toBe(true);
  down = false;
  load.mockClear();

  if (recovery === "retry") {
    fireEvent.click(screen.getByRole("button", { name: "Try again" }));
  } else {
    cleanup();
    await act(() => renderPage().load());
  }

  await waitFor(() => expect(sendNow()).toBe(false));
  expect(load).toHaveBeenCalledWith("pending");
  expect(store.getState().scheduled.items[5]?.replyTarget?.excerpt).toBe("Only us");
});
