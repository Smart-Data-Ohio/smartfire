import { afterEach, expect, it } from "vitest";
import { beginSnapshotRequest } from "./request-order.ts";
import { initialState } from "./state.ts";
import { mutations, store } from "./store.ts";

afterEach(() => store.setState(initialState, true));

it("refreshes the activity count when mute policy changes without a content revision", () => {
  mutations.setActivityUnreadCount({ unreadCount: 2, unreadRevision: 7 });
  mutations.setActivityUnreadCount(
    { unreadCount: 0, unreadRevision: 7 },
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
