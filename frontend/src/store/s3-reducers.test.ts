import { describe, expect, it } from "vitest";
import type { ActivityEventType } from "../gen/ActivityEventType.ts";
import type { ActivityItem } from "../gen/ActivityItem.ts";
import type { ActivityList } from "../gen/ActivityList.ts";
import type { ActivitySourceType } from "../gen/ActivitySourceType.ts";
import type { ActivityState } from "../gen/ActivityState.ts";
import type { SavedItem } from "../gen/SavedItem.ts";
import type { SavedItemList } from "../gen/SavedItemList.ts";
import type { ScheduledMessage } from "../gen/ScheduledMessage.ts";
import type { ScheduledMessageList } from "../gen/ScheduledMessageList.ts";
import type { User } from "../gen/User.ts";
import {
  activityDestination,
  activityListOf,
  activityLoadStart,
  applyActivityItem,
  beginActivityGeneration,
  endActivityChange,
  landActivityPage,
  markActivityStale,
  nextActivityItem,
  removeActivityItem,
  setActivityListLoading,
  setActivityUnreadCount,
  showActivityChange,
  unreadDelta,
} from "./activity.ts";
import { conversationNameOf } from "./conversations.ts";
import type { MessageDTO, SyncEvent } from "./model.ts";
import {
  emptyPagedList,
  type IdOrder,
  type PagedList,
  pagedFailed,
  pagedLanded,
  pagedLoading,
  pagedPlaced,
  pagedStale,
} from "./paged-list.ts";
import { applyEvents } from "./reducers.ts";
import {
  applySavedChange,
  landSavedPage,
  markSavedStale,
  savedItemForMessage,
  savedListOf,
} from "./saved-list.ts";
import {
  applyScheduled,
  landScheduledPage,
  removeScheduled,
  roomScheduledKey,
  scheduledListOf,
  scheduledQueryOf,
} from "./scheduled.ts";
import { initialState, type State } from "./state.ts";

type DistributiveOmit<T, K extends PropertyKey> = T extends unknown ? Omit<T, K> : never;

const ROOM = 4;

const OTHER_ROOM = 9;

const NOW = Date.UTC(2026, 9, 6, 16, 30, 0);

/** A time on the test day, `minute` minutes past 09:00. */
function at(minute: number): string {
  return new Date(Date.UTC(2026, 9, 6, 9, minute, 0)).toISOString();
}

function events(state: State, ...payloads: DistributiveOmit<SyncEvent, "seq">[]): State {
  const batch = payloads.map((payload, index) => ({ ...payload, seq: index + 1 }));

  // SAFETY: each payload is a SyncEvent without its `seq`; adding it back restores the union.
  return applyEvents(state, batch as SyncEvent[], NOW);
}

function user(id: number): User {
  return {
    id,
    name: `Person ${id}`,
    role: "member",
    status: "active",
    bio: null,
    avatarUrl: `/avatars/${id}`,
    hasAvatar: true,
    avatarIcon: null,
    customStatus: null,
    agent: null,
    createdAt: at(0),
    updatedAt: at(0),
  };
}

// The inbox.

function item(
  id: number,
  minute: number,
  eventType: ActivityEventType = "mention",
  state: ActivityState = "unread",
): ActivityItem {
  const sourceType: ActivitySourceType =
    eventType === "huddle_started" ? "huddle_grant" : "message";

  return {
    id,
    eventType,
    state,
    readAt: state === "unread" ? null : at(minute),
    handledAt: state === "handled" ? at(minute) : null,
    createdAt: at(minute),
    updatedAt: at(minute),
    source: {
      sourceType,
      sourceId: id * 10,
      roomId: ROOM,
      threadId: null,
      messageId: id * 10,
      eventId: null,
      creatorId: 2,
      title: "general",
      body: `Item ${id}`,
      occurredAt: at(minute),
      approvalStatus: null,
      budgetCap: null,
      path: `/rooms/${ROOM}/@${id * 10}`,
    },
  };
}

function activityPage(
  items: readonly ActivityItem[],
  unreadCount: number,
  nextCursor: string | null = null,
  unreadRevision = 1,
): ActivityList {
  return { items: [...items], users: [user(2)], unreadCount, unreadRevision, nextCursor };
}

/** The inbox with Unread, Read and Handled loaded in All, and Unread loaded in Mentions. */
function inbox(): State {
  const unread = [item(5, 50), item(4, 40, "huddle_started"), item(3, 30)];
  const read = [item(2, 20, "mention", "read")];
  const handled = [item(1, 10, "mention", "handled")];

  let state = landActivityPage(initialState, "all", "unread", activityPage(unread, 3), "replace");

  state = landActivityPage(state, "all", "read", activityPage(read, 3), "replace");
  state = landActivityPage(state, "all", "handled", activityPage(handled, 3), "replace");

  return landActivityPage(
    state,
    "mentions",
    "unread",
    activityPage([item(5, 50), item(3, 30)], 3),
    "replace",
  );
}

function ids(state: State, tab: "all" | "mentions", status: ActivityState): readonly number[] {
  return activityListOf(state, tab, status).ids;
}

// Saved items.

function message(id: number, roomId = ROOM, threadId: number | null = null): MessageDTO {
  const createdAt = at(id % 60);

  return {
    id,
    roomId,
    threadId,
    creatorId: 2,
    clientMessageId: `client-${id}`,
    bodyHtml: `<p>${id}</p>`,
    markdownSource: `${id}`,
    systemNote: false,
    action: false,
    streaming: false,
    embedsSuppressed: false,
    replyToMessageId: null,
    forwardedFromMessageId: null,
    forwardedAt: null,
    forwardNote: null,
    editedAt: null,
    attachment: null,
    reactions: [],
    boosts: [],
    pinned: false,
    thread: null,
    poll: null,
    cards: [],
    cardsAsOf: createdAt,
    steps: [],
    createdAt,
    updatedAt: createdAt,
  };
}

function savedItem(id: number, messageId: number, minute: number, done = false): SavedItem {
  return {
    id,
    messageId,
    status: done ? "done" : "in_progress",
    remindAt: null,
    remindedAt: null,
    createdAt: at(minute),
  };
}

function savedPage(items: readonly SavedItem[], nextCursor: string | null = null): SavedItemList {
  return {
    items: [...items],
    messages: items.map((saved) => message(saved.messageId)),
    users: [user(2)],
    conversations: [
      {
        roomId: ROOM,
        threadId: null,
        roomKind: "open",
        roomName: "general",
        roomIconName: null,
        threadName: null,
      },
    ],
    nextCursor,
  };
}

/** Saved items 3 (newest), 2 and 1, all in progress; All and In progress loaded, Done empty. */
function saved(): State {
  const items = [savedItem(3, 103, 30), savedItem(2, 102, 20), savedItem(1, 101, 10)];

  let state = landSavedPage(initialState, "all", savedPage(items), "replace");

  state = landSavedPage(state, "in_progress", savedPage(items), "replace");

  return landSavedPage(state, "done", savedPage([]), "replace");
}

// Scheduled messages.

function scheduled(
  id: number,
  minute: number,
  roomId = ROOM,
  extra: Partial<ScheduledMessage> = {},
): ScheduledMessage {
  return {
    id,
    roomId,
    threadId: null,
    replyToMessageId: null,
    markdownSource: `Later ${id}`,
    sendAt: at(minute),
    state: "pending",
    sendable: true,
    sentAt: null,
    sentMessageId: null,
    droppedAt: null,
    dropReason: null,
    createdAt: at(0),
    ...extra,
  };
}

function scheduledPage(messages: readonly ScheduledMessage[]): ScheduledMessageList {
  return { scheduledMessages: [...messages], conversations: [], nextCursor: null };
}

/** Pending 1 (soonest), 2 (other room) and 3; Past holds 9 (sent) and 8 (dropped). */
function schedule(): State {
  const pending = [scheduled(1, 10), scheduled(2, 20, OTHER_ROOM), scheduled(3, 30)];

  const past = [
    scheduled(9, 5, ROOM, { state: "sent", sendable: false, sentAt: at(5), sentMessageId: 900 }),
    scheduled(8, 2, ROOM, { state: "dropped", sendable: false, droppedAt: at(2) }),
  ];

  let state = landScheduledPage(initialState, "pending", scheduledPage(pending), "replace");

  state = landScheduledPage(state, "past", scheduledPage(past), "replace");

  return landScheduledPage(
    state,
    roomScheduledKey(ROOM),
    scheduledPage([scheduled(1, 10), scheduled(3, 30)]),
    "replace",
  );
}

describe("a keyset-paged list", () => {
  const ascending: IdOrder = (left, right) => left - right;

  function ready(ids: readonly number[], nextCursor: string | null): PagedList {
    return pagedLanded(emptyPagedList, ids, nextCursor, "replace");
  }

  it("places a live id at its sorted place inside the loaded window", () => {
    expect(pagedPlaced(ready([2, 4, 6], "c"), 3, true, ascending).ids).toEqual([2, 3, 4, 6]);
    expect(pagedPlaced(ready([2, 4, 6], "c"), 1, true, ascending).ids).toEqual([1, 2, 4, 6]);
  });

  it("leaves an id past the last loaded row to a later page, but appends on the last page", () => {
    expect(pagedPlaced(ready([2, 4, 6], "c"), 7, true, ascending).ids).toEqual([2, 4, 6]);
    expect(pagedPlaced(ready([2, 4, 6], null), 7, true, ascending).ids).toEqual([2, 4, 6, 7]);
  });

  it("drops an id that no longer belongs, and keeps the same list when nothing moves", () => {
    const list = ready([2, 4, 6], null);

    expect(pagedPlaced(list, 4, false, ascending).ids).toEqual([2, 6]);
    expect(pagedPlaced(list, 4, true, ascending)).toBe(list);
    expect(pagedPlaced(list, 5, false, ascending)).toBe(list);
  });

  it("never adds to a list that hasn't loaded", () => {
    expect(pagedPlaced(emptyPagedList, 3, true, ascending)).toBe(emptyPagedList);
  });

  it("skips ids a next page repeats, and a reload clears stale", () => {
    const stale = pagedStale(ready([2, 4], "c"));
    const more = pagedLanded(stale, [4, 6], null, "more");

    expect(more).toMatchObject({ ids: [2, 4, 6], nextCursor: null, stale: true });
    expect(pagedLanded(stale, [2], null, "replace").stale).toBe(false);
    expect(pagedStale(emptyPagedList)).toBe(emptyPagedList);
  });

  it("lands a page or failure only in the generation its load started in", () => {
    const list = ready([2, 4], "c");
    const started = list.generation;
    const reloading = pagedLoading(list);

    expect(reloading.generation).toBe(started + 1);
    expect(pagedLanded(reloading, [6], null, "more", started)).toBe(reloading);
    expect(pagedFailed(reloading, "Down", started)).toBe(reloading);
    expect(pagedLanded(reloading, [6], null, "replace", reloading.generation).ids).toEqual([6]);
  });
});

describe("the activity inbox", () => {
  it("lands a page: the items, the people, the cursor and the server's unread count", () => {
    const state = landActivityPage(
      initialState,
      "all",
      "unread",
      activityPage([item(2, 20), item(1, 10)], 7, "next"),
      "replace",
    );

    expect(activityListOf(state, "all", "unread")).toMatchObject({
      ids: [2, 1],
      nextCursor: "next",
      status: "ready",
    });
    expect(state.activity.unreadCount).toBe(7);
    expect(state.users[2]?.name).toBe("Person 2");
    expect(activityListOf(state, "all", "read").status).toBe("idle");
  });

  it("moves a handled item from Unread to its place in Handled, in every tab it's in", () => {
    const state = inbox();
    const before = state.activity.items[3];

    if (before === undefined) throw new Error("no item 3");

    const handled = nextActivityItem(before, "handled", at(55));

    const next = events(state, {
      topic: "user",
      type: "activity.item",
      data: { item: handled, unreadCount: 2, unreadRevision: 2 },
    });

    expect(ids(next, "all", "unread")).toEqual([5, 4]);
    expect(ids(next, "mentions", "unread")).toEqual([5]);
    expect(ids(next, "all", "handled")).toEqual([3, 1]);
    expect(activityListOf(next, "mentions", "handled").status).toBe("idle");
    expect(next.activity.unreadCount).toBe(2);
  });

  it("puts a new arrival at the top of the lists of its tab, and nowhere else", () => {
    const next = applyActivityItem(inbox(), item(6, 59, "huddle_started"), {
      unreadCount: 4,
      unreadRevision: 2,
    });

    expect(ids(next, "all", "unread")).toEqual([6, 5, 4, 3]);
    expect(ids(next, "mentions", "unread")).toEqual([5, 3]);
    expect(next.activity.unreadCount).toBe(4);
  });

  it("keeps the version of a confirmed row installed during reconnect", () => {
    const before = item(5, 50);
    const optimistic = nextActivityItem(before, "read", at(55));
    let state = showActivityChange(inbox(), optimistic, 1, -1);

    state = showActivityChange(state, nextActivityItem(item(3, 30), "read", at(55)), 2, -1);
    const start = activityLoadStart(state, "all", "read");
    const confirmed = item(5, 60, "mention", "read");

    state = setActivityListLoading(state, "all", "read", false);
    state = landActivityPage(
      state,
      "all",
      "read",
      activityPage([confirmed], 2, null, 2),
      "replace",
      start,
    );
    expect(state.activity.items[5]).toBe(optimistic);
    state = beginActivityGeneration(state, false);
    expect(state.activity.items[5]).toEqual(confirmed);
    expect(state.activity.versions[5]).toBe(confirmed.updatedAt);
    state = applyActivityItem(state, item(5, 56), null);
    expect(state.activity.items[5]).toEqual(confirmed);
  });

  it("keeps a newer reply item when its count snapshot is stale", () => {
    const before = item(5, 50);
    const optimistic = nextActivityItem(before, "read", at(55));
    let state = showActivityChange(inbox(), optimistic, 1, -1);

    state = applyActivityItem(state, item(5, 50, "mention", "read"), {
      unreadCount: 2,
      unreadRevision: 2,
    });
    state = setActivityUnreadCount(state, { unreadCount: 8, unreadRevision: 5 });
    const reply = item(5, 60, "mention", "handled");

    state = endActivityChange(state, {
      generation: state.activity.generation,
      token: 1,
      optimistic,
      settled: reply,
      unread: { unreadCount: 2, unreadRevision: 3 },
    });
    expect(state.activity.items[5]).toEqual(reply);
    expect(state.activity.unreadCount).toBe(8);
  });

  it("reconciles the newest zero-delta transition without changing an earlier read adjustment", () => {
    const before = item(5, 50);
    const optimistic = nextActivityItem(before, "read", at(55));
    let state = showActivityChange(inbox(), optimistic, 1, -1);

    state = showActivityChange(state, nextActivityItem(item(3, 30), "read", at(55)), 2, -1);
    const read = item(5, 50, "mention", "read");

    state = applyActivityItem(state, read, { unreadCount: 2, unreadRevision: 2 });
    state = endActivityChange(state, {
      generation: state.activity.generation,
      token: 1,
      optimistic,
      settled: before,
      unread: null,
    });
    const handled = nextActivityItem(read, "handled", at(55));

    state = showActivityChange(state, handled, 3, 0);
    state = applyActivityItem(state, item(5, 50, "mention", "handled"), {
      unreadCount: 2,
      unreadRevision: 3,
    });
    expect(state.activity.unreadCount).toBe(1);
    state = applyActivityItem(state, before, { unreadCount: 3, unreadRevision: 4 });
    expect(state.activity.pendingUnread[1]?.delta).toBe(-1);
    expect(state.activity.pendingUnread[3]?.delta).toBe(1);
    expect(state.activity.unreadCount).toBe(2);
    state = endActivityChange(state, {
      generation: state.activity.generation,
      token: 3,
      optimistic: handled,
      settled: read,
      unread: null,
    });
    expect(state.activity.items[5]).toEqual(before);
    expect(state.activity.unreadCount).toBe(2);
  });

  for (const latestState of ["handled", "unread"] as const) {
    it(`keeps the latest equal-timestamp ${latestState} observation on failure`, () => {
      const before = item(5, 50);
      const optimistic = nextActivityItem(before, "read", at(55));
      let state = showActivityChange(inbox(), optimistic, 1, -1);

      state = showActivityChange(state, nextActivityItem(item(3, 30), "read", at(55)), 2, -1);
      state = applyActivityItem(state, item(5, 50, "mention", "read"), {
        unreadCount: 2,
        unreadRevision: 2,
      });
      const latest = item(5, 50, "mention", latestState);
      const unreadCount = latestState === "unread" ? 3 : 2;

      state = applyActivityItem(state, latest, { unreadCount, unreadRevision: 3 });
      state = applyActivityItem(state, item(5, 50, "mention", "read"), {
        unreadCount: 2,
        unreadRevision: 2,
      });
      state = endActivityChange(state, {
        generation: state.activity.generation,
        token: 1,
        optimistic,
        settled: before,
        unread: null,
      });
      expect(state.activity.items[5]).toEqual(latest);
      expect(state.activity.unreadCount).toBe(latestState === "unread" ? 2 : 1);
    });
  }

  it("leaves an older item past a loaded window to the page that brings it", () => {
    const state = landActivityPage(
      initialState,
      "all",
      "read",
      activityPage([item(5, 50, "mention", "read")], 0, "more"),
      "replace",
    );

    const next = applyActivityItem(state, item(2, 20, "mention", "read"), {
      unreadCount: 0,
      unreadRevision: 2,
    });

    expect(ids(next, "all", "read")).toEqual([5]);
    expect(next.activity.items[2]).toBeDefined();
  });

  it("takes the server's count from each change, and keeps the badge on an optimistic one", () => {
    const state = inbox();
    const read = item(3, 30, "mention", "read");

    expect(
      applyActivityItem(state, read, { unreadCount: 11, unreadRevision: 2 }).activity.unreadCount,
    ).toBe(11);
    expect(applyActivityItem(state, read, null).activity.unreadCount).toBe(3);
    expect(
      setActivityUnreadCount(state, { unreadCount: 3, unreadRevision: 2 }).activity.unreadCount,
    ).toBe(3);
    expect(
      setActivityUnreadCount(state, { unreadCount: 0, unreadRevision: 2 }).activity.unreadCount,
    ).toBe(0);
  });

  it("removes an item gone with its source from the store and every list", () => {
    const next = events(inbox(), {
      topic: "user",
      type: "activity.removed",
      data: { id: 5, unreadCount: 2, unreadRevision: 2 },
    });

    expect(next.activity.items[5]).toBeUndefined();
    expect(ids(next, "all", "unread")).toEqual([4, 3]);
    expect(ids(next, "mentions", "unread")).toEqual([3]);
    expect(next.activity.unreadCount).toBe(2);
  });

  it("invalidates a delayed count even when removal of an unseen item leaves the count unchanged", () => {
    const state = inbox();
    const next = removeActivityItem(state, 999, { unreadCount: 3, unreadRevision: 2 });

    expect(
      setActivityUnreadCount(next, { unreadCount: 10, unreadRevision: 1 }).activity.unreadCount,
    ).toBe(3);
  });

  it("accepts revision zero first and ignores equal or older count snapshots", () => {
    const first = setActivityUnreadCount(initialState, { unreadCount: 2, unreadRevision: 0 });
    const newer = setActivityUnreadCount(first, { unreadCount: 4, unreadRevision: 3 });

    expect(first.activity.serverUnread).toEqual({ unreadCount: 2, unreadRevision: 0 });
    expect(
      setActivityUnreadCount(newer, { unreadCount: 0, unreadRevision: 3 }).activity.unreadCount,
    ).toBe(4);
    expect(
      setActivityUnreadCount(newer, { unreadCount: 9, unreadRevision: 2 }).activity.unreadCount,
    ).toBe(4);
  });

  it("discards deferred counts at a generation fence and accepts a lower new count", () => {
    const before = item(3, 30);

    const optimistic = nextActivityItem(before, "read", at(55));
    const held = setActivityUnreadCount(inbox(), { unreadCount: 8, unreadRevision: 100 });
    const pending = showActivityChange(held, optimistic, 1, -1);
    const queued = setActivityUnreadCount(pending, { unreadCount: 7, unreadRevision: 101 });

    expect(queued.activity.unreadCount).toBe(7);

    const restored = beginActivityGeneration(queued);

    expect(restored.activity.deferredUnread).toBeNull();

    const nextPending = showActivityChange(restored, optimistic, 2, -1);
    const nextQueued = setActivityUnreadCount(nextPending, { unreadCount: 2, unreadRevision: 50 });

    const settled = endActivityChange(nextQueued, {
      generation: restored.activity.generation,
      token: 2,
      optimistic,
      settled: optimistic,
      unread: { unreadCount: 1, unreadRevision: 49 },
    });

    expect(settled.activity.unreadCount).toBe(2);
    expect(settled.activity.serverUnread?.unreadRevision).toBe(50);
    expect(settled.activity.deferredUnread).toBeNull();
  });

  it("keeps a newer count when an old page lands, without invalidating a newer GET", () => {
    const state = inbox();
    const start = activityLoadStart(state, "all", "unread");
    const newer = setActivityUnreadCount(state, { unreadCount: 4, unreadRevision: 2 });

    const paged = landActivityPage(
      newer,
      "all",
      "unread",
      activityPage([item(5, 50)], 3),
      "replace",
      start,
    );

    const refreshed = setActivityUnreadCount(paged, { unreadCount: 5, unreadRevision: 3 });

    expect(paged.activity.serverUnread).toEqual({ unreadCount: 4, unreadRevision: 2 });
    expect(refreshed.activity.unreadCount).toBe(5);
  });

  it("accepts a newer page count even when its list generation has been superseded", () => {
    const state = inbox();
    const start = activityLoadStart(state, "all", "unread");
    const reloading = setActivityListLoading(state, "all", "unread", false);

    const next = landActivityPage(
      reloading,
      "all",
      "unread",
      activityPage([item(99, 59)], 4, null, 2),
      "replace",
      start,
    );

    expect(next.activity.unreadCount).toBe(4);
    expect(next.activity.items[99]).toBeUndefined();
    expect(ids(next, "all", "unread")).toEqual([5, 4, 3]);
  });

  it("ignores conflicting equal or older websocket counts independently of item timestamps", () => {
    const state = setActivityUnreadCount(inbox(), { unreadCount: 4, unreadRevision: 3 });

    const changed = events(
      state,
      {
        topic: "user",
        type: "activity.item",
        data: { item: item(3, 59, "mention", "read"), unreadCount: 0, unreadRevision: 2 },
      },
      {
        topic: "user",
        type: "activity.removed",
        data: { id: 5, unreadCount: 9, unreadRevision: 3 },
      },
    );

    expect(changed.activity.items[3]?.state).toBe("read");
    expect(changed.activity.items[5]).toBeUndefined();
    expect(changed.activity.unreadCount).toBe(4);
  });

  it("marks only loaded lists stale, and keeps the rows shown while one reloads", () => {
    const loading = setActivityListLoading(markActivityStale(inbox()), "all", "unread", false);

    expect(activityListOf(loading, "all", "unread")).toMatchObject({
      status: "ready",
      stale: true,
      ids: [5, 4, 3],
    });
    expect(activityListOf(loading, "huddles", "unread")).toBe(emptyPagedList);
  });

  it("applies each action as the server does", () => {
    const unread = item(1, 10);
    const read = nextActivityItem(unread, "read", at(20));
    const handled = nextActivityItem(read, "handled", at(30));

    expect(read).toMatchObject({ state: "read", readAt: at(20), updatedAt: at(20) });
    expect(handled).toMatchObject({ state: "handled", readAt: at(20), handledAt: at(30) });
    expect(nextActivityItem(handled, "read", at(40))).toBe(handled);
    expect(nextActivityItem(handled, "unhandled", at(40))).toMatchObject({
      state: "read",
      readAt: at(20),
      handledAt: null,
    });
    expect(nextActivityItem(handled, "unread", at(40))).toMatchObject({
      state: "unread",
      readAt: null,
      handledAt: null,
    });
    expect(nextActivityItem(unread, "handled", at(50))).toMatchObject({
      readAt: at(50),
      handledAt: at(50),
    });
    expect(unreadDelta(unread, read)).toBe(-1);
    expect(unreadDelta(read, unread)).toBe(1);
    expect(unreadDelta(read, handled)).toBe(0);
  });

  it("opens messages, rooms and the scheduled page in the app, and the rest on classic pages", () => {
    const mention = item(1, 10);
    const huddle = item(2, 20, "huddle_started");

    const dropped: ActivityItem = {
      ...mention,
      eventType: "scheduled_message_dropped",
      source: { ...mention.source, sourceType: "scheduled_message", messageId: null },
    };

    const approval: ActivityItem = {
      ...mention,
      eventType: "agent_approval_request",
      source: { ...mention.source, sourceType: "agent_approval", path: "/agents/1/approvals" },
    };

    expect(activityDestination(mention)).toEqual({
      kind: "message",
      roomId: ROOM,
      messageId: 10,
      threadId: null,
    });
    expect(activityDestination(huddle)).toEqual({ kind: "room", roomId: ROOM });
    expect(activityDestination(dropped)).toEqual({ kind: "scheduled" });
    expect(activityDestination(approval)).toEqual({ kind: "classic", path: "/agents/1/approvals" });
  });
});

describe("the saved list", () => {
  it("lands a page: items, marks, messages, people and conversation names", () => {
    const state = saved();

    expect(savedListOf(state, "all").ids).toEqual([3, 2, 1]);
    expect(state.saved[102]).toBe(2);
    expect(savedItemForMessage(state, 102)?.id).toBe(2);
    expect(state.messages[101]?.id).toBe(101);
    expect(state.users[2]).toBeDefined();
    expect(conversationNameOf(state, ROOM, null)?.roomName).toBe("general");
  });

  it("moves an item marked done from In progress to Done, keeping its place in All", () => {
    const next = events(saved(), {
      topic: "user",
      type: "saved.changed",
      data: { messageId: 102, item: savedItem(2, 102, 20, true) },
    });

    expect(savedListOf(next, "in_progress").ids).toEqual([3, 1]);
    expect(savedListOf(next, "done").ids).toEqual([2]);
    expect(savedListOf(next, "all").ids).toEqual([3, 2, 1]);
    expect(savedItemForMessage(next, 102)?.status).toBe("done");
  });

  it("clears the mark and the item everywhere when a message is unsaved", () => {
    const next = applySavedChange(saved(), 103, null);

    expect(next.saved[103]).toBeUndefined();
    expect(next.savedList.items[3]).toBeUndefined();
    expect(savedListOf(next, "all").ids).toEqual([2, 1]);
    expect(savedListOf(next, "in_progress").ids).toEqual([2, 1]);
  });

  it("replaces the old item when a message is saved again under a new id", () => {
    const next = applySavedChange(saved(), 101, savedItem(7, 101, 40));

    expect(next.saved[101]).toBe(7);
    expect(next.savedList.items[1]).toBeUndefined();
    expect(savedListOf(next, "all").ids).toEqual([7, 3, 2]);
  });

  it("marks the lists stale for a newly saved message the store doesn't hold", () => {
    const next = applySavedChange(saved(), 555, savedItem(8, 555, 50));

    expect(next.saved[555]).toBe(8);
    expect(savedListOf(next, "all")).toMatchObject({ ids: [3, 2, 1], stale: true });
    expect(savedListOf(next, "in_progress").stale).toBe(true);
    expect(savedListOf(next, "done").stale).toBe(false);
  });

  it("drops a deleted message's item, and marks every loaded list stale on a gap", () => {
    const next = events(saved(), {
      topic: `room:${ROOM}`,
      type: "message.removed",
      data: { id: 102, roomId: ROOM, threadId: null },
    });

    expect(next.savedList.items[2]).toBeUndefined();
    expect(savedListOf(next, "all").ids).toEqual([3, 1]);
    expect(savedListOf(markSavedStale(next), "done").stale).toBe(true);
  });
});

describe("the scheduled lists", () => {
  it("moves a sent message from the pending lists to its place in Past", () => {
    const sent = scheduled(3, 30, ROOM, {
      state: "sent",
      sendable: false,
      sentAt: at(31),
      sentMessageId: 901,
    });

    const next = events(schedule(), { topic: "user", type: "scheduled.changed", data: sent });

    expect(scheduledListOf(next, "pending").ids).toEqual([1, 2]);
    expect(scheduledListOf(next, roomScheduledKey(ROOM)).ids).toEqual([1]);
    expect(scheduledListOf(next, "past").ids).toEqual([3, 9, 8]);
    expect(next.scheduled.items[3]?.sentMessageId).toBe(901);
  });

  it("keeps a sending one pending, and re-sorts a pending one moved to a new time", () => {
    const sending = applyScheduled(schedule(), scheduled(1, 10, ROOM, { state: "sending" }));
    const moved = applyScheduled(schedule(), scheduled(1, 40));

    expect(scheduledListOf(sending, "pending").ids).toEqual([1, 2, 3]);
    expect(scheduledListOf(moved, "pending").ids).toEqual([2, 3, 1]);
    expect(scheduledListOf(moved, roomScheduledKey(ROOM)).ids).toEqual([3, 1]);
  });

  it("puts a new one in the room's list and the pending list, not another room's", () => {
    const next = applyScheduled(schedule(), scheduled(4, 15, OTHER_ROOM));

    expect(scheduledListOf(next, "pending").ids).toEqual([1, 4, 2, 3]);
    expect(scheduledListOf(next, roomScheduledKey(ROOM)).ids).toEqual([1, 3]);
    expect(scheduledListOf(next, roomScheduledKey(OTHER_ROOM)).status).toBe("idle");
  });

  it("removes a cancelled one from the store and every list", () => {
    const next = events(schedule(), {
      topic: "user",
      type: "scheduled.removed",
      data: { id: 1, roomId: ROOM },
    });

    expect(next.scheduled.items[1]).toBeUndefined();
    expect(scheduledListOf(next, "pending").ids).toEqual([2, 3]);
    expect(scheduledListOf(next, roomScheduledKey(ROOM)).ids).toEqual([3]);
    expect(removeScheduled(next, 1)).toBe(next);
  });

  it("asks the server for a list's filter and room", () => {
    expect(scheduledQueryOf("past")).toEqual({ status: "past", roomId: null });
    expect(scheduledQueryOf(roomScheduledKey(12))).toEqual({ status: "pending", roomId: 12 });
  });
});
