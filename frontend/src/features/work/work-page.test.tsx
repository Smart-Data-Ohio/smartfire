import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  Outlet,
  RouterProvider,
} from "@tanstack/react-router";
import { act, render, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { mutations } from "../../store/store.ts";
import { actions } from "../../sync/runtime.ts";
import { WorkPage } from "./work-page.tsx";

function Page() {
  return <WorkPage filter="open" onFilterChange={() => undefined} />;
}

async function mount() {
  const root = createRootRoute({ component: Outlet });
  const home = createRoute({ getParentRoute: () => root, path: "/", component: Page });

  const router = createRouter({
    routeTree: root.addChildren([home]),
    history: createMemoryHistory({ initialEntries: ["/"] }),
  });

  render(<RouterProvider router={router} />);
  await act(() => router.load());

  return router;
}

function setVisibility(value: DocumentVisibilityState) {
  Object.defineProperty(document, "visibilityState", {
    configurable: true,
    get: () => value,
  });
}

afterEach(() => {
  vi.restoreAllMocks();
  mutations.reset();
  setVisibility("visible");
});

describe("the Work page", () => {
  it("refetches when the browser tab comes back into view, and when the window is focused", async () => {
    const load = vi.spyOn(actions.work, "loadList").mockResolvedValue(undefined);

    await mount();
    await waitFor(() => expect(load).toHaveBeenCalledTimes(1));
    expect(load).toHaveBeenCalledWith("open");

    setVisibility("hidden");
    act(() => {
      document.dispatchEvent(new Event("visibilitychange"));
      window.dispatchEvent(new Event("focus"));
    });
    expect(load).toHaveBeenCalledTimes(1);

    setVisibility("visible");
    act(() => {
      document.dispatchEvent(new Event("visibilitychange"));
    });
    await waitFor(() => expect(load).toHaveBeenCalledTimes(2));

    act(() => {
      window.dispatchEvent(new Event("focus"));
    });
    await waitFor(() => expect(load).toHaveBeenCalledTimes(3));
  });
});
