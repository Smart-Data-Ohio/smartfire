import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  Outlet,
  RouterProvider,
} from "@tanstack/react-router";
import { act, cleanup, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { roomDetailFixture } from "../../api/testing.ts";
import { beginRoomRequest, resetJoinState } from "../../store/join-state.ts";
import { mutations, store } from "../../store/store.ts";

const { read, openRoom } = vi.hoisted(() => ({
  read: vi.fn(),
  openRoom: vi.fn(() => Promise.resolve()),
}));

vi.mock("../../sync/runtime.ts", () => ({
  actions: {
    messages: { read },
    openRoom,
    closeRoom: vi.fn(),
    joinOpenRoom: vi.fn(() => Promise.resolve()),
    reloadRoom: vi.fn(() => Promise.resolve()),
  },
}));

vi.mock("../panes/use-right-pane.ts", () => ({
  usePhoneLayout: () => false,
  useRightPaneView: () => null,
  useRoomPaneLifecycle: () => undefined,
}));

vi.mock("../threads/prefetch.ts", () => ({
  prefetchThreadMemberships: () => undefined,
}));

vi.mock("./timeline.tsx", () => ({ Timeline: () => null }));
vi.mock("./room-header.tsx", () => ({ RoomHeader: () => null }));
vi.mock("../composer/composer.tsx", () => ({ Composer: () => null }));
vi.mock("../boards/board-view.tsx", () => ({ BoardView: () => null }));
vi.mock("../fizzy/fizzy-card-overlay.tsx", () => ({ FizzyCardOverlay: () => null }));
vi.mock("../huddle/call-alerts.tsx", () => ({ JoinBanner: () => null }));
vi.mock("../huddle/call-view.tsx", () => ({ CallView: () => null }));
vi.mock("../panes/right-pane.tsx", () => ({ RightPane: () => null }));
vi.mock("../rooms/room-settings-host.tsx", () => ({ RoomSettingsHost: () => null }));

const { RoomRoute } = await import("./room-route.tsx");

const ROOM = 8;
const MESSAGE = 4;

type PendingRead = {
  readonly issued: "member" | "unjoined" | null;
  resolve: (value: { message: { id: number; roomId: number; threadId: number | null } }) => void;
  reject: (error: unknown) => void;
};

const pending: PendingRead[] = [];

/** The same membership `roomAccess` would report for this fixture. */
function accessNow(): PendingRead["issued"] {
  const room = store.getState().rooms[ROOM];

  if (room?.detail != null) {
    return "member";
  }

  if (room?.preview != null) {
    return "unjoined";
  }

  return null;
}

async function renderPermalink() {
  const root = createRootRoute({ component: Outlet });

  const room = createRoute({
    getParentRoute: () => root,
    path: "r/$roomId",
    params: {
      parse: ({ roomId }) => ({ roomId: Number(roomId) }),
      stringify: ({ roomId }) => ({ roomId: `${roomId}` }),
    },
    component: RoomRoute,
  });

  const message = createRoute({
    getParentRoute: () => room,
    path: "m/$messageId",
    params: {
      parse: ({ messageId }) => ({ messageId: Number(messageId) }),
      stringify: ({ messageId }) => ({ messageId: `${messageId}` }),
    },
    component: () => null,
  });

  const router = createRouter({
    routeTree: root.addChildren([room.addChildren([message])]),
    history: createMemoryHistory({ initialEntries: [`/r/${ROOM}/m/${MESSAGE}`] }),
  });

  render(<RouterProvider router={router} />);
  await act(() => router.load());

  return router;
}

beforeEach(() => {
  pending.length = 0;
  read.mockReset();
  openRoom.mockClear();
  resetJoinState();
  mutations.reset();
  mutations.setRoomPreview(ROOM, { id: ROOM, name: "campfire" }, beginRoomRequest());

  read.mockImplementation(
    () =>
      new Promise((resolve, reject) => {
        pending.push({ issued: accessNow(), resolve, reject });
      }),
  );
});

afterEach(() => {
  cleanup();
  resetJoinState();
  mutations.reset();
});

describe("permalink read issued before join", () => {
  it("keeps the anchor and re-reads once when join beats the pre-join 404", async () => {
    const router = await renderPermalink();

    expect(screen.getByRole("region", { name: "Join #campfire" })).toBeTruthy();
    expect(pending).toHaveLength(1);
    expect(pending[0]?.issued).toBe("unjoined");

    await act(() => {
      mutations.setRoomDetail(roomDetailFixture(ROOM), beginRoomRequest());
    });

    await act(async () => {
      pending[0]?.reject(new Error("not found"));
    });

    // The pre-join 404 either starts the member re-read or replaces the permalink.
    await vi.waitFor(() => {
      const dropped = router.state.location.pathname === `/r/${ROOM}`;
      const reread = pending.length === 2;

      expect(dropped || reread).toBe(true);
    });

    expect(pending.map((entry) => entry.issued)).toEqual(["unjoined", "member"]);
    expect(router.state.location.pathname).toBe(`/r/${ROOM}/m/${MESSAGE}`);
    expect(openRoom).not.toHaveBeenCalled();

    await act(async () => {
      pending[1]?.resolve({ message: { id: MESSAGE, roomId: ROOM, threadId: null } });
      await Promise.resolve();
    });

    expect(pending).toHaveLength(2);
    expect(openRoom).toHaveBeenCalledWith(ROOM, MESSAGE);
    expect(router.state.location.pathname).toBe(`/r/${ROOM}/m/${MESSAGE}`);
  });

  it("keeps the permalink when the 404 settles before join, then reads as a member", async () => {
    const router = await renderPermalink();

    await act(async () => {
      pending[0]?.reject(new Error("not found"));
      await Promise.resolve();
    });

    expect(pending).toHaveLength(1);
    expect(pending[0]?.issued).toBe("unjoined");
    expect(router.state.location.pathname).toBe(`/r/${ROOM}/m/${MESSAGE}`);
    expect(openRoom).toHaveBeenCalledWith(ROOM, MESSAGE);

    await act(() => {
      mutations.setRoomDetail(roomDetailFixture(ROOM), beginRoomRequest());
    });

    expect(pending.map((entry) => entry.issued)).toEqual(["unjoined", "member"]);
    expect(router.state.location.pathname).toBe(`/r/${ROOM}/m/${MESSAGE}`);
  });

  it("drops the anchor when the member re-read is also missing, and does not read again", async () => {
    const router = await renderPermalink();

    await act(() => {
      mutations.setRoomDetail(roomDetailFixture(ROOM), beginRoomRequest());
    });

    await act(async () => {
      pending[0]?.reject(new Error("not found"));
    });

    await vi.waitFor(() => {
      expect(pending).toHaveLength(2);
    });

    await act(async () => {
      pending[1]?.reject(new Error("not found"));
    });

    await vi.waitFor(() => {
      expect(router.state.location.pathname).toBe(`/r/${ROOM}`);
    });

    expect(pending).toHaveLength(2);
    expect(pending.map((entry) => entry.issued)).toEqual(["unjoined", "member"]);
  });

  it("drops a missing message from a read that was issued as a member", async () => {
    mutations.setRoomDetail(roomDetailFixture(ROOM), beginRoomRequest());

    const router = await renderPermalink();

    expect(pending).toHaveLength(1);
    expect(pending[0]?.issued).toBe("member");

    await act(async () => {
      pending[0]?.reject(new Error("not found"));
    });

    await vi.waitFor(() => {
      expect(router.state.location.pathname).toBe(`/r/${ROOM}`);
    });

    expect(pending).toHaveLength(1);
  });
});
