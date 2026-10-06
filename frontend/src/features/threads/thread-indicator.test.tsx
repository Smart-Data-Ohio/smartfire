import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  RouterProvider,
} from "@tanstack/react-router";
import { act, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { MessageDTO } from "../../store/model.ts";
import { initialState } from "../../store/state.ts";
import { store } from "../../store/store.ts";
import { membershipFixture, messageFixture } from "./test-fixtures.ts";
import { ThreadIndicator } from "./thread-indicator.tsx";

/** The indicator in a small router with the room and thread routes it links between. */
async function renderIndicator(message: MessageDTO) {
  const rootRoute = createRootRoute({ component: () => <ThreadIndicator message={message} /> });
  const roomRoute = createRoute({ getParentRoute: () => rootRoute, path: "/r/$roomId" });

  const threadRoute = createRoute({
    getParentRoute: () => rootRoute,
    path: "/r/$roomId/t/$threadId",
  });

  const router = createRouter({
    routeTree: rootRoute.addChildren([roomRoute, threadRoute]),
    history: createMemoryHistory({ initialEntries: ["/r/4"] }),
  });

  const view = render(<RouterProvider router={router} />);

  await act(() => router.load());

  return { router, view };
}

const NOW = Date.parse("2026-10-06T12:00:00.000Z");

function indicated(replyCount: number) {
  return messageFixture(10, {
    thread: {
      threadId: 5,
      replyCount,
      lastReplyAt: new Date(NOW - 3 * 3_600_000).toISOString(),
      replierIds: [],
    },
  });
}

afterEach(() => {
  vi.useRealTimers();
  store.setState(initialState, true);
});

describe("ThreadIndicator", () => {
  it("shows the reply count and the last reply, and opens the thread", async () => {
    vi.useFakeTimers({ now: NOW, toFake: ["Date"] });

    const user = userEvent.setup();

    const { router } = await renderIndicator(indicated(3));

    const button = screen.getByRole("button", {
      name: "3 replies. Last reply 3 hours ago. View thread",
    });

    expect(button.textContent).toContain("3 replies");
    await user.click(button);
    expect(router.state.location.pathname).toBe("/r/4/t/5");
  });

  it("marks an unread thread, and follows the membership live", async () => {
    vi.useFakeTimers({ now: NOW, toFake: ["Date"] });
    await renderIndicator(indicated(1));

    const button = screen.getByRole("button");

    expect(button.hasAttribute("data-unread")).toBe(false);

    act(() => {
      store.setState({
        threadMemberships: { 5: membershipFixture(5, { unreadAt: "2026-10-06T11:00:00.000Z" }) },
      });
    });

    expect(button.getAttribute("data-unread")).toBe("true");
    expect(button.getAttribute("aria-label")).toBe(
      "1 reply, unread. Last reply 3 hours ago. View thread",
    );
  });

  it("hides when the thread has no replies", async () => {
    await renderIndicator(indicated(0));

    expect(screen.queryByRole("button")).toBeNull();
  });
});
