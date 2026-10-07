import { describe, expect, it } from "vitest";
import type { BoardListing } from "../../src/gen/BoardListing.ts";
import type { BoardPostForm } from "../../src/gen/BoardPostForm.ts";
import type { ThreadDetail } from "../../src/gen/ThreadDetail.ts";
import type { WorkList } from "../../src/gen/WorkList.ts";
import { field } from "../json.ts";
import { collect, errorOf, expectStatus, get, harness, messageBody, send } from "../s2/testing.ts";
import { BOT_ID, DEACTIVATED_ID, VIEWER_ID } from "../seed.ts";
import { BOARD_POST_IDS, BOARD_ROOM_ID } from "./seed.ts";

const root = `/api/v1/rooms/${BOARD_ROOM_ID}`;

const newPost = {
  name: "Add pagination",
  status: "planned",
  ownerId: null,
  tags: [" API ", "api", "bug"],
  message: null,
};

describe("mock boards and work", () => {
  it("lists the seeded board with classic filters, owner options and tag counts", async () => {
    const { server } = harness();
    const all = await get<BoardListing>(server, `${root}/board?status=all`);
    expect(all.posts).toHaveLength(12);
    expect(all.posts[0]?.thread.id).toBe(BOARD_POST_IDS.apiPagination);
    expect(all.ownerOptions.find(({ userId }) => userId === BOT_ID)?.agent).toBe(true);
    expect(all.tagCounts).toEqual([
      { name: "api", count: 3 },
      { name: "bug", count: 2 },
      { name: "design", count: 4 },
      { name: "infra", count: 4 },
    ]);
    expect(all.digest?.text).toContain("backup restore");
    expect(all.canAdminister).toBe(true);
    const done = await get<BoardListing>(server, `${root}/board?status=done`);
    expect(done.posts).toHaveLength(3);
    const mine = await get<BoardListing>(server, `${root}/board?owner=me&tag=INFRA`);
    expect(mine.posts.map(({ thread }) => thread.id)).toEqual([9012, 9009]);
    const agents = await get<BoardListing>(server, `${root}/board?owner=agents&status=all`);
    expect(agents.posts.map(({ thread }) => thread.id)).toEqual([9005, 9008]);
    const unavailable = await get<BoardListing>(server, `${root}/board?owner=${DEACTIVATED_ID}`);
    expect(unavailable.posts[0]?.thread.work?.ownerActive).toBe(false);

    const unknown = await get<BoardListing>(
      server,
      `${root}/board?status=no&owner=no&tag=%20API%20&page=99`,
    );

    expect([unknown.status, unknown.owner, unknown.tag, unknown.page]).toEqual([
      "open",
      "anyone",
      "api",
      20,
    ]);
    expect((await server.handle({ method: "GET", path: "/api/v1/rooms/1/board" })).status).toBe(
      404,
    );
  });
  it("returns detailed results, links, candidate users, history and discussion", async () => {
    const { server } = harness();
    const form = await get<BoardPostForm>(server, `${root}/posts/new`);
    expect(form.ownerCandidates.at(-1)?.userId).toBe(BOT_ID);
    expect(form.tagSuggestions).toEqual(["api", "bug", "design", "infra"]);
    const done = await get<ThreadDetail>(server, "/api/v1/threads/9003");
    expect(done.work?.resultHtml).toContain("<strong>");
    expect(done.work?.history[0]?.kind).toBe("result");
    expect(done.permissions.canRemoveWork).toBe(false);
    const linked = await get<ThreadDetail>(server, "/api/v1/threads/9005");
    expect(linked.thread.work?.links.map(({ kind }) => kind)).toEqual([
      "pull_request",
      "event",
      "drive_file",
    ]);
    expect(linked.thread.work?.runUrl).toMatch(/^https:/);
    expect(linked.work?.handoffReceivers).toEqual([]);

    const replies = await get<{ messages: { markdownSource: string }[] }>(
      server,
      "/api/v1/threads/9005/messages",
    );

    expect(replies.messages).toHaveLength(3);
    expect(replies.messages[0]?.markdownSource).toContain("Brief:");

    const locked = await send(
      server,
      "POST",
      "/api/v1/threads/9010/messages",
      messageBody("locked", "Reply"),
    );

    expect(locked.status).toBe(403);
  });
  it("validates and normalizes creation and deduplicates an opener retry", async () => {
    const { server, clock } = harness();
    const events = collect(server, [`room:${BOARD_ROOM_ID}`]);
    const body = { ...newPost, message: messageBody("board-opener", "Brief **Markdown**") };
    const created = await expectStatus<ThreadDetail>(server, "POST", `${root}/posts`, body, 201);
    expect(created.thread.work?.tags).toEqual(["api", "bug"]);
    expect(created.membership?.threadId).toBe(created.thread.id);
    expect(created.parentMessage).toBeNull();
    const retry = await expectStatus<ThreadDetail>(server, "POST", `${root}/posts`, body, 200);
    expect(retry.thread.id).toBe(created.thread.id);
    clock.advance(30);
    expect(events.filter(({ type }) => type === "thread.created")).toHaveLength(1);

    for (const invalid of [
      { name: " " },
      { name: "n".repeat(101) },
      { tags: ["a", "b", "c", "d", "e", "f"] },
      { tags: ["a".repeat(31)] },
      { tags: ["bad_tag"] },
      { ownerId: DEACTIVATED_ID },
      { message: messageBody("long", "x".repeat(50001)) },
    ]) {
      const failed = await send(server, "POST", `${root}/posts`, { ...newPost, ...invalid });
      expect(failed.status).toBe(422);
      expect(errorOf(failed.json).tag).toBe("Validation");
    }

    const refused = await send(server, "POST", `${root}/threads`, {});
    expect(errorOf(refused.json)).toEqual({
      tag: "Forbidden",
      message: "Board rooms take posts, not threads",
    });
  });
  it("pages by a growing window of 50 and publishes remote controls", async () => {
    const { server, clock } = harness();
    const events = collect(server, ["room:900"]);

    for (let index = 0; index < 45; index++)
      await send(server, "POST", "/__mock/board-create", { name: `Remote ${index}` });
    const first = await get<BoardListing>(server, `${root}/board?status=all`);
    const second = await get<BoardListing>(server, `${root}/board?status=all&page=2`);
    expect(first.posts).toHaveLength(50);
    expect(first.hasMore).toBe(true);
    expect(second.posts).toHaveLength(57);
    expect(second.hasMore).toBe(false);
    await send(server, "POST", "/__mock/board-update", {
      threadId: 9004,
      status: "done",
      ownerId: VIEWER_ID,
      tags: ["bug"],
    });
    clock.advance(30);
    expect(
      events.some((event) => event.type === "thread.created" && event.data.creatorId === 2),
    ).toBe(true);
    expect(
      events.some(
        (event) =>
          event.type === "thread.updated" &&
          event.data.id === 9004 &&
          event.data.work?.status === "done",
      ),
    ).toBe(true);
  });
  it("applies atomic work changes, handoffs and removal, and broadcasts replies", async () => {
    const { server, clock } = harness();
    const events = collect(server, ["room:900", "thread:9004"]);

    const updated = await expectStatus<ThreadDetail>(
      server,
      "PATCH",
      "/api/v1/threads/9004/work",
      {
        status: "in_progress",
        ownerId: VIEWER_ID,
        tags: ["infra"],
        resultMarkdown: "Ready **now**",
      },
      200,
    );

    expect(updated.work?.resultUpdatedById).toBe(VIEWER_ID);
    expect(updated.work?.history[0]?.kind).toBe("result");

    const failed = await send(server, "PATCH", "/api/v1/threads/9004/work", {
      status: "done",
      ownerId: DEACTIVATED_ID,
    });

    expect(failed.status).toBe(422);
    expect((await get<ThreadDetail>(server, "/api/v1/threads/9004")).thread.work?.status).toBe(
      "in_progress",
    );
    const untrack = await send(server, "PATCH", "/api/v1/threads/9004/work", { status: null });
    expect(field(field(untrack.json, "error"), "fields")).toHaveProperty("ownerId");

    const handed = await expectStatus<ThreadDetail>(
      server,
      "POST",
      "/api/v1/threads/9004/work/handoff",
      {
        receiverAgentId: BOT_ID,
        summary: "Finish this",
        links: ["https://example.com", " https://example.com "],
        openQuestions: ["How soon?"],
      },
      201,
    );

    expect(handed.thread.work?.owner?.id).toBe(BOT_ID);
    expect(handed.work?.history[0]?.handoff?.linkCount).toBe(1);
    expect(
      (
        await send(server, "POST", "/api/v1/threads/9004/work/handoff", {
          receiverAgentId: BOT_ID,
          summary: "Again",
          links: [],
          openQuestions: [],
        })
      ).status,
    ).toBe(422);
    server.threadPost(9004, 2, "A reply");
    await send(server, "DELETE", "/api/v1/threads/9004");
    clock.advance(30);
    expect(
      events.some((event) => event.type === "thread.updated" && event.data.replyCount === 4),
    ).toBe(true);
    expect(events.some(({ type }) => type === "thread.removed")).toBe(true);
    const work = await get<WorkList>(server, "/api/v1/work?state=boards");
    expect(work.threads).toHaveLength(11);
    expect(work.threads.every(({ board }) => board)).toBe(true);
    expect((await get<WorkList>(server, "/api/v1/work?state=agents")).threads).toHaveLength(2);
  });
});
