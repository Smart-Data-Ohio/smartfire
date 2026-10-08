import { describe, expect, it } from "vitest";
import type { ThreadCreated } from "../../src/gen/ThreadCreated.ts";
import type { ThreadDetail } from "../../src/gen/ThreadDetail.ts";
import type { WorkList } from "../../src/gen/WorkList.ts";
import type { Json } from "../json.ts";
import { collect, errorOf, expectStatus, get, harness, messageBody, send } from "../s2/testing.ts";
import { SEED_IDS } from "../server.ts";
import { AGENT_IDS } from "./agents.ts";
import { normalizeList, truncate, workPermissions } from "./work-model.ts";

const { users } = SEED_IDS;

const { work: ids, boardPosts, board } = SEED_IDS.s4;

const detailOf = (server: ReturnType<typeof harness>["server"], threadId: number) =>
  get<ThreadDetail>(server, `/api/v1/threads/${threadId}`);

const patch = (threadId: number) => `/api/v1/threads/${threadId}/work`;

describe("the work list", () => {
  it("lists open work newest first, board posts marked, with creators and owners", async () => {
    const { server } = harness();
    const list = await get<WorkList>(server, "/api/v1/work");
    const rowIds = list.threads.map((row) => row.thread.id);

    expect(rowIds).toEqual(
      expect.arrayContaining([ids.agentOwned, ids.viewerOwned, ids.unassigned]),
    );
    expect(rowIds).not.toContain(ids.done);
    expect(rowIds).not.toContain(ids.untracked);
    expect(list.threads.every((row) => row.thread.work !== null)).toBe(true);
    expect(list.threads.every((row) => row.thread.work?.status !== "done")).toBe(true);

    const times = list.threads.map((row) => Date.parse(row.updatedAt));

    expect(times).toEqual([...times].sort((a, b) => b - a));

    const post = list.threads.find((row) => row.thread.id === boardPosts.publicApi);

    expect(post?.board).toBe(true);
    expect(post?.roomName).toBe(board.name);
    expect(post?.thread.parentMessageId).toBeNull();
    expect(list.threads.find((row) => row.thread.id === ids.agentOwned)?.roomName).toBe("general");

    const userIds = new Set(list.users.map((user) => user.id));

    for (const row of list.threads) {
      expect(userIds).toContain(row.thread.creatorId);
    }
  });

  it("filters done, agents, boards and all; an unknown state is open", async () => {
    const { server } = harness();
    const done = await get<WorkList>(server, "/api/v1/work?state=done");
    const agents = await get<WorkList>(server, "/api/v1/work?state=agents");
    const boards = await get<WorkList>(server, "/api/v1/work?state=boards");
    const all = await get<WorkList>(server, "/api/v1/work?state=all");
    const open = await get<WorkList>(server, "/api/v1/work?state=nonsense");

    expect(done.threads.map((row) => row.thread.id)).toEqual(
      expect.arrayContaining([ids.done, boardPosts.darkMode]),
    );
    expect(done.threads.every((row) => row.thread.work?.status === "done")).toBe(true);
    expect(agents.threads.map((row) => row.thread.id)).toEqual([ids.agentOwned, 9005, 9008]);
    expect(boards.threads.every((row) => row.board)).toBe(true);
    expect(boards.threads).toHaveLength(15);
    expect(all.threads).toHaveLength(done.threads.length + open.threads.length);
  });
});

describe("work facts and detail", () => {
  it("puts facts on the thread and the full section on the detail", async () => {
    const { server } = harness();
    const detail = await detailOf(server, ids.agentOwned);
    const work = detail.thread.work;

    expect(work?.status).toBe("in_progress");
    expect(work?.owner?.id).toBe(users.ember);
    expect(work?.owner?.agent?.agentId).toBeDefined();
    expect(work?.ownerActive).toBe(true);
    expect(work?.runUrl).toMatch(/^https:/);
    expect(work?.links.map((link) => link.kind)).toEqual(["pull_request", "drive_file"]);
    expect(detail.work?.steps.length).toBeGreaterThan(0);
    expect(detail.work?.history.map((entry) => entry.kind)).toContain("handoff");

    const history = detail.work?.history ?? [];

    expect(history.map((entry) => entry.createdAt)).toEqual(
      [...history.map((entry) => entry.createdAt)].sort().reverse(),
    );
    // The owner is Ember, the room's only agent: nobody left to hand off to.
    expect(detail.work?.handoffReceivers).toEqual([]);
    expect(detail.permissions.canManageWork).toBe(true);

    const candidates = detail.work?.ownerCandidates ?? [];
    const agent = candidates.find((candidate) => candidate.userId === users.ember);

    expect(agent?.provider).not.toBeNull();
    expect(candidates.at(-1)?.userId).toBe(users.ember);
    expect(candidates.some((candidate) => candidate.userId === users.dana)).toBe(false);
    expect(new Set(detail.users.map((user) => user.id))).toContain(users.jonah);
  });

  it("marks a deactivated owner inactive, and leaves untracked threads alone", async () => {
    const { server } = harness();
    const blocked = await detailOf(server, ids.inactiveOwner);
    const plain = await detailOf(server, ids.untracked);

    expect(blocked.thread.work?.ownerActive).toBe(false);
    expect(blocked.thread.work?.links.some((link) => link.eventCancelled)).toBe(true);
    expect(plain.thread.work).toBeNull();
    expect(plain.work).toBeNull();
    expect(plain.permissions.canConvertWork).toBe(true);
    expect(plain.permissions.canManageWork).toBe(false);
  });

  it("derives the five flags from tracking, managing and owning", () => {
    const base = {
      id: 1,
      roomId: 1,
      parentMessageId: null,
      creatorId: 2,
      name: "x",
      closed: false,
      locked: false,
      lastActivityAt: "",
      autoArchiveAfterMinutes: 1,
      createdAt: "",
      messages: [],
      memberIds: new Set<number>(),
      viewerMembership: null,
    };

    const tracked = (ownerId: number | null) => ({
      ...base,
      work: {
        status: "planned" as const,
        ownerId,
        owner: null,
        ownerActive: true,
        runUrl: null,
        links: [],
        tags: [],
        resultMarkdown: null,
        resultHtml: null,
        resultUpdatedById: null,
        resultUpdatedAt: null,
        steps: [],
        history: [],
        updatedAt: "",
      },
    });

    expect(workPermissions(base, true, 1)).toEqual({
      canConvertWork: true,
      canManageWork: false,
      canUpdateWorkStatus: false,
      canAssignWork: false,
      canRemoveWork: false,
    });
    expect(workPermissions(base, false, 1).canConvertWork).toBe(false);
    expect(workPermissions(tracked(1), false, 1)).toEqual({
      canConvertWork: false,
      canManageWork: true,
      canUpdateWorkStatus: true,
      canAssignWork: false,
      canRemoveWork: false,
    });
    expect(workPermissions(tracked(3), false, 1).canManageWork).toBe(false);
    expect(workPermissions(tracked(null), true, 1).canAssignWork).toBe(true);
  });
});

describe("PATCH /threads/:id/work", () => {
  it("tracks a thread, publishes thread.updated, and records the history", async () => {
    const { server } = harness();
    const events = collect(server, [`thread:${ids.untracked}`]);

    const detail = await expectStatus<ThreadDetail>(
      server,
      "PATCH",
      patch(ids.untracked),
      { status: "planned" },
      200,
    );

    expect(detail.thread.work?.status).toBe("planned");
    expect(detail.permissions.canConvertWork).toBe(false);
    expect(detail.work?.history[0]).toMatchObject({
      kind: "update",
      fromStatus: null,
      toStatus: "planned",
      actorId: SEED_IDS.viewer,
    });
    expect(detail.work?.handoffReceivers.map((receiver) => receiver.userId)).toEqual([users.ember]);
    expect(events.some((event) => event.type === "thread.updated")).toBe(true);
  });

  it("moves, assigns and records the result; a blank result clears it", async () => {
    const { server } = harness();
    const path = patch(ids.unassigned);

    const moved = await expectStatus<ThreadDetail>(
      server,
      "PATCH",
      path,
      { status: "in_progress", ownerId: users.maya },
      200,
    );

    expect(moved.thread.work).toMatchObject({ status: "in_progress", owner: { id: users.maya } });
    expect(moved.work?.history[0]?.toOwner?.name).toBe("Maya Okafor");

    const assigned = await expectStatus<ThreadDetail>(
      server,
      "PATCH",
      path,
      { ownerId: users.jonah },
      200,
    );

    expect(assigned.work?.history[0]?.kind).toBe("assignment");

    const result = await expectStatus<ThreadDetail>(
      server,
      "PATCH",
      path,
      { resultMarkdown: "Shipped **v2**" },
      200,
    );

    expect(result.work?.resultHtml).toContain("<strong>v2</strong>");
    expect(result.work?.resultUpdatedById).toBe(SEED_IDS.viewer);
    expect(result.thread.work?.resultUpdatedAt).not.toBeNull();
    expect(result.work?.history[0]?.kind).toBe("result");

    const cleared = await expectStatus<ThreadDetail>(
      server,
      "PATCH",
      path,
      { resultMarkdown: "  " },
      200,
    );

    expect(cleared.work?.resultMarkdown).toBeNull();
    expect(cleared.thread.work?.resultUpdatedAt).not.toBeNull();
  });

  it("stops tracking only with the owner cleared too", async () => {
    const { server } = harness();
    const path = patch(ids.viewerOwned);
    const refused = await expectStatus<Json>(server, "PATCH", path, { status: null }, 422);

    expect(refused).toEqual({
      error: {
        _tag: expect.stringMatching(/^Validation$/),
        message: "Work owner requires work tracking",
        fields: { ownerId: ["requires work tracking"] },
      },
    });

    const stopped = await expectStatus<ThreadDetail>(
      server,
      "PATCH",
      path,
      { status: null, ownerId: null },
      200,
    );

    expect(stopped.thread.work).toBeNull();
    expect(stopped.work).toBeNull();
    expect(stopped.permissions.canConvertWork).toBe(true);
  });

  it("refuses owners who can't act, overlong results, bad statuses and missing permissions", async () => {
    const { server } = harness();
    const path = patch(ids.viewerOwned);

    const outsider = await expectStatus<Json>(server, "PATCH", path, { ownerId: users.ember }, 422);
    const inactive = await expectStatus<Json>(server, "PATCH", path, { ownerId: users.dana }, 422);

    const long = await expectStatus<Json>(
      server,
      "PATCH",
      path,
      { resultMarkdown: "x".repeat(20_001) },
      422,
    );

    const status = await expectStatus<Json>(server, "PATCH", path, { status: "someday" }, 422);
    const untracked = patch(ids.untracked);

    const stop = await expectStatus<ThreadDetail>(
      server,
      "PATCH",
      untracked,
      { status: null },
      200,
    );

    const result = await expectStatus<ThreadDetail>(
      server,
      "PATCH",
      untracked,
      { resultMarkdown: "x" },
      200,
    );

    const missing = await send(server, "PATCH", "/api/v1/threads/9999/work", { status: "done" });

    expect(errorOf(outsider).message).toContain("active agent member of the parent room");
    expect(errorOf(inactive).message).toContain("active human member of the parent room");
    expect(errorOf(long).message).toContain("maximum is 20000 characters");
    expect(errorOf(status).tag).toBe("Validation");
    expect(stop.thread.work).toBeNull();
    expect(result.thread.work).toBeNull();
    expect(missing.status).toBe(404);
    expect((await detailOf(server, ids.untracked)).thread.work).toBeNull();

    await send(server, "POST", "/__mock/viewer-role", { role: "member" });

    const forbidden = await expectStatus<Json>(
      server,
      "PATCH",
      patch(ids.agentOwned),
      { status: "done" },
      403,
    );

    expect(errorOf(forbidden).message).toBe("You can't make that change to this thread");
    expect(missing.json).toEqual({
      error: { _tag: expect.stringMatching(/^NotFound$/), message: "Not found" },
    });
  });

  it("returns decode errors without field errors for malformed work bodies", async () => {
    const { server } = harness();

    for (const body of [
      { status: 42 },
      { status: "someday" },
      { ownerId: "1" },
      { ownerId: true },
      { resultMarkdown: 42 },
      null,
    ]) {
      const response = await send(server, "PATCH", patch(ids.viewerOwned), body);

      expect(response.status).toBe(422);
      expect(response.json).toEqual({
        error: {
          _tag: expect.stringMatching(/^Validation$/),
          message: expect.stringMatching(/^The request body isn't valid/),
          fields: {},
        },
      });
    }
  });
});

describe("POST /threads/:id/work/handoff", () => {
  const body = (extra: Readonly<Record<string, Json>> = {}): Json => ({
    receiverAgentId: users.ember,
    summary: "Draft is done, needs a pass.",
    links: [" https://example.com/spec ", "", "https://example.com/spec"],
    openQuestions: ["Which tone?"],
    ...extra,
  });

  it("hands tracked work to an agent and records the package's size", async () => {
    const { server } = harness();
    const events = collect(server, [`thread:${ids.done}`]);

    const detail = await expectStatus<ThreadDetail>(
      server,
      "POST",
      `/api/v1/threads/${ids.done}/work/handoff`,
      body({ summary: "y".repeat(250) }),
      201,
    );

    expect(detail.thread.work?.owner?.id).toBe(users.ember);
    expect(detail.work?.handoffReceivers).toEqual([]);
    expect(detail.work?.history[0]?.kind).toBe("handoff");
    expect(detail.work?.history[0]?.handoff).toEqual({
      summary: `${"y".repeat(197)}...`,
      linkCount: 1,
      questionCount: 1,
    });
    expect(events.some((event) => event.type === "thread.updated")).toBe(true);
  });

  it("validates the receiver and the package the way the server does", async () => {
    const { server } = harness();
    const path = (threadId: number) => `/api/v1/threads/${threadId}/work/handoff`;

    const untracked = await expectStatus<Json>(server, "POST", path(ids.untracked), body(), 422);
    const owner = await expectStatus<Json>(server, "POST", path(ids.agentOwned), body(), 422);
    const absent = await expectStatus<Json>(server, "POST", path(ids.viewerOwned), body(), 422);

    const blank = await expectStatus<Json>(
      server,
      "POST",
      path(ids.done),
      body({ summary: " ", links: ["ftp://example.com"], openQuestions: ["q".repeat(501)] }),
      422,
    );

    const many = await expectStatus<Json>(
      server,
      "POST",
      path(ids.done),
      body({ links: Array.from({ length: 11 }, (_, index) => `https://example.com/${index}`) }),
      422,
    );

    expect(errorOf(untracked).tag).toBe("Validation");
    expect(errorOf(owner).message).toContain("Receiver is already the owner of this work");
    expect(errorOf(absent).message).toContain("Receiver must be an active agent member");
    expect(errorOf(blank).message).toBe(
      "Summary can't be blank, Links must be http(s) URLs, Open questions must be at most 500 characters each",
    );
    expect(blank).toMatchObject({
      error: {
        fields: {
          summary: ["can't be blank"],
          links: ["must be http(s) URLs"],
          openQuestions: ["must be at most 500 characters each"],
        },
      },
    });
    expect(errorOf(many).message).toContain("Links are limited to 10 per handoff");
  });

  it("checks handoff scope before decoding the body", async () => {
    const { server } = harness();

    await send(server, "POST", "/__mock/viewer-role", { role: "member" });

    const missing = await send(server, "POST", "/api/v1/threads/9999/work/handoff", {});

    const untracked = await send(
      server,
      "POST",
      `/api/v1/threads/${ids.untracked}/work/handoff`,
      {},
    );

    const forbidden = await send(
      server,
      "POST",
      `/api/v1/threads/${ids.agentOwned}/work/handoff`,
      {},
    );

    expect(missing.status).toBe(404);
    expect(missing.json).toEqual({
      error: { _tag: expect.stringMatching(/^NotFound$/), message: "Not found" },
    });
    expect(untracked.status).toBe(422);
    expect(untracked.json).toEqual({
      error: {
        _tag: expect.stringMatching(/^Validation$/),
        message: "This thread isn't tracked as work",
        fields: { base: ["This thread isn't tracked as work"] },
      },
    });
    expect(forbidden.status).toBe(403);
    expect(forbidden.json).toEqual({
      error: {
        _tag: expect.stringMatching(/^Forbidden$/),
        message: "You cannot manage work in this thread",
      },
    });

    const malformed = await send(
      server,
      "POST",
      `/api/v1/threads/${ids.viewerOwned}/work/handoff`,
      { summary: "Ready", links: [], openQuestions: [] },
    );

    expect(malformed.status).toBe(422);
    expect(malformed.json).toEqual({
      error: {
        _tag: expect.stringMatching(/^Validation$/),
        message: expect.stringMatching(/^The request body isn't valid/),
        fields: {},
      },
    });
  });

  it("checks the seeded receiver's capability before already the owner or the package", async () => {
    const { server } = harness();
    const parent = server.post(SEED_IDS.rooms.announcements, SEED_IDS.viewer, "Track release work");

    const created = await expectStatus<ThreadCreated>(
      server,
      "POST",
      `/api/v1/rooms/${SEED_IDS.rooms.announcements}/threads`,
      {
        parentMessageId: parent.id,
        name: null,
        message: messageBody("ember-announcements", "Release notes"),
      },
      201,
    );

    const threadId = created.detail.thread.id;

    await expectStatus<ThreadDetail>(
      server,
      "PATCH",
      patch(threadId),
      { status: "planned", ownerId: AGENT_IDS.ember },
      200,
    );

    const detail = await detailOf(server, threadId);

    const response = await send(
      server,
      "POST",
      `/api/v1/threads/${threadId}/work/handoff`,
      body({ receiverAgentId: AGENT_IDS.ember, summary: "" }),
    );

    expect(detail.work?.handoffReceivers).toEqual([]);
    expect(response.status).toBe(422);
    expect(response.json).toEqual({
      error: {
        _tag: expect.stringMatching(/^Validation$/),
        message: "Receiver must hold the manage_threads capability in this room",
        fields: {
          receiverAgentId: ["Receiver must hold the manage_threads capability in this room"],
        },
      },
    });
  });

  it("normalizes lists and truncates like the server", () => {
    expect(normalizeList([" a ", "", "a", "b"])).toEqual(["a", "b"]);
    expect(truncate("abcdef", 5)).toBe("ab...");
    expect(truncate("abc", 5)).toBe("abc");
  });
});

describe("the work-status control", () => {
  it("moves work as someone else and publishes thread.updated", async () => {
    const { server } = harness();
    const events = collect(server, [`room:${SEED_IDS.rooms.general}`]);

    const response = await send(server, "POST", "/__mock/work-status", {
      threadId: ids.agentOwned,
      status: "blocked",
    });

    expect(response.status).toBe(200);
    expect((await detailOf(server, ids.agentOwned)).work?.history[0]?.actorId).toBe(users.maya);
    expect(events.filter((event) => event.type === "thread.updated")).toHaveLength(1);
  });
});
