import { afterEach, expect, it } from "vitest";
import { sidebarFixture, sidebarRowFixture } from "../api/testing.ts";
import { beginSnapshotRequest } from "./request-order.ts";
import { initialState } from "./state.ts";
import { mutations, store } from "./store.ts";

afterEach(() => store.setState(initialState, true));

it("keeps the expired-mute count when an earlier equal-revision count arrives last", () => {
  mutations.setActivityUnreadCount(
    { unreadCount: 1, unreadRevision: 8, evaluatedAt: "2026-10-10T12:15:00.000000000Z" },
    undefined,
    beginSnapshotRequest(),
  );
  mutations.setActivityUnreadCount(
    { unreadCount: 0, unreadRevision: 8, evaluatedAt: "2026-10-10T12:14:59.999999999Z" },
    undefined,
    beginSnapshotRequest(),
  );
  expect(store.getState().activity.unreadCount).toBe(1);
});

it("keeps sidebar counts restored at expiry over delayed equal-revision rows", () => {
  const row = {
    ...sidebarRowFixture(4, "general"),
    revision: 8,
    evaluatedAt: "2026-10-10T12:15:00.000000000Z",
    notificationCount: 1,
  };

  mutations.loadSidebar(sidebarFixture([row]), 0);
  mutations.loadSidebar(
    sidebarFixture([
      { ...row, evaluatedAt: "2026-10-10T12:14:59.999999999Z", notificationCount: 0 },
    ]),
    0,
  );
  expect(store.getState().sidebar.rows[4]?.notificationCount).toBe(1);
});

it("prioritizes persisted revisions over evaluation time", () => {
  mutations.setActivityUnreadCount({
    unreadCount: 0,
    unreadRevision: 8,
    evaluatedAt: "2026-10-10T12:15:00.000000000Z",
  });
  mutations.setActivityUnreadCount({
    unreadCount: 1,
    unreadRevision: 9,
    evaluatedAt: "2026-10-10T12:00:00.000000000Z",
  });
  expect(store.getState().activity.unreadCount).toBe(1);
  mutations.setActivityUnreadCount({
    unreadCount: 0,
    unreadRevision: 8,
    evaluatedAt: "2026-10-10T13:00:00.000000000Z",
  });
  expect(store.getState().activity.unreadCount).toBe(1);
});

it("orders sync count evaluations without HTTP request tickets", () => {
  mutations.setActivityUnreadCount({
    unreadCount: 0,
    unreadRevision: 8,
    evaluatedAt: "2026-10-10T12:14:59.999999999Z",
  });
  mutations.setActivityUnreadCount({
    unreadCount: 1,
    unreadRevision: 8,
    evaluatedAt: "2026-10-10T12:15:00.000000000Z",
  });
  mutations.setActivityUnreadCount({
    unreadCount: 0,
    unreadRevision: 8,
    evaluatedAt: "2026-10-10T12:14:59.999999999Z",
  });
  expect(store.getState().activity.unreadCount).toBe(1);
});

it("accepts a count at the newest server revision regardless of request-start order", () => {
  const first = beginSnapshotRequest();
  const second = beginSnapshotRequest();
  mutations.setActivityUnreadCount(
    { unreadCount: 0, unreadRevision: 8, evaluatedAt: "2026-10-10T12:00:00.000000000Z" },
    undefined,
    second,
  );
  mutations.setActivityUnreadCount(
    { unreadCount: 1, unreadRevision: 8, evaluatedAt: "2026-10-10T12:00:00.000000000Z" },
    undefined,
    first,
  );
  expect(store.getState().activity.unreadCount).toBe(1);
});

it("orders mute-policy counts by server revision", () => {
  mutations.setActivityUnreadCount({
    unreadCount: 2,
    unreadRevision: 7,
    evaluatedAt: "2026-10-10T12:00:00.000000000Z",
  });
  mutations.setActivityUnreadCount(
    { unreadCount: 0, unreadRevision: 8, evaluatedAt: "2026-10-10T12:00:00.000000000Z" },
    store.getState().activity.generation,
    beginSnapshotRequest(),
  );
  expect(store.getState().activity.unreadCount).toBe(0);
  mutations.setActivityUnreadCount({
    unreadCount: 2,
    unreadRevision: 7,
    evaluatedAt: "2026-10-10T12:00:00.000000000Z",
  });
  expect(store.getState().activity.unreadCount).toBe(0);
  mutations.setActivityUnreadCount(
    { unreadCount: 9, unreadRevision: 6, evaluatedAt: "2026-10-10T12:00:00.000000000Z" },
    store.getState().activity.generation,
    beginSnapshotRequest(),
  );
  expect(store.getState().activity.unreadCount).toBe(0);
});
