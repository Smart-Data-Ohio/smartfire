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
