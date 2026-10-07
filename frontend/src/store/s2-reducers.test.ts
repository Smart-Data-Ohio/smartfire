import { describe, expect, it } from "vitest";
import type { Thread } from "../gen/Thread.ts";
import { toggledReactions } from "./message-extras.ts";
import type { MessageDTO, MessagePage, PendingMessage, SyncEvent } from "./model.ts";
import {
  addPending,
  applyEvents,
  applyPage,
  applyThreadPage,
  moveUnreadDivider,
  receiveMessage,
  removeMessage,
  setPageFailed,
  setPageReplacing,
  setRoomDetail,
  setThreadPageReplacing,
} from "./reducers.ts";
import { initialState, type State } from "./state.ts";
import { loadThreadList, setThreadListFailed, setThreadListLoading } from "./threads.ts";

type DistributiveOmit<T, K extends PropertyKey> = T extends unknown ? Omit<T, K> : never;

const ROOM = 4;

const THREAD = 88;

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
    cardsAsOf: createdAt,
    steps: [],
    createdAt,
    updatedAt: createdAt,
    ...extra,
  };
}

function page(messages: readonly MessageDTO[], saved: MessagePage["saved"] = []): MessagePage {
  return { messages: [...messages], users: [], before: null, after: null, saved };
}

function thread(status: Thread["status"], lastActivityMinute: number, id = THREAD): Thread {
  return {
    id,
    roomId: ROOM,
    parentMessageId: 1,
    creatorId: 2,
    name: `Thread ${id}`,
    status,
    replyCount: 2,
    lastActivityAt: `2026-10-06T10:${String(lastActivityMinute).padStart(2, "0")}:00.000Z`,
    autoArchiveAfterMinutes: 4320,
    createdAt: "2026-10-06T09:30:00.000Z",
    work: null,
  };
}

let seq = 0;

function events(state: State, ...payloads: DistributiveOmit<SyncEvent, "seq">[]): State {
  return applyEvents(
    state,
    payloads.map((payload) => {
      seq += 1;

      return { ...payload, seq };
    }),
    Date.UTC(2026, 9, 6, 11),
  );
}

const opened = (messages: readonly MessageDTO[], saved: MessagePage["saved"] = []) =>
  applyPage(initialState, ROOM, page(messages, saved), "replace");

describe("mark unread", () => {
  it("moves the divider to the message, counting it and what follows", () => {
    const state = opened([message(1, 1), message(2, 2), message(3, 3), message(4, 4)]);
    const moved = moveUnreadDivider(state, ROOM, 2);

    expect(moved.timelines[ROOM]?.unreadFromId).toBe(2);
    expect(moved.timelines[ROOM]?.unreadCount).toBe(3);
  });

  it("leaves the divider alone for a message outside the window", () => {
    const state = opened([message(1, 1), message(2, 2)]);

    expect(moveUnreadDivider(state, ROOM, 99)).toBe(state);
  });
});

describe("saved marks", () => {
  it("take each page's word for its own messages, and follow saved.changed", () => {
    const first = opened([message(1, 1), message(2, 2)], [{ messageId: 1, savedItemId: 31 }]);

    expect(first.saved).toEqual({ 1: 31 });

    const repaged = applyPage(first, ROOM, page([message(1, 1)]), "replace");

    expect(repaged.saved).toEqual({});

    const saved = events(first, {
      topic: "user",
      type: "saved.changed",
      data: {
        messageId: 2,
        item: {
          id: 32,
          messageId: 2,
          status: "in_progress",
          remindAt: null,
          remindedAt: null,
          createdAt: "2026-10-06T00:00:00.000Z",
        },
      },
    });

    expect(saved.saved).toEqual({ 1: 31, 2: 32 });

    const unsaved = events(saved, {
      topic: "user",
      type: "saved.changed",
      data: { messageId: 1, item: null },
    });

    expect(unsaved.saved).toEqual({ 2: 32 });
  });
});

describe("reactions", () => {
  const reaction = { content: "🎉", title: "Party popper", imageUrl: null, reactorIds: [3] };

  it("land when at least as new as the held message, and never roll it back", () => {
    const state = opened([message(1, 1)]);

    const reacted = events(state, {
      topic: `room:${ROOM}`,
      type: "message.reactions",
      data: {
        messageId: 1,
        roomId: ROOM,
        threadId: null,
        reactions: [reaction],
        boosts: [],
        updatedAt: "2026-10-06T09:05:00.000Z",
      },
    });

    expect(reacted.messages[1]?.reactions).toEqual([reaction]);
    expect(reacted.messages[1]?.updatedAt).toBe("2026-10-06T09:05:00.000Z");

    const stale = events(reacted, {
      topic: `room:${ROOM}`,
      type: "message.reactions",
      data: {
        messageId: 1,
        roomId: ROOM,
        threadId: null,
        reactions: [],
        boosts: [],
        updatedAt: "2026-10-06T09:02:00.000Z",
      },
    });

    expect(stale.messages[1]?.reactions).toEqual([reaction]);
  });

  it("toggle the viewer in and out, dropping a reaction nobody holds", () => {
    const shown = { title: "Party popper", imageUrl: null };
    const joined = toggledReactions([reaction], "🎉", 1, shown);

    expect(joined).toEqual([{ ...reaction, reactorIds: [3, 1] }]);
    expect(toggledReactions(joined, "🎉", 1, shown)).toEqual([reaction]);
    expect(toggledReactions([{ ...reaction, reactorIds: [1] }], "🎉", 1, shown)).toEqual([]);
    expect(toggledReactions([reaction], "👍", 1, { title: "Thumbs up", imageUrl: null })).toEqual([
      reaction,
      { content: "👍", title: "Thumbs up", imageUrl: null, reactorIds: [1] },
    ]);
  });
});

describe("pins", () => {
  it("flip the message's flag and set the header's count", () => {
    const state = setRoomDetail(opened([message(1, 1)]), {
      room: {
        id: ROOM,
        kind: "open",
        name: "general",
        iconName: null,
        creatorId: 2,
        createdAt: "2026-01-01T00:00:00.000Z",
        updatedAt: "2026-01-01T00:00:00.000Z",
      },
      membership: {
        id: 40,
        roomId: ROOM,
        userId: 1,
        involvement: "everything",
        unreadAt: null,
        lastReadMessageId: null,
        roomCategoryId: null,
        favoritePosition: null,
        stageRole: null,
      },
      displayName: "general",
      memberCount: 3,
      pinsCount: 2,
      directMemberIds: [],
      memberPreviewIds: [],
      users: [],
      unread: null,
    });

    const pinned = events(state, {
      topic: `room:${ROOM}`,
      type: "message.pinned",
      data: { messageId: 1, roomId: ROOM, pinned: true, pinCount: 3 },
    });

    expect(pinned.messages[1]?.pinned).toBe(true);
    expect(pinned.rooms[ROOM]?.detail?.pinsCount).toBe(3);
  });
});

describe("threads", () => {
  const reply = (id: number, minute: number) =>
    message(id, minute, { threadId: THREAD, clientMessageId: `reply-${id}` });

  it("put live replies on the open thread's timeline, not the room's", () => {
    const state = applyThreadPage(opened([message(1, 1)]), THREAD, page([reply(5, 5)]), "replace");
    const next = receiveMessage(state, reply(6, 6));

    expect(next.threadTimelines[THREAD]?.ids).toEqual([5, 6]);
    expect(next.timelines[ROOM]?.ids).toEqual([1]);
  });

  it("reconcile a pending reply and drop a deleted one from the thread", () => {
    const pending: PendingMessage = {
      clientMessageId: "reply-6",
      roomId: ROOM,
      threadId: THREAD,
      attachmentSignedId: null,
      attachment: null,
      creatorId: 1,
      markdownSource: "6",
      createdAt: "2026-10-06T09:06:00.000Z",
      state: "sending",
      error: null,
    };

    const state = addPending(
      applyThreadPage(initialState, THREAD, page([reply(5, 5)]), "replace"),
      pending,
    );

    expect(state.pendingByThread[THREAD]).toEqual(["reply-6"]);
    expect(state.pendingByRoom[ROOM]).toBeUndefined();

    const confirmed = receiveMessage(state, reply(6, 6));

    expect(confirmed.pendingByThread[THREAD]).toEqual([]);
    expect(confirmed.threadTimelines[THREAD]?.ids).toEqual([5, 6]);
    expect(removeMessage(confirmed, 5, ROOM, THREAD, 0).threadTimelines[THREAD]?.ids).toEqual([6]);
  });

  it("keep the parent's indicator current", () => {
    const indicator = {
      threadId: THREAD,
      replyCount: 3,
      lastReplyAt: "2026-10-06T10:00:00.000Z",
      replierIds: [3, 2],
    };

    const state = events(opened([message(1, 1)]), {
      topic: `room:${ROOM}`,
      type: "thread.indicator",
      data: { roomId: ROOM, parentMessageId: 1, thread: indicator },
    });

    expect(state.messages[1]?.thread).toEqual(indicator);
  });

  it("keep a loaded list in activity order for its filter, and handle deletion", () => {
    const listed = loadThreadList(opened([message(1, 1)]), ROOM, "active", {
      threads: [{ thread: thread("active", 5, 70), membership: null }],
      users: [],
    });

    const created = events(listed, {
      topic: `room:${ROOM}`,
      type: "thread.created",
      data: thread("active", 9),
    });

    expect(created.roomThreads[ROOM]?.ids).toEqual([THREAD, 70]);

    const closed = events(created, {
      topic: `thread:${THREAD}`,
      type: "thread.updated",
      data: thread("closed", 9),
    });

    expect(closed.roomThreads[ROOM]?.ids).toEqual([70]);
    expect(closed.threads[THREAD]?.status).toBe("closed");

    const removed = events(closed, {
      topic: `room:${ROOM}`,
      type: "thread.removed",
      data: { threadId: 70, roomId: ROOM },
    });

    expect(removed.threads[70]).toBeUndefined();
    expect(removed.roomThreads[ROOM]?.ids).toEqual([]);
    expect(removed.threadPanes[70]?.status).toBe("error");
  });

  it("track the viewer's unread state from thread.unread and thread.read", () => {
    const listed = loadThreadList(initialState, ROOM, "all", {
      threads: [
        {
          thread: thread("active", 5),
          membership: {
            threadId: THREAD,
            involvement: "everything",
            unreadAt: null,
            joinedAt: "2026-10-06T09:30:00.000Z",
          },
        },
      ],
      users: [],
    });

    const unread = events(listed, {
      topic: "user",
      type: "thread.unread",
      data: { threadId: THREAD, roomId: ROOM, refreshOnly: false },
    });

    expect(unread.threadMemberships[THREAD]?.unreadAt).toBe("2026-10-06T11:00:00.000Z");

    const refresh = events(listed, {
      topic: "user",
      type: "thread.unread",
      data: { threadId: THREAD, roomId: ROOM, refreshOnly: true },
    });

    expect(refresh.threadMemberships[THREAD]?.unreadAt).toBeNull();

    const read = events(unread, {
      topic: "user",
      type: "thread.read",
      data: { threadId: THREAD, roomId: ROOM },
    });

    expect(read.threadMemberships[THREAD]?.unreadAt).toBeNull();
  });
});

describe("fresh windows", () => {
  const reply = (id: number, minute: number) =>
    message(id, minute, { threadId: THREAD, clientMessageId: `reply-${id}` });

  it("keep a reply that arrives while a thread's first page loads", () => {
    const loading = setThreadPageReplacing(initialState, THREAD);
    const arrived = receiveMessage(loading, reply(7, 7));
    // The page was read before reply 7 was posted.
    const landed = applyThreadPage(arrived, THREAD, page([reply(5, 5), reply(6, 6)]), "replace");

    expect(landed.threadTimelines[THREAD]?.ids).toEqual([5, 6, 7]);
    expect(landed.threadTimelines[THREAD]?.arrived).toBeNull();
  });

  it("drop held messages that were deleted, or when the window stops short of the present", () => {
    const loading = setThreadPageReplacing(initialState, THREAD);
    const arrived = removeMessage(receiveMessage(loading, reply(7, 7)), 7, ROOM, THREAD, 0);

    expect(
      applyThreadPage(arrived, THREAD, page([reply(5, 5)]), "replace").threadTimelines[THREAD]?.ids,
    ).toEqual([5]);

    const held = receiveMessage(loading, reply(7, 7));
    const away = { ...page([reply(5, 5)]), after: 6 };

    expect(applyThreadPage(held, THREAD, away, "replace").threadTimelines[THREAD]?.ids).toEqual([
      5,
    ]);
  });

  it("stop holding once the load fails", () => {
    const loading = setPageReplacing(
      applyPage(initialState, ROOM, page([message(1, 1)]), "replace"),
      ROOM,
    );

    const arrived = receiveMessage(loading, message(2, 2));

    expect(arrived.timelines[ROOM]?.arrived).toEqual([2]);
    expect(setPageFailed(arrived, ROOM).timelines[ROOM]?.arrived).toBeNull();
  });

  it("re-read part of a window in place: drop what the page lacks inside its span only", () => {
    const window = {
      ...page([1, 2, 3, 4, 5].map((id) => message(id, id))),
      before: 0,
      after: 6,
    };

    const state = applyPage(initialState, ROOM, window, "replace");
    const generation = state.timelines[ROOM]?.generation;
    // 3 was deleted; 1 and 5 lie outside the re-read page; 9 is newer than the window.
    const reread = { ...page([2, 4].map((id) => message(id, id))), before: 1, after: 5 };
    const next = applyPage(state, ROOM, reread, "refresh");

    expect(next.timelines[ROOM]?.ids).toEqual([1, 2, 4, 5]);
    expect(next.timelines[ROOM]?.before).toBe(0);
    expect(next.timelines[ROOM]?.after).toBe(6);
    expect(next.timelines[ROOM]?.generation).toBe(generation);
  });
});

describe("thread lists", () => {
  const list = (id: number) => ({
    threads: [{ thread: thread("active", 5, id), membership: null }],
    users: [],
  });

  it("ignore a late answer for a filter the pane has left", () => {
    const switched = setThreadListLoading(
      setThreadListLoading(initialState, ROOM, "active"),
      ROOM,
      "closed",
    );

    const late = loadThreadList(switched, ROOM, "active", list(70));

    expect(late.roomThreads[ROOM]).toEqual({ filter: "closed", ids: [], status: "loading" });
    expect(late.threads[70]?.id).toBe(70);
    expect(setThreadListFailed(late, ROOM, "active").roomThreads[ROOM]?.status).toBe("loading");

    const current = loadThreadList(late, ROOM, "closed", list(71));

    expect(current.roomThreads[ROOM]).toEqual({ filter: "closed", ids: [71], status: "ready" });
  });
});
