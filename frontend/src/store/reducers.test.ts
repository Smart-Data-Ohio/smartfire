import { describe, expect, it } from "vitest";
import type { MessageDTO, MessagePage, PendingMessage, SidebarRow, SyncEvent } from "./model.ts";
import {
  addPending,
  applyEvents,
  applyPage,
  loadSidebar,
  markRoomRead,
  prune,
  receiveMessage,
  setRoomDetail,
} from "./reducers.ts";
import { initialState, type State, TOMBSTONE_TTL_MS, TYPING_TTL_MS } from "./state.ts";

type DistributiveOmit<T, K extends PropertyKey> = T extends unknown ? Omit<T, K> : never;

const ROOM = 4;

function message(id: number, minute: number, extra: Partial<MessageDTO> = {}): MessageDTO {
  const createdAt = `2026-10-06T09:${String(minute).padStart(2, "0")}:00.000Z`;

  return {
    id,
    roomId: ROOM,
    threadId: null,
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
    createdAt,
    updatedAt: createdAt,
    ...extra,
  };
}

function page(messages: readonly MessageDTO[], before: number | null = null): MessagePage {
  return { messages: [...messages], users: [], before, after: null, saved: [] };
}

function event(seq: number, payload: DistributiveOmit<SyncEvent, "seq">): SyncEvent {
  return { ...payload, seq };
}

function opened(messages: readonly MessageDTO[]): State {
  return applyPage(initialState, ROOM, page(messages), "replace");
}

function ids(state: State): readonly number[] {
  return state.timelines[ROOM]?.ids ?? [];
}

const pending: PendingMessage = {
  clientMessageId: "mine-1",
  roomId: ROOM,
  threadId: null,
  attachmentSignedId: null,
  attachment: null,
  creatorId: 1,
  markdownSource: "hello",
  createdAt: "2026-10-06T09:30:00.000Z",
  state: "sending",
  error: null,
};

describe("applyPage", () => {
  it("replaces, prepends and appends windows in timeline order", () => {
    const first = applyPage(initialState, ROOM, page([message(3, 3), message(4, 4)], 3), "replace");
    const older = applyPage(first, ROOM, page([message(1, 1), message(2, 2)]), "older");

    expect(ids(older)).toEqual([1, 2, 3, 4]);
    expect(older.timelines[ROOM]?.before).toBeNull();
    expect(older.timelines[ROOM]?.generation).toBe(first.timelines[ROOM]?.generation);
  });
});

describe("receiveMessage", () => {
  it("reconciles a pending send when the POST reply arrives first, then ignores the event", () => {
    const sent = addPending(opened([message(1, 1)]), pending);
    const confirmed = message(9, 30, { clientMessageId: "mine-1", creatorId: 1 });
    const afterReply = receiveMessage(sent, confirmed);

    const afterEvent = applyEvents(
      afterReply,
      [event(1, { topic: `room:${ROOM}`, type: "message.created", data: confirmed })],
      0,
    );

    expect(ids(afterEvent)).toEqual([1, 9]);
    expect(afterEvent.pending).toEqual({});
    expect(afterEvent.pendingByRoom[ROOM]).toEqual([]);
  });

  it("reconciles when the event arrives first, then ignores the POST reply", () => {
    const sent = addPending(opened([message(1, 1)]), pending);
    const confirmed = message(9, 30, { clientMessageId: "mine-1", creatorId: 1 });

    const afterEvent = applyEvents(
      sent,
      [event(1, { topic: `room:${ROOM}`, type: "message.created", data: confirmed })],
      0,
    );

    const afterReply = receiveMessage(afterEvent, confirmed);

    expect(ids(afterReply)).toEqual([1, 9]);
    expect(afterReply.pending).toEqual({});
  });

  it("leaves a window that stops short of the present alone", () => {
    const state = applyPage(initialState, ROOM, { ...page([message(1, 1)]), after: 1 }, "replace");

    expect(ids(receiveMessage(state, message(2, 2)))).toEqual([1]);
  });
});

describe("tombstones", () => {
  it("keep a removed message from coming back until they lapse", () => {
    const removed = applyEvents(
      opened([message(1, 1), message(2, 2)]),
      [
        event(1, {
          topic: `room:${ROOM}`,
          type: "message.removed",
          data: { id: 2, roomId: ROOM, threadId: null },
        }),
      ],
      1000,
    );

    const late = applyEvents(
      removed,
      [event(2, { topic: `room:${ROOM}`, type: "message.created", data: message(2, 2) })],
      1500,
    );

    expect(ids(late)).toEqual([1]);
    expect(ids(applyPage(late, ROOM, page([message(1, 1), message(2, 2)]), "replace"))).toEqual([
      1,
    ]);
    expect(prune(late, 1000 + TOMBSTONE_TTL_MS).tombstones).toEqual({});
  });
});

describe("typing", () => {
  it("lists a typist until the TTL, and drops them when they post", () => {
    const topic = `room:${ROOM}`;

    const typing = applyEvents(
      initialState,
      [event(1, { topic, type: "typing", data: { userId: 2, on: true } })],
      0,
    );

    expect(Object.keys(typing.typing[topic] ?? {})).toEqual(["2"]);
    expect(Object.keys(prune(typing, TYPING_TTL_MS).typing[topic] ?? {})).toEqual([]);

    const posted = applyEvents(
      typing,
      [event(2, { topic, type: "message.created", data: message(5, 5) })],
      10,
    );

    expect(Object.keys(posted.typing[topic] ?? {})).toEqual([]);
  });
});

describe("sidebar counts", () => {
  const row: SidebarRow = {
    room: {
      id: ROOM,
      kind: "open",
      name: "general",
      iconName: null,
      creatorId: 1,
      createdAt: "2026-10-01T00:00:00.000Z",
      updatedAt: "2026-10-01T00:00:00.000Z",
    },
    membership: {
      id: 40,
      roomId: ROOM,
      userId: 1,
      involvement: "mentions",
      unreadAt: null,
      lastReadMessageId: null,
      roomCategoryId: null,
      favoritePosition: null,
      stageRole: null,
    },
    displayName: "general",
    directMemberIds: [],
    unreadCount: 0,
    mentionCount: 0,
  };

  it("counts room.unread events and clears on read", () => {
    const loaded = loadSidebar(initialState, {
      rows: [row],
      categories: [],
      users: [],
      directPlaceholderUserIds: [],
      canCreateRooms: true,
    });

    const unread = applyEvents(
      loaded,
      [
        event(1, {
          topic: "user",
          type: "room.unread",
          data: { roomId: ROOM, messageId: 7, mentioned: false },
        }),
        event(2, {
          topic: "user",
          type: "room.unread",
          data: { roomId: ROOM, messageId: 8, mentioned: true },
        }),
      ],
      0,
    );

    expect(unread.sidebar.rows[ROOM]).toMatchObject({ unreadCount: 2, mentionCount: 1 });
    expect(unread.sidebar.rows[ROOM]?.membership.unreadAt).not.toBeNull();

    const read = markRoomRead(unread, ROOM);

    expect(read.sidebar.rows[ROOM]).toMatchObject({ unreadCount: 0, mentionCount: 0 });
    expect(read.sidebar.rows[ROOM]?.membership.unreadAt).toBeNull();
  });

  it("seeds the timeline's unread divider from the room detail", () => {
    const state = setRoomDetail(initialState, {
      room: row.room,
      membership: row.membership,
      displayName: "general",
      memberCount: 3,
      pinsCount: 0,
      directMemberIds: [],
      memberPreviewIds: [],
      users: [],
      unread: { firstUnreadMessageId: 12, count: 5 },
    });

    expect(state.timelines[ROOM]).toMatchObject({ unreadFromId: 12, unreadCount: 5 });
  });
});
