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
import { useState } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { WorkFilter } from "../../gen/WorkFilter.ts";
import type { WorkList } from "../../gen/WorkList.ts";
import { initialState } from "../../store/state.ts";
import { store } from "../../store/store.ts";
import { ActionError } from "../../sync/run.ts";
import { actions } from "../../sync/runtime.ts";
import { boardThread } from "../../test/board-fixtures.ts";
import { WorkPage } from "./work-page.tsx";

function list(...names: string[]): WorkList {
  return {
    threads: names.map((name, index) => ({
      thread: { ...boardThread(9000 + index, "in_progress"), name },
      roomName: "Roadmap",
      board: true,
      updatedAt: "2026-10-07T10:00:00.000Z",
    })),
    users: [],
  };
}

/**
 * A row's link, by its label. The virtualiser keeps rows `visibility: hidden` until it has
 * measured them, which jsdom never lets it do, so they have no accessible name here.
 */
function findRow(name: RegExp): Promise<HTMLElement> {
  return screen.findByLabelText(name, { selector: "a" });
}

function Harness() {
  const [filter, setFilter] = useState<WorkFilter>("open");

  return <WorkPage filter={filter} onFilterChange={setFilter} />;
}

async function mount() {
  const root = createRootRoute({ component: Outlet });
  const home = createRoute({ getParentRoute: () => root, path: "/", component: Harness });

  const thread = createRoute({
    getParentRoute: () => root,
    path: "/r/$roomId/t/$threadId",
    component: () => <p>Thread destination</p>,
  });

  const router = createRouter({
    routeTree: root.addChildren([home, thread]),
    basepath: "/app",
    history: createMemoryHistory({ initialEntries: ["/app/"] }),
  });

  render(<RouterProvider router={router} />);
  await act(() => router.load());

  return router;
}

const VIEWPORT = { width: 800, height: 600 } as const;

const VIEWPORT_RECT: DOMRectReadOnly = {
  ...VIEWPORT,
  x: 0,
  y: 0,
  top: 0,
  left: 0,
  right: VIEWPORT.width,
  bottom: VIEWPORT.height,
  toJSON: () => VIEWPORT,
};

const VIEWPORT_SIZE: ResizeObserverSize = {
  inlineSize: VIEWPORT.width,
  blockSize: VIEWPORT.height,
};

/**
 * jsdom has no layout or ResizeObserver: this one reports every observed box at a desktop list's
 * size, so the virtualiser draws the first rows.
 */
class ViewportResize implements ResizeObserver {
  readonly #report: ResizeObserverCallback;

  constructor(report: ResizeObserverCallback) {
    this.#report = report;
  }

  observe(target: Element) {
    const entry: ResizeObserverEntry = {
      target,
      contentRect: VIEWPORT_RECT,
      borderBoxSize: [VIEWPORT_SIZE],
      contentBoxSize: [VIEWPORT_SIZE],
      devicePixelContentBoxSize: [VIEWPORT_SIZE],
    };

    queueMicrotask(() => this.#report([entry], this));
  }

  unobserve() {}

  disconnect() {}
}

beforeEach(() => {
  vi.stubGlobal("ResizeObserver", ViewportResize);
});

afterEach(() => {
  vi.unstubAllGlobals();

  vi.restoreAllMocks();
  store.setState(initialState, true);
});

describe("the Work page", () => {
  it("shows a skeleton, then the open work, each row opening its thread", async () => {
    const pending = Promise.withResolvers<WorkList>();
    const load = vi.spyOn(actions.work, "list").mockReturnValue(pending.promise);
    const router = await mount();

    expect(load).toHaveBeenCalledWith("open");
    expect(document.querySelector('.t-skel[aria-busy="true"]')).toBeTruthy();
    expect(screen.getByRole("tab", { name: "Open" }).getAttribute("aria-selected")).toBe("true");

    await act(async () => pending.resolve(list("Cursor pagination")));

    const row = await findRow(/^Cursor pagination, In progress, in Roadmap/);

    expect(row.getAttribute("href")).toBe("/app/r/900/t/9000");
    await userEvent.click(row);
    await screen.findByText("Thread destination");
    expect(router.state.location.pathname).toBe("/r/900/t/9000");
  });

  it("loads each tab's filter, with that tab's own empty state", async () => {
    const load = vi
      .spyOn(actions.work, "list")
      .mockImplementation(async (filter) =>
        filter === "open" ? list("Cursor pagination") : list(),
      );

    await mount();
    await findRow(/^Cursor pagination/);

    await userEvent.click(screen.getByRole("tab", { name: "Done" }));
    expect(load).toHaveBeenLastCalledWith("done");
    expect(await screen.findByText("Nothing completed")).toBeTruthy();
    expect(
      screen.getByText("No completed work yet. Work shows up here once its status is set to Done."),
    ).toBeTruthy();

    await userEvent.click(screen.getByRole("tab", { name: "Agents" }));
    expect(load).toHaveBeenLastCalledWith("agents");
    expect(await screen.findByText("No agent-owned work")).toBeTruthy();

    await userEvent.click(screen.getByRole("tab", { name: "Boards" }));
    expect(load).toHaveBeenLastCalledWith("boards");
    expect(await screen.findByText("No board work")).toBeTruthy();
  });

  it("says the open work is empty in the classic page's words", async () => {
    vi.spyOn(actions.work, "list").mockResolvedValue(list());
    await mount();

    expect(await screen.findByText("No open work")).toBeTruthy();
    expect(
      screen.getByText("No open work yet. Track a channel thread as work and it will appear here."),
    ).toBeTruthy();
  });

  it("offers Try again when the work couldn't be loaded, and loads it again", async () => {
    const load = vi
      .spyOn(actions.work, "list")
      .mockRejectedValueOnce(new ActionError("ServerError", "Server error"))
      .mockResolvedValueOnce(list("Cursor pagination"));

    await mount();

    expect((await screen.findByRole("alert")).textContent).toContain(
      "Your work couldn't be loaded.",
    );
    await userEvent.click(screen.getByRole("button", { name: "Try again" }));
    await findRow(/^Cursor pagination/);
    expect(load).toHaveBeenCalledTimes(2);
    expect(load).toHaveBeenNthCalledWith(2, "open");
  });

  it("refetches when the browser tab comes back into view", async () => {
    const load = vi.spyOn(actions.work, "list").mockResolvedValue(list("Cursor pagination"));

    await mount();
    await findRow(/^Cursor pagination/);
    act(() => {
      document.dispatchEvent(new Event("visibilitychange"));
    });
    await waitFor(() => expect(load).toHaveBeenCalledTimes(2));
  });
});
