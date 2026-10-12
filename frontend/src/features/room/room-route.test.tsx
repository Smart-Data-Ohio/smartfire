import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  Outlet,
  RouterProvider,
} from "@tanstack/react-router";
import { act, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, expect, it, vi } from "vitest";
import { initialState } from "../../store/state.ts";
import { store } from "../../store/store.ts";
import { actions } from "../../sync/runtime.ts";
import { RoomRoute } from "./room-route.tsx";

afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
  store.setState(initialState, true);
});

it("retries an unavailable room through the API without a classic link", async () => {
  vi.stubGlobal("matchMedia", (query: string) =>
    Object.assign(new EventTarget(), {
      matches: false,
      media: query,
      onchange: null,
      addListener: () => undefined,
      removeListener: () => undefined,
    }),
  );
  store.setState({
    rooms: { 12: { detail: null, preview: null, status: "error", error: "Network unavailable" } },
  });
  vi.spyOn(actions, "openRoom").mockResolvedValue(undefined);
  vi.spyOn(actions, "closeRoom").mockImplementation(() => undefined);
  const reload = vi.spyOn(actions, "reloadRoom").mockResolvedValue(undefined);
  const root = createRootRoute({ component: Outlet });

  const room = createRoute({
    getParentRoute: () => root,
    path: "/r/$roomId",
    params: {
      parse: ({ roomId }) => ({ roomId: Number(roomId) }),
      stringify: ({ roomId }) => ({ roomId: String(roomId) }),
    },
    component: RoomRoute,
  });

  const router = createRouter({
    routeTree: root.addChildren([room]),
    basepath: "/app",
    history: createMemoryHistory({ initialEntries: ["/app/r/12"] }),
  });

  render(<RouterProvider router={router} />);
  await act(() => router.load());
  expect(screen.getByRole("region", { name: "Room unavailable" }).textContent).toContain(
    "Network unavailable",
  );
  expect(screen.queryByRole("link")).toBeNull();
  await userEvent.setup().click(screen.getByRole("button", { name: "Try again" }));
  expect(reload).toHaveBeenCalledWith(12, null);
  expect(router.history.location.href).toBe("/app/r/12");
});
