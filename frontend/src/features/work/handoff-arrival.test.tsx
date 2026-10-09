import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  Outlet,
  RouterProvider,
} from "@tanstack/react-router";
import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { mutations } from "../../store/store.ts";
import { removeToast, toastSnapshot } from "../../ui/toast-store.ts";
import { PostWork } from "../boards/post-work.tsx";
import { NO_RECEIVER_HANDOFF } from "./handoff-access.ts";
import { HandoffArrival } from "./handoff-arrival.tsx";
import {
  factsFixture,
  threadDetailFixture,
  workDetailFixture,
  workPermissionsFixture,
} from "./test-fixtures.ts";
import { WorkBar } from "./work-bar.tsx";

const THREAD = 7;

/**
 * `stay` is the thread pane's own UI (the work bar, a board post), which stays mounted on the
 * handoff URL. Without it, only the arrival is mounted, as the child route.
 */
async function mount(path: string, stay?: ReactNode) {
  const root = createRootRoute({ component: Outlet });

  const thread = createRoute({
    getParentRoute: () => root,
    path: "/r/$roomId/t/$threadId",
    component: () => (
      <>
        <p>Thread destination</p>
        {stay}
        <Outlet />
      </>
    ),
  });

  const handoff = createRoute({
    getParentRoute: () => thread,
    path: "handoff",
    component: () => (stay === undefined ? <HandoffArrival threadId={THREAD} /> : null),
  });

  const router = createRouter({
    routeTree: root.addChildren([thread.addChildren([handoff])]),
    history: createMemoryHistory({ initialEntries: [path] }),
  });

  render(<RouterProvider router={router} />);
  await act(() => router.load());

  return router;
}

function titles(): string[] {
  return toastSnapshot().map((record) => record.title);
}

beforeEach(() => {
  // jsdom has no matchMedia; avatars and motion ask it which theme is on screen.
  window.matchMedia = (query: string) =>
    Object.assign(new EventTarget(), {
      matches: false,
      media: query,
      onchange: null,
      addListener: () => undefined,
      removeListener: () => undefined,
    });
  Element.prototype.scrollIntoView = () => undefined;
});

afterEach(() => {
  for (const record of toastSnapshot()) {
    removeToast(record.id);
  }

  mutations.reset();
});

describe("HandoffArrival", () => {
  it("tells a non-manager on a board post why, and replaces the handoff URL with the thread", async () => {
    mutations.loadThreadDetail(
      threadDetailFixture(
        THREAD,
        factsFixture(),
        workDetailFixture(),
        workPermissionsFixture({ canManageWork: false }),
      ),
    );

    const router = await mount(`/r/4/t/${THREAD}/handoff`);

    await waitFor(() => expect(titles()).toContain("You cannot manage work in this thread"));
    await waitFor(() => expect(router.history.location.pathname).toBe(`/r/4/t/${THREAD}`));
    expect(router.history.length).toBe(1);
    expect(screen.queryByRole("dialog")).toBeNull();

    await act(async () => {
      await Promise.resolve();
    });
    expect(router.history.location.pathname).toBe(`/r/4/t/${THREAD}`);
  });

  it("tells a manager with nobody to receive the work why, and replaces the handoff URL", async () => {
    mutations.loadThreadDetail(
      threadDetailFixture(THREAD, factsFixture(), workDetailFixture({ handoffReceivers: [] })),
    );

    const router = await mount(`/r/4/t/${THREAD}/handoff`);

    await waitFor(() => expect(titles()).toContain(NO_RECEIVER_HANDOFF));
    expect(titles()).toContain(
      "No agent here can take this work. An agent needs to be in this room and allowed to post, manage threads and read messages.",
    );
    await waitFor(() => expect(router.history.location.pathname).toBe(`/r/4/t/${THREAD}`));
    expect(router.history.length).toBe(1);
    expect(screen.queryByRole("dialog")).toBeNull();
  });
});

describe("a handoff with no eligible receiver", () => {
  function load() {
    mutations.loadThreadDetail(
      threadDetailFixture(THREAD, factsFixture(), workDetailFixture({ handoffReceivers: [] })),
    );
  }

  it("does not open an empty dialog from the thread work bar", async () => {
    load();

    const router = await mount(`/r/4/t/${THREAD}/handoff`, <WorkBar threadId={THREAD} />);
    const user = userEvent.setup();

    await user.click(screen.getByRole("button", { name: "Result, steps and history" }));
    expect(screen.queryByRole("button", { name: "Hand off to an agent" })).toBeNull();
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(router.history.location.pathname).toBe(`/r/4/t/${THREAD}/handoff`);
  });

  it("explains a thread work bar handoff and leaves the URL", async () => {
    load();

    const router = await mount(
      `/r/4/t/${THREAD}/handoff`,
      <>
        <WorkBar threadId={THREAD} />
        <HandoffArrival threadId={THREAD} />
      </>,
    );

    await waitFor(() => expect(titles()).toContain(NO_RECEIVER_HANDOFF));
    await waitFor(() => expect(router.history.location.pathname).toBe(`/r/4/t/${THREAD}`));
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(router.history.length).toBe(1);
  });

  it("explains a board post handoff and leaves the URL", async () => {
    load();

    const router = await mount(
      `/r/4/t/${THREAD}/handoff`,
      <>
        <PostWork threadId={THREAD} />
        <HandoffArrival threadId={THREAD} />
      </>,
    );

    await waitFor(() => expect(titles()).toContain(NO_RECEIVER_HANDOFF));
    expect(screen.queryByRole("button", { name: "Hand off to an agent" })).toBeNull();
    expect(screen.queryByRole("dialog")).toBeNull();
    await waitFor(() => expect(router.history.location.pathname).toBe(`/r/4/t/${THREAD}`));
    expect(router.history.length).toBe(1);
  });
});
