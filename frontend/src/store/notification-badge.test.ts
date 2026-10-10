import { afterEach, expect, it } from "vitest";
import { beginSnapshotRequest } from "./request-order.ts";
import { initialState } from "./state.ts";
import { mutations, store } from "./store.ts";

afterEach(() => store.setState(initialState, true));

it("accepts a count at the newest server revision regardless of request-start order", () => {
  const first = beginSnapshotRequest();
  const second = beginSnapshotRequest();
  mutations.setActivityUnreadCount({ unreadCount: 0, unreadRevision: 8 }, undefined, second);
  mutations.setActivityUnreadCount({ unreadCount: 1, unreadRevision: 8 }, undefined, first);
  expect(store.getState().activity.unreadCount).toBe(1);
});

it("orders mute-policy counts by server revision", () => {
  mutations.setActivityUnreadCount({ unreadCount: 2, unreadRevision: 7 });
  mutations.setActivityUnreadCount(
    { unreadCount: 0, unreadRevision: 8 },
    store.getState().activity.generation,
    beginSnapshotRequest(),
  );
  expect(store.getState().activity.unreadCount).toBe(0);
  mutations.setActivityUnreadCount({ unreadCount: 2, unreadRevision: 7 });
  expect(store.getState().activity.unreadCount).toBe(0);
  mutations.setActivityUnreadCount(
    { unreadCount: 9, unreadRevision: 6 },
    store.getState().activity.generation,
    beginSnapshotRequest(),
  );
  expect(store.getState().activity.unreadCount).toBe(0);
});
