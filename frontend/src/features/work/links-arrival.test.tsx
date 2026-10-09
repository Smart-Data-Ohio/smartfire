import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  Outlet,
  RouterProvider,
} from "@tanstack/react-router";
import { act, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { mutations } from "../../store/store.ts";
import { removeToast, toastSnapshot } from "../../ui/toast-store.ts";
import { LinksArrival } from "./links-arrival.tsx";
import { threadDetailFixture } from "./test-fixtures.ts";

const THREAD = 7;

/** A pasted room-scoped links URL: the thread pane stays mounted, and so does the arrival. */
async function mount(path: string) {
  const root = createRootRoute({ component: Outlet });

  const thread = createRoute({
    getParentRoute: () => root,
    path: "/r/$roomId/t/$threadId",
    component: () => (
      <>
        <p>Thread destination</p>
        <LinksArrival threadId={THREAD} />
        <Outlet />
      </>
    ),
  });

  const links = createRoute({
    getParentRoute: () => thread,
    path: "links",
    component: () => null,
  });

  const router = createRouter({
    routeTree: root.addChildren([thread.addChildren([links])]),
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
  for (const record of toastSnapshot()) {
    removeToast(record.id);
  }

  mutations.reset();
});

describe("LinksArrival", () => {
  it("explains an untracked thread and replaces the links URL with the thread", async () => {
    mutations.loadThreadDetail(threadDetailFixture(THREAD, null, null));

    const router = await mount(`/r/4/t/${THREAD}/links`);

    await waitFor(() => expect(titles()).toContain("This thread isn't tracked as work"));
    await waitFor(() => expect(router.history.location.pathname).toBe(`/r/4/t/${THREAD}`));
    expect(router.history.length).toBe(1);
    expect(screen.queryByRole("form", { name: "Link to this work" })).toBeNull();
    expect(screen.getByText("Thread destination")).toBeTruthy();
  });
});
