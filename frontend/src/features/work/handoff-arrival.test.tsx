import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  Outlet,
  RouterProvider,
} from "@tanstack/react-router";
import { act, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { mutations } from "../../store/store.ts";
import { removeToast, toastSnapshot } from "../../ui/toast-store.ts";
import { HandoffArrival } from "./handoff-arrival.tsx";
import {
  factsFixture,
  threadDetailFixture,
  workDetailFixture,
  workPermissionsFixture,
} from "./test-fixtures.ts";

const THREAD = 7;

async function mount(path: string) {
  const root = createRootRoute({ component: Outlet });

  const thread = createRoute({
    getParentRoute: () => root,
    path: "/r/$roomId/t/$threadId",
    component: () => (
      <>
        <p>Thread destination</p>
        <Outlet />
      </>
    ),
  });

  const handoff = createRoute({
    getParentRoute: () => thread,
    path: "handoff",
    component: () => <HandoffArrival threadId={THREAD} />,
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
});
