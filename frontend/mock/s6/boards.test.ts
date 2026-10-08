import { describe, expect, it } from "vitest";
import type { BoardListing } from "../../src/gen/BoardListing.ts";
import type { BoardPostForm } from "../../src/gen/BoardPostForm.ts";
import type { ThreadDetail } from "../../src/gen/ThreadDetail.ts";
import type { WorkList } from "../../src/gen/WorkList.ts";
import { field } from "../json.ts";
import { THREAD_IDS } from "../s2/seed.ts";
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
    const briefless = { ...newPost, message: null, clientPostId: "retry-without-brief" };
    const first = await expectStatus<ThreadDetail>(server, "POST", `${root}/posts`, briefless, 201);
    const again = await expectStatus<ThreadDetail>(server, "POST", `${root}/posts`, briefless, 200);
    expect(again.thread.id).toBe(first.thread.id);
    clock.advance(30);
    expect(events.filter(({ type }) => type === "thread.created")).toHaveLength(2);

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

describe("mock work list and handoffs", () => {
  it("filters the work list as work_threads#index does", async () => {
    const { server } = harness();

    const ids = async (state: string) =>
      (await get<WorkList>(server, `/api/v1/work${state}`)).threads.map(({ thread }) => thread.id);

    const open = await ids("");

    expect(open).toEqual(await ids("?state=open"));
    expect(open).toEqual(await ids("?state=unknown"));
    expect(open).not.toContain(BOARD_POST_IDS.pricingPage);
    expect(open[0]).toBe(BOARD_POST_IDS.apiPagination);
    expect(await ids("?state=done")).toEqual([
      BOARD_POST_IDS.rateLimits,
      BOARD_POST_IDS.iconRefresh,
      BOARD_POST_IDS.pricingPage,
    ]);
    expect(await ids("?state=all")).toHaveLength(12);
    expect(await ids("?state=agents")).toEqual([
      BOARD_POST_IDS.retryBug,
      BOARD_POST_IDS.rateLimits,
    ]);

    const row = (await get<WorkList>(server, "/api/v1/work")).threads[0];

    expect(row?.roomName).toBe("Roadmap");
    expect(row?.board).toBe(true);
    expect(row?.thread.work).not.toBeNull();
  });

  it("refuses handoffs in the server's words, fields and all", async () => {
    const { server } = harness();
    const path = `/api/v1/threads/${BOARD_POST_IDS.apiPagination}/work/handoff`;
    const valid = { receiverAgentId: BOT_ID, summary: "Over to you", links: [], openQuestions: [] };

    const untracked = await send(
      server,
      "POST",
      `/api/v1/threads/${THREAD_IDS.generalActive}/work/handoff`,
      valid,
    );

    expect(untracked.status).toBe(422);
    expect(errorOf(untracked.json)).toEqual({
      tag: "Validation",
      message: "This thread isn't tracked as work",
    });
    expect(field(field(untracked.json, "error"), "fields")).toEqual({
      base: ["This thread isn't tracked as work"],
    });

    const stranger = await send(server, "POST", path, { ...valid, receiverAgentId: 424242 });

    const notMember =
      "Receiver must be an active agent member of this room with permission to post";

    expect(errorOf(stranger.json)).toEqual({ tag: "Validation", message: notMember });
    expect(field(field(stranger.json, "error"), "fields")).toEqual({
      receiverAgentId: [notMember],
    });

    const owned = await send(
      server,
      "POST",
      `/api/v1/threads/${BOARD_POST_IDS.retryBug}/work/handoff`,
      valid,
    );

    expect(errorOf(owned.json).message).toBe("Receiver is already the owner of this work");

    const package_ = await send(server, "POST", path, {
      receiverAgentId: BOT_ID,
      summary: "   ",
      links: ["ftp://example.com/spec"],
      openQuestions: Array.from({ length: 11 }, (_, index) => `Question ${index}?`),
    });

    expect(package_.status).toBe(422);
    expect(errorOf(package_.json)).toEqual({
      tag: "Validation",
      message:
        "Summary can't be blank, Links must be http(s) URLs, Open questions are limited to 10 per handoff",
    });
    expect(field(field(package_.json, "error"), "fields")).toEqual({
      summary: ["can't be blank"],
      links: ["must be http(s) URLs"],
      openQuestions: ["are limited to 10 per handoff"],
    });

    const long = await send(server, "POST", path, { ...valid, summary: "x".repeat(2001) });

    expect(field(field(long.json, "error"), "fields")).toEqual({
      summary: ["is too long (maximum is 2000 characters)"],
    });
    // Nothing changed hands.
    expect(
      (await get<ThreadDetail>(server, `/api/v1/threads/${BOARD_POST_IDS.apiPagination}`)).thread
        .work?.owner,
    ).toBeNull();
  });

  it("hands work to the agent and records the package", async () => {
    const { server } = harness();

    const handed = await expectStatus<ThreadDetail>(
      server,
      "POST",
      `/api/v1/threads/${BOARD_POST_IDS.apiPagination}/work/handoff`,
      {
        receiverAgentId: BOT_ID,
        summary: "  Cursor shape agreed; wire the endpoint.  ",
        links: ["https://example.com/spec", "https://example.com/spec", " "],
        openQuestions: ["Page size?", ""],
      },
      201,
    );

    expect(handed.thread.work?.owner?.id).toBe(BOT_ID);
    expect(handed.work?.handoffReceivers).toEqual([]);
    expect(handed.work?.history[0]).toMatchObject({
      kind: "handoff",
      actorId: VIEWER_ID,
      toOwner: { userId: BOT_ID },
      handoff: {
        summary: "Cursor shape agreed; wire the endpoint.",
        linkCount: 1,
        questionCount: 1,
      },
    });

    const agents = await get<WorkList>(server, "/api/v1/work?state=agents");

    expect(agents.threads[0]?.thread.id).toBe(BOARD_POST_IDS.apiPagination);
  });
});
