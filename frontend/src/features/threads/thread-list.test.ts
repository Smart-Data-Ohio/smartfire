import { describe, expect, it } from "vitest";
import type { RoomThreadList, Thread, ThreadMembership } from "../../store/model.ts";
import { isThreadTab, serverFilter, visibleThreads } from "./thread-list.ts";

function thread(id: number, status: Thread["status"] = "active"): Thread {
  return {
    id,
    roomId: 1,
    parentMessageId: id * 10,
    creatorId: 2,
    name: `Thread ${id}`,
    status,
    replyCount: 1,
    lastActivityAt: "2026-10-06T09:00:00.000Z",
    autoArchiveAfterMinutes: 1440,
    createdAt: "2026-10-06T08:00:00.000Z",
  };
}

function membership(
  threadId: number,
  involvement: ThreadMembership["involvement"],
): ThreadMembership {
  return { threadId, involvement, unreadAt: null, joinedAt: "2026-10-06T08:00:00.000Z" };
}

const threads = { 1: thread(1), 2: thread(2), 3: thread(3, "closed") };

const memberships = { 1: membership(1, "everything"), 2: membership(2, "mentions"), 3: null };

describe("serverFilter", () => {
  it("asks for every thread on the Following tab", () => {
    expect(serverFilter("following")).toBe("all");
    expect(serverFilter("active")).toBe("active");
    expect(serverFilter("closed")).toBe("closed");
  });
});

describe("isThreadTab", () => {
  it("knows the four tabs", () => {
    expect(isThreadTab("following")).toBe(true);
    expect(isThreadTab("locked")).toBe(false);
  });
});

describe("visibleThreads", () => {
  it("reads as loading until the tab's own filter has a list", () => {
    const list: RoomThreadList = { filter: "active", ids: [1, 2], status: "ready" };

    expect(visibleThreads("closed", { list, threads, memberships })).toEqual({
      status: "loading",
      threads: [],
    });

    expect(visibleThreads("active", { list: undefined, threads, memberships }).status).toBe(
      "loading",
    );
  });

  it("lists the threads in the list's order, skipping unknown ids", () => {
    const list: RoomThreadList = { filter: "all", ids: [3, 9, 1], status: "ready" };
    const view = visibleThreads("all", { list, threads, memberships });

    expect(view.status).toBe("ready");
    expect(view.threads.map((row) => row.id)).toEqual([3, 1]);
  });

  it("keeps only followed threads on the Following tab", () => {
    const list: RoomThreadList = { filter: "all", ids: [1, 2, 3], status: "ready" };

    expect(
      visibleThreads("following", { list, threads, memberships }).threads.map((row) => row.id),
    ).toEqual([1]);
  });

  it("keeps the rows it had through a reload, and reports errors with them", () => {
    const reloading: RoomThreadList = { filter: "active", ids: [1], status: "loading" };
    const failed: RoomThreadList = { filter: "active", ids: [], status: "error" };

    expect(visibleThreads("active", { list: reloading, threads, memberships }).status).toBe(
      "ready",
    );
    expect(visibleThreads("active", { list: failed, threads, memberships })).toEqual({
      status: "error",
      threads: [],
    });
  });
});
