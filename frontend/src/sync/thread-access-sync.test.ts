import { describe, expect, it } from "vitest";
import type { SyncEvent } from "../gen/SyncEvent.ts";
import type { Thread } from "../gen/Thread.ts";
import { threadAccessChanges } from "./thread-access-sync.ts";

const secret: Thread = {
  id: 88,
  roomId: 4,
  parentMessageId: 1,
  creatorId: 2,
  name: "Just us",
  status: "active",
  visibility: "private",
  replyCount: 1,
  lastActivityAt: "2026-10-12T10:00:00.000Z",
  autoArchiveAfterMinutes: 4320,
  createdAt: "2026-10-12T09:00:00.000Z",
  work: null,
};

const removed: SyncEvent = {
  seq: 1,
  topic: "user",
  type: "thread.removed",
  data: { threadId: 88, roomId: 4 },
};

const updated = (topic: string, thread: Thread = secret): SyncEvent => ({
  seq: 2,
  topic,
  type: "thread.updated",
  data: thread,
});

const held = { threadPanes: {} };

describe("threadAccessChanges", () => {
  it("counts a removal on the viewer's own topic as lost access, and not one on a room topic", () => {
    expect(
      threadAccessChanges({ removedThreads: {}, ...held }, { removedThreads: { 88: 1 } }, [
        removed,
      ]),
    ).toEqual({
      lost: [88],
      regained: [],
    });

    expect(
      threadAccessChanges({ removedThreads: {}, ...held }, { removedThreads: { 88: 1 } }, [
        { ...removed, topic: "room:4" },
      ]).lost,
    ).toEqual([]);
  });

  it("counts a private thread's update that lifted its removal as regained", () => {
    const before = { removedThreads: { 88: 1 }, ...held };
    const after = { removedThreads: {} };

    expect(threadAccessChanges(before, after, [updated("user")]).regained).toEqual([88]);
    // Still removed afterwards, on a room topic, or public: nothing was regained.
    expect(threadAccessChanges(before, before, [updated("user")]).regained).toEqual([]);
    expect(threadAccessChanges(before, after, [updated("room:4")]).regained).toEqual([]);
    expect(
      threadAccessChanges(before, after, [updated("user", { ...secret, visibility: "public" })])
        .regained,
    ).toEqual([]);
  });

  it("counts a removal and a grant in one batch as regained, whichever came first", () => {
    const granted = updated("user");

    // Removed, then added back, in one replay: the batch ends readable, so the pane reloads.
    expect(
      threadAccessChanges({ removedThreads: {}, ...held }, { removedThreads: {} }, [
        removed,
        granted,
      ]),
    ).toEqual({ lost: [88], regained: [88] });

    // Added, then removed: it ends unreadable, so nothing is regained.
    expect(
      threadAccessChanges({ removedThreads: {}, ...held }, { removedThreads: { 88: 1 } }, [
        granted,
        removed,
      ]),
    ).toEqual({ lost: [88], regained: [] });
  });

  it("recovers a failed pane without a tombstone, but not an ordinary private update", () => {
    const after = { removedThreads: {} };
    const failed = {
      status: "error" as const,
      error: "gone",
      permissions: null,
      work: null,
      workFacts: null,
    };

    expect(
      threadAccessChanges({ removedThreads: {}, threadPanes: {} }, after, [updated("user")])
        .regained,
    ).toEqual([]);
    expect(
      threadAccessChanges({ removedThreads: {}, threadPanes: { 88: failed } }, after, [
        updated("user"),
      ]).regained,
    ).toEqual([88]);
    // Held and readable: an ordinary update regains nothing.
    expect(
      threadAccessChanges({ removedThreads: {}, ...held }, after, [updated("user")]).regained,
    ).toEqual([]);
  });
});
