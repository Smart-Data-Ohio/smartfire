import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  Outlet,
  RouterProvider,
  useParams,
} from "@tanstack/react-router";
import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { ThreadPermissions } from "../../gen/ThreadPermissions.ts";
import { initialState } from "../../store/state.ts";
import { store } from "../../store/store.ts";
import { ActionError } from "../../sync/run.ts";
import { actions } from "../../sync/runtime.ts";
import { threadFixture } from "./test-fixtures.ts";
import { ThreadPane } from "./thread-pane.tsx";

/** The thread route. A thread that belongs to another room leaves this component. */
function ThreadRoute() {
  const { roomId, threadId } = useParams({ strict: false });

  if (roomId !== 4) {
    return <p>Opened in its room</p>;
  }

  return <ThreadPane roomId={roomId ?? 0} threadId={threadId ?? 0} />;
}

async function mount(path: string) {
  const root = createRootRoute({ component: Outlet });

  const room = createRoute({
    getParentRoute: () => root,
    path: "/r/$roomId",
    params: {
      parse: ({ roomId }) => ({ roomId: Number(roomId) }),
      stringify: ({ roomId }) => ({ roomId: String(roomId) }),
    },
    component: Outlet,
  });

  const thread = createRoute({
    getParentRoute: () => room,
    path: "t/$threadId",
    params: {
      parse: ({ threadId }) => ({ threadId: Number(threadId) }),
      stringify: ({ threadId }) => ({ threadId: String(threadId) }),
    },
    validateSearch: (search: { m?: unknown }) => {
      const m = Number(search.m);

      return Number.isSafeInteger(m) && m > 0 ? { m } : {};
    },
    component: ThreadRoute,
  });

  const router = createRouter({
    routeTree: root.addChildren([room.addChildren([thread])]),
    basepath: "/app",
    history: createMemoryHistory({ initialEntries: [path] }),
  });

  render(<RouterProvider router={router} />);
  await act(() => router.load());

  return router;
}

beforeEach(() => {
  // jsdom has no matchMedia; the motion helpers ask it about reduced motion.
  window.matchMedia = (query: string) =>
    Object.assign(new EventTarget(), {
      matches: false,
      media: query,
      onchange: null,
      addListener: () => undefined,
      removeListener: () => undefined,
    });

  // jsdom has no ResizeObserver; the composer's send button watches its size.
  vi.stubGlobal(
    "ResizeObserver",
    class {
      observe() {}
      unobserve() {}
      disconnect() {}
    },
  );
});

afterEach(() => {
  store.setState(initialState, true);
  vi.restoreAllMocks();
});

/** The pane opens itself here instead of fetching; only its controls matter. */
function stubPane(): void {
  vi.spyOn(actions.threads, "open").mockResolvedValue(undefined);
  vi.spyOn(actions.threads, "close").mockImplementation(() => undefined);
}

/** The thread the route names, with its pane loaded and the viewer's say over it. */
function readyPane(threadId: number, canRename: boolean, autoArchiveAfterMinutes = 1440): void {
  store.setState({
    ...initialState,
    threads: { [threadId]: threadFixture(threadId, { autoArchiveAfterMinutes }) },
    threadPanes: {
      [threadId]: {
        status: "ready",
        error: null,
        permissions: {
          canRename,
          canClose: canRename,
          canReopen: false,
          canLock: canRename,
          canUnlock: false,
          canDelete: canRename,
          canConvertWork: false,
          canManageWork: false,
          canUpdateWorkStatus: false,
          canAssignWork: false,
          canRemoveWork: false,
        } satisfies ThreadPermissions,
        work: null,
        workFacts: null,
      },
    },
  });
}

describe("ThreadPane", () => {
  it("sends a thread from another room to the room it belongs to", async () => {
    vi.spyOn(actions.threads, "open").mockResolvedValue(undefined);
    vi.spyOn(actions.threads, "close").mockImplementation(() => undefined);
    store.setState({
      ...initialState,
      threads: { 9: threadFixture(9, { roomId: 8 }) },
    });

    const router = await mount("/app/r/4/t/9?m=3");

    await waitFor(() => expect(screen.getByText("Opened in its room")).toBeTruthy());
    expect(router.history.location.href).toBe("/app/r/8/t/9?m=3");
  });

  it("changes how long an idle thread stays open", async () => {
    stubPane();
    readyPane(9, true);
    const update = vi.spyOn(actions.threads, "update").mockResolvedValue(undefined);
    const user = userEvent.setup();

    await mount("/app/r/4/t/9");
    await user.click(screen.getByRole("button", { name: "Thread actions" }));
    await user.click(screen.getByRole("menuitem", { name: /Auto-archive after/ }));

    const dialog = await screen.findByRole("dialog");
    // SAFETY: the field labelled Auto-archive after is the <select>.
    const picker = within(dialog).getByLabelText("Auto-archive after") as HTMLSelectElement;

    expect(picker.value).toBe("1440");

    await user.selectOptions(picker, "10080");
    await user.click(within(dialog).getByRole("button", { name: "Save" }));

    await waitFor(() =>
      expect(update).toHaveBeenCalledWith(9, {
        name: null,
        autoArchiveAfterMinutes: 10080,
        status: null,
      }),
    );
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
  });

  it("keeps the picker open and says why when the server refuses the duration", async () => {
    stubPane();
    readyPane(9, true);
    const message = "Auto archive after minutes must be one of 60, 1440, 4320 or 10080";

    vi.spyOn(actions.threads, "update").mockRejectedValue(
      new ActionError("Validation", message, { autoArchiveAfterMinutes: [message] }),
    );
    const user = userEvent.setup();

    await mount("/app/r/4/t/9");
    await user.click(screen.getByRole("button", { name: "Thread actions" }));
    await user.click(screen.getByRole("menuitem", { name: /Auto-archive after/ }));

    const dialog = await screen.findByRole("dialog");
    const picker = within(dialog).getByLabelText("Auto-archive after");

    await user.selectOptions(picker, "60");
    await user.click(within(dialog).getByRole("button", { name: "Save" }));

    expect(await screen.findByText(message)).toBeTruthy();
    expect(picker.getAttribute("aria-invalid")).toBe("true");
    expect(screen.getByRole("dialog")).toBeTruthy();
  });

  it("offers no auto-archive change to a viewer who may only reply", async () => {
    stubPane();
    readyPane(9, false);
    const user = userEvent.setup();

    await mount("/app/r/4/t/9");
    await user.click(screen.getByRole("button", { name: "Thread actions" }));

    expect(screen.queryByRole("menuitem", { name: /Auto-archive after/ })).toBeNull();
  });
});
