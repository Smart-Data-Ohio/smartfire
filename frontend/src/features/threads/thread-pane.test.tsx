import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  Outlet,
  RouterProvider,
  useParams,
} from "@tanstack/react-router";
import { act, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { initialState } from "../../store/state.ts";
import { store } from "../../store/store.ts";
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

afterEach(() => {
  store.setState(initialState, true);
  vi.restoreAllMocks();
});

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
});
