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
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it } from "vitest";
import { roomDetailFixture } from "../../api/testing.ts";
import { initialState } from "../../store/state.ts";
import { mutations, store } from "../../store/store.ts";
import { NotificationsButton } from "./notifications-button.tsx";

function Probe() {
  const { roomId } = useParams({ strict: false });

  return (
    <>
      <NotificationsButton roomId={roomId ?? 0} />
      <button type="button">Outside control</button>
    </>
  );
}

async function mount() {
  mutations.setRoomDetail(roomDetailFixture(4));

  const root = createRootRoute({ component: Outlet });

  const room = createRoute({
    getParentRoute: () => root,
    path: "/r/$roomId",
    params: {
      parse: ({ roomId }) => ({ roomId: Number(roomId) }),
      stringify: ({ roomId }) => ({ roomId: String(roomId) }),
    },
    component: Probe,
  });

  const notifications = createRoute({
    getParentRoute: () => room,
    path: "notifications",
    component: () => null,
  });

  const router = createRouter({
    routeTree: root.addChildren([room.addChildren([notifications])]),
    history: createMemoryHistory({ initialEntries: ["/r/4", "/r/4/notifications"] }),
  });

  render(<RouterProvider router={router} />);
  await act(() => router.load());

  return router;
}

afterEach(() => store.setState(initialState, true));

describe("the notification menu URL", () => {
  it("opens the existing menu and clears its route on Escape", async () => {
    const router = await mount();
    const user = userEvent.setup();

    expect(screen.getByRole("menu", { name: /^Notifications$/ })).toBeTruthy();
    await user.keyboard("{Escape}");
    await waitFor(() => expect(router.state.location.pathname).toBe("/r/4"));
    await waitFor(() => expect(screen.queryByRole("menu")).toBeNull());
  });

  it("closes on outside dismissal and clears the menu URL", async () => {
    const router = await mount();
    const user = userEvent.setup();

    await user.click(screen.getByRole("button", { name: "Outside control" }));
    await waitFor(() => expect(router.state.location.pathname).toBe("/r/4"));
    await waitFor(() => expect(screen.queryByRole("menu")).toBeNull());
  });

  it("does not latch the routed menu open when its trigger receives an arrow key", async () => {
    const router = await mount();
    const user = userEvent.setup();

    act(() => screen.getByRole("button", { name: /^Notifications:/ }).focus());
    await user.keyboard("{ArrowDown}");
    await act(async () => router.history.back());
    await waitFor(() => expect(router.state.location.pathname).toBe("/r/4"));
    await waitFor(() => expect(screen.queryByRole("menu")).toBeNull());
  });
});
