import { act, renderHook } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { sidebarFixture, sidebarRowFixture } from "../../api/testing.ts";
import { initialState } from "../../store/state.ts";
import { mutations, store } from "../../store/store.ts";
import { useAppIndicators } from "./app-shell.tsx";

const setAppBadge = vi.fn(async (_count: number) => undefined);

const clearAppBadge = vi.fn(async () => undefined);

function openApp() {
  vi.stubGlobal("navigator", { setAppBadge, clearAppBadge });

  return renderHook(() => useAppIndicators(null, null));
}

function unreadRow(id: number) {
  const row = sidebarRowFixture(id, `Room ${id}`);

  return { ...row, membership: { ...row.membership, unreadAt: row.room.createdAt } };
}

afterEach(() => {
  store.setState(initialState, true);
  vi.unstubAllGlobals();
  vi.clearAllMocks();
});

it("lowers the push badge when rooms are read and clears it when all are read", async () => {
  mutations.loadSidebar(sidebarFixture([unreadRow(1), unreadRow(2)]), 0);
  openApp();
  expect(setAppBadge).toHaveBeenLastCalledWith(2);
  await act(async () => mutations.markRoomRead(1));
  expect(setAppBadge).toHaveBeenLastCalledWith(1);
  await act(async () => mutations.markRoomRead(2));
  expect(clearAppBadge).toHaveBeenCalledOnce();
});

it("clears a stale push badge on opening an app with no unread rooms", () => {
  mutations.loadSidebar(sidebarFixture([]), 0);
  openApp();
  expect(clearAppBadge).toHaveBeenCalledOnce();
});

it("updates for cable rows, thread-only notifications, hiding, and removal", async () => {
  mutations.loadSidebar(sidebarFixture([]), 0);
  openApp();
  const row = { ...sidebarRowFixture(1, "Room"), notificationCount: 3, threadNotificationCount: 3 };
  await act(async () =>
    mutations.applyEvents(
      [{ topic: "user:1", seq: 1, type: "sidebar.row.upserted", data: row }],
      Date.now(),
    ),
  );
  expect(setAppBadge).toHaveBeenLastCalledWith(1);
  await act(async () =>
    mutations.applyEvents(
      [
        {
          topic: "user:1",
          seq: 2,
          type: "sidebar.row.upserted",
          data: { ...row, membership: { ...row.membership, involvement: "invisible" } },
        },
      ],
      Date.now(),
    ),
  );
  expect(clearAppBadge).toHaveBeenCalledTimes(2);
  await act(async () =>
    mutations.applyEvents(
      [{ topic: "user:1", seq: 3, type: "sidebar.row.upserted", data: row }],
      Date.now(),
    ),
  );
  expect(setAppBadge).toHaveBeenCalledTimes(2);
  await act(async () =>
    mutations.applyEvents(
      [{ topic: "user:1", seq: 4, type: "sidebar.row.removed", data: { roomId: 1 } }],
      Date.now(),
    ),
  );
  expect(clearAppBadge).toHaveBeenCalledTimes(3);
});

it("keeps the push badge until the sidebar snapshot is known", () => {
  openApp();
  expect(setAppBadge).not.toHaveBeenCalled();
  expect(clearAppBadge).not.toHaveBeenCalled();
});

it("does nothing on browsers without badging support", () => {
  mutations.loadSidebar(sidebarFixture([unreadRow(1)]), 0);
  vi.stubGlobal("navigator", {});
  expect(() => renderHook(() => useAppIndicators(null, null))).not.toThrow();
});

it("handles a browser rejecting a badge update", async () => {
  setAppBadge.mockRejectedValueOnce(new DOMException("Denied", "NotAllowedError"));
  mutations.loadSidebar(sidebarFixture([unreadRow(1)]), 0);
  openApp();
  await act(async () => mutations.markRoomRead(1));
  expect(clearAppBadge).toHaveBeenCalledOnce();
});
