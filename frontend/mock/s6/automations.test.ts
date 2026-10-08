import { describe, expect, it } from "vitest";
import type { BoardAutomations } from "../../src/gen/BoardAutomations.ts";
import type { ThreadDetail } from "../../src/gen/ThreadDetail.ts";
import { HttpError, type MockResponse, notFound } from "../http.ts";
import { field, type Json } from "../json.ts";
import { dispatch } from "../s2/context.ts";
import { errorOf, expectStatus, get, harness, NOW, send } from "../s2/testing.ts";
import { buildWorld } from "../s3/seed.ts";
import { BOT_ID, DEACTIVATED_ID, USER_IDS, VIEWER_ID } from "../seed.ts";
import { autoAssignedBoardOwner, createBoardAutomations } from "./automations.ts";
import { BOARD_ROOM_ID, BOARD_TAG_RULE_IDS } from "./seed.ts";

const root = `/api/v1/rooms/${BOARD_ROOM_ID}/automations`;

const off = { nudgeAfterMinutes: null, escalateAfterMinutes: null };

const timer = { nudgeAfterMinutes: 60, escalateAfterMinutes: 120 };

const sla = { planned: timer, inProgress: off, blocked: off };

function fieldsOf(response: MockResponse): Json | undefined {
  return field(field(response.json, "error"), "fields");
}

/** Inject mutable membership and user state to exercise permissions without changing sessions. */
function permissionHarness() {
  const world = buildWorld(NOW, 1);
  const board = world.rooms.get(BOARD_ROOM_ID);
  const viewer = world.users.get(VIEWER_ID);

  if (board === undefined || viewer === undefined) throw new Error("Seed missing");

  const { routes } = createBoardAutomations({
    world: () => world,
    roomOr404: (roomId) => {
      const room = world.rooms.get(roomId);

      if (room === undefined || room.membership.involvement === "invisible") throw notFound();

      return room;
    },
    usersFor: (ids) =>
      [...new Set(ids)]
        .sort((a, b) => a - b)
        .flatMap((id) => {
          const user = world.users.get(id);

          return user === undefined ? [] : [user];
        }),
  });

  const request = async (
    method = "GET",
    path = `/rooms/${BOARD_ROOM_ID}/automations`,
    body?: Json,
  ): Promise<MockResponse> => {
    const handler = dispatch(routes, method, path, new URLSearchParams(), body);

    if (handler === null) throw new Error("Route missing");

    try {
      return await handler();
    } catch (error) {
      if (error instanceof HttpError) return { status: error.status, json: { error: error.error } };

      throw error;
    }
  };

  return { world, board, viewer, request };
}

describe("mock board automations", () => {
  it("stops an unavailable first matching rule from assigning and ignores non-board rooms", () => {
    const { world, board } = permissionHarness();
    expect(autoAssignedBoardOwner(world, BOARD_ROOM_ID, ["design", "bug"])?.id).toBe(BOT_ID);
    expect(autoAssignedBoardOwner(world, 1, ["bug"])).toBeNull();
    const bot = world.users.get(BOT_ID);

    if (bot?.agent == null) throw new Error("Agent missing");
    bot.agent.suspended = true;
    expect(autoAssignedBoardOwner(world, BOARD_ROOM_ID, ["design", "bug"])).toBeNull();
    expect(autoAssignedBoardOwner(world, BOARD_ROOM_ID, ["design"])?.id).toBe(USER_IDS.maya);
    board.memberIds = board.memberIds.filter((id) => id !== USER_IDS.maya);
    expect(autoAssignedBoardOwner(world, BOARD_ROOM_ID, ["design"])).toBeNull();
    bot.agent.suspended = false;
    bot.status = "deactivated";
    expect(autoAssignedBoardOwner(world, BOARD_ROOM_ID, ["bug"])).toBeNull();
  });

  it("seeds tag rules in creation order, one planned timer and candidates by lower-cased name", async () => {
    const { server } = harness();
    const settings = await get<BoardAutomations>(server, root);
    expect(settings.roomId).toBe(BOARD_ROOM_ID);
    expect(settings.tagRules).toEqual([
      { id: BOARD_TAG_RULE_IDS.bug, tag: "bug", assigneeId: BOT_ID },
      { id: BOARD_TAG_RULE_IDS.design, tag: "design", assigneeId: USER_IDS.maya },
    ]);
    expect(settings.slaTimers).toEqual([
      { status: "planned", nudgeAfterMinutes: 1440, escalateAfterMinutes: 2880 },
    ]);
    const byId = new Map(settings.users.map((user) => [user.id, user]));
    const names = settings.candidates.map((id) => byId.get(id)?.name.toLowerCase());
    expect(names).toEqual([...names].sort());
    expect(settings.candidates).toContain(BOT_ID);
    expect(new Set(settings.users.map((user) => user.id)).size).toBe(settings.users.length);
  });

  it("normalizes tags, appends rules, deletes only that board's rules and does not reuse ids", async () => {
    const { server } = harness();

    const added = await expectStatus<BoardAutomations>(
      server,
      "POST",
      `${root}/tag_rules`,
      { tag: "  API-V2  ", assigneeId: VIEWER_ID },
      201,
    );

    expect(added.tagRules.map((rule) => rule.tag)).toEqual(["bug", "design", "api-v2"]);
    expect(added.tagRules.at(-1)?.id).toBe(9103);

    const deleted = await expectStatus<BoardAutomations>(
      server,
      "DELETE",
      `${root}/tag_rules/9103`,
      null,
      200,
    );

    expect(deleted.tagRules.map((rule) => rule.id)).toEqual([9101, 9102]);

    const again = await expectStatus<BoardAutomations>(
      server,
      "POST",
      `${root}/tag_rules`,
      { tag: "api-v2", assigneeId: VIEWER_ID },
      201,
    );

    expect(again.tagRules.at(-1)?.id).toBe(9104);
    const missing = await send(server, "DELETE", `${root}/tag_rules/999`);
    expect(missing.status).toBe(404);
    expect(errorOf(missing.json)).toEqual({ tag: "NotFound", message: "Rule not found." });
  });

  it.each([
    { tag: " ", messages: ["can't be blank", "is invalid"] },
    { tag: "a".repeat(31), messages: ["is too long (maximum is 30 characters)"] },
    { tag: "bad_tag", messages: ["is invalid"] },
    { tag: "-bug", messages: ["is invalid"] },
    { tag: " BUG ", messages: ["has already been taken"] },
  ])("validates tag $tag before adding a rule", async ({ tag, messages }) => {
    const { server } = harness();
    const failed = await send(server, "POST", `${root}/tag_rules`, { tag, assigneeId: VIEWER_ID });
    expect(failed.status).toBe(422);
    expect(fieldsOf(failed)).toEqual({ tag: messages });
    expect((await get<BoardAutomations>(server, root)).tagRules).toHaveLength(2);
  });

  it.each([
    { assigneeId: null, message: "must exist" },
    { assigneeId: 999, message: "must exist" },
    { assigneeId: DEACTIVATED_ID, message: "must be an active board member able to own posts" },
    { assigneeId: USER_IDS.jonah, message: "must be an active board member able to own posts" },
  ])("validates assignee $assigneeId", async ({ assigneeId, message }) => {
    const { server } = harness();
    const failed = await send(server, "POST", `${root}/tag_rules`, { tag: "infra", assigneeId });
    expect(failed.status).toBe(422);
    expect(fieldsOf(failed)).toEqual({ assigneeId: [message] });
  });

  it("collects both tag and assignee fields and refuses suspended or missing agents", async () => {
    const { request, world } = permissionHarness();
    const path = `/rooms/${BOARD_ROOM_ID}/automations/tag_rules`;
    const both = await request("POST", path, { tag: "bug" });
    expect(fieldsOf(both)).toEqual({ tag: ["has already been taken"], assigneeId: ["must exist"] });
    const bot = world.users.get(BOT_ID);

    if (bot?.agent === undefined || bot.agent === null) throw new Error("Agent missing");
    bot.agent.suspended = true;
    expect(fieldsOf(await request("POST", path, { tag: "infra", assigneeId: BOT_ID }))).toEqual({
      assigneeId: ["must be an active board member able to own posts"],
    });
    bot.agent = null;
    expect((await request("POST", path, { tag: "infra", assigneeId: BOT_ID })).status).toBe(422);
  });

  it("includes departed or deactivated assignees once while removing them from candidates", async () => {
    const { request, board, world } = permissionHarness();
    board.memberIds = board.memberIds.filter((id) => id !== USER_IDS.maya);
    const bot = world.users.get(BOT_ID);

    if (bot === undefined) throw new Error("Agent missing");
    bot.status = "deactivated";
    const response = await request();
    expect(response.status).toBe(200);
    expect(field(response.json, "candidates")).not.toContain(USER_IDS.maya);
    expect(field(response.json, "candidates")).not.toContain(BOT_ID);
    expect(field(response.json, "users")).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ id: BOT_ID }),
        expect.objectContaining({ id: USER_IDS.maya }),
      ]),
    );
  });

  it("saves all statuses in order, accepts boundaries and removes disabled timers", async () => {
    const { server } = harness();

    const saved = await expectStatus<BoardAutomations>(
      server,
      "PUT",
      `${root}/sla_timers`,
      {
        blocked: timer,
        inProgress: timer,
        planned: { nudgeAfterMinutes: 1, escalateAfterMinutes: 43200 },
      },
      200,
    );

    expect(saved.slaTimers.map((row) => row.status)).toEqual(["planned", "in_progress", "blocked"]);

    const updated = await expectStatus<BoardAutomations>(
      server,
      "PUT",
      `${root}/sla_timers`,
      { planned: off, inProgress: timer, blocked: off },
      200,
    );

    expect(updated.slaTimers).toEqual([{ status: "in_progress", ...timer }]);

    const disabled = await expectStatus<BoardAutomations>(
      server,
      "PUT",
      `${root}/sla_timers`,
      { planned: off, inProgress: off, blocked: off },
      200,
    );

    expect(disabled.slaTimers).toEqual([]);
  });

  it.each([
    {
      row: { nudgeAfterMinutes: null, escalateAfterMinutes: 60 },
      errors: ["Nudge after minutes can't be blank"],
    },
    {
      row: { nudgeAfterMinutes: 60, escalateAfterMinutes: null },
      errors: ["Escalate after minutes can't be blank"],
    },
    {
      row: { nudgeAfterMinutes: 0, escalateAfterMinutes: 60 },
      errors: ["Nudge after minutes must be greater than 0"],
    },
    {
      row: { nudgeAfterMinutes: 1, escalateAfterMinutes: -1 },
      errors: [
        "Escalate after minutes must be greater than 0",
        "Escalate after minutes must be after the nudge threshold",
      ],
    },
    {
      row: { nudgeAfterMinutes: 43201, escalateAfterMinutes: 43202 },
      errors: [
        "Nudge after minutes must be less than or equal to 43200",
        "Escalate after minutes must be less than or equal to 43200",
      ],
    },
    {
      row: { nudgeAfterMinutes: 60, escalateAfterMinutes: 60 },
      errors: ["Escalate after minutes must be after the nudge threshold"],
    },
    {
      row: { nudgeAfterMinutes: 60, escalateAfterMinutes: 59 },
      errors: ["Escalate after minutes must be after the nudge threshold"],
    },
  ])("validates SLA thresholds $row", async ({ row, errors }) => {
    const { server } = harness();
    const failed = await send(server, "PUT", `${root}/sla_timers`, { ...sla, planned: row });
    expect(failed.status).toBe(422);
    expect(fieldsOf(failed)).toEqual({ planned: errors });
    expect(errorOf(failed.json).tag).toBe("Validation");
    expect(errorOf(failed.json).message).toContain(`Planned: ${errors[0]}`);
    expect((await get<BoardAutomations>(server, root)).slaTimers).toEqual([
      { status: "planned", nudgeAfterMinutes: 1440, escalateAfterMinutes: 2880 },
    ]);
  });

  it("validates every row first and joins row alerts in classic sentence order", async () => {
    const { server } = harness();

    const failed = await send(server, "PUT", `${root}/sla_timers`, {
      planned: off,
      inProgress: { nudgeAfterMinutes: 60, escalateAfterMinutes: 30 },
      blocked: { nudgeAfterMinutes: null, escalateAfterMinutes: 60 },
    });

    expect(fieldsOf(failed)).toEqual({
      inProgress: ["Escalate after minutes must be after the nudge threshold"],
      blocked: ["Nudge after minutes can't be blank"],
    });
    expect(errorOf(failed.json).message).toBe(
      "In progress: Escalate after minutes must be after the nudge threshold and Blocked: Nudge after minutes can't be blank",
    );
    expect((await get<BoardAutomations>(server, root)).slaTimers[0]?.status).toBe("planned");

    const all = await send(server, "PUT", `${root}/sla_timers`, {
      planned: { nudgeAfterMinutes: 0, escalateAfterMinutes: 60 },
      inProgress: { nudgeAfterMinutes: 0, escalateAfterMinutes: 60 },
      blocked: { nudgeAfterMinutes: 0, escalateAfterMinutes: 60 },
    });

    expect(errorOf(all.json).message).toBe(
      "Planned: Nudge after minutes must be greater than 0, In progress: Nudge after minutes must be greater than 0, and Blocked: Nudge after minutes must be greater than 0",
    );
  });

  it("auto-assigns new and newly tagged ownerless posts by first matching rule, preserving owners", async () => {
    const { server } = harness();
    const post = { name: "A new post", status: "planned", ownerId: null, message: null };

    const created = await expectStatus<ThreadDetail>(
      server,
      "POST",
      `/api/v1/rooms/${BOARD_ROOM_ID}/posts`,
      { ...post, tags: ["design", "bug"] },
      201,
    );

    expect(created.thread.work?.owner?.id).toBe(BOT_ID);

    const owned = await expectStatus<ThreadDetail>(
      server,
      "POST",
      `/api/v1/rooms/${BOARD_ROOM_ID}/posts`,
      { ...post, ownerId: VIEWER_ID, tags: ["bug"] },
      201,
    );

    expect(owned.thread.work?.owner?.id).toBe(VIEWER_ID);

    await expectStatus<BoardAutomations>(
      server,
      "POST",
      `${root}/tag_rules`,
      { tag: "api", assigneeId: VIEWER_ID },
      201,
    );

    const unchangedTags = await expectStatus<ThreadDetail>(
      server,
      "PATCH",
      "/api/v1/threads/9004/work",
      { tags: ["api"] },
      200,
    );

    expect(unchangedTags.thread.work?.owner).toBeNull();

    const tagged = await expectStatus<ThreadDetail>(
      server,
      "PATCH",
      "/api/v1/threads/9004/work",
      { tags: ["bug"] },
      200,
    );

    expect(tagged.thread.work?.owner?.id).toBe(BOT_ID);

    const kept = await expectStatus<ThreadDetail>(
      server,
      "PATCH",
      "/api/v1/threads/9009/work",
      { tags: ["bug"] },
      200,
    );

    expect(kept.thread.work?.owner?.id).toBe(VIEWER_ID);
    await send(server, "DELETE", `${root}/tag_rules/${BOARD_TAG_RULE_IDS.bug}`);

    const next = await expectStatus<ThreadDetail>(
      server,
      "POST",
      `/api/v1/rooms/${BOARD_ROOM_ID}/posts`,
      { ...post, tags: ["design", "bug"] },
      201,
    );

    expect(next.thread.work?.owner?.id).toBe(USER_IDS.maya);
    await send(server, "DELETE", `${root}/tag_rules/${BOARD_TAG_RULE_IDS.design}`);

    const unowned = await expectStatus<ThreadDetail>(
      server,
      "POST",
      `/api/v1/rooms/${BOARD_ROOM_ID}/posts`,
      { ...post, tags: ["design", "bug"] },
      201,
    );

    expect(unowned.thread.work?.owner).toBeNull();
  });
});

describe("mock automations authorization", () => {
  const writes = [
    { method: "GET", suffix: "", body: null },
    { method: "POST", suffix: "/tag_rules", body: { tag: "infra", assigneeId: VIEWER_ID } },
    { method: "DELETE", suffix: `/tag_rules/${BOARD_TAG_RULE_IDS.bug}`, body: null },
    { method: "PUT", suffix: "/sla_timers", body: sla },
  ];

  it.each(writes)(
    "returns 403 to an ordinary board member for $method $suffix",
    async ({ method, suffix, body }) => {
      const { viewer, board, request } = permissionHarness();
      viewer.role = "member";
      board.room.creatorId = USER_IDS.maya;
      const before = structuredClone(board.boardAutomations);
      const response = await request(method, `/rooms/${BOARD_ROOM_ID}/automations${suffix}`, body);
      expect(response.status).toBe(403);
      expect(errorOf(response.json)).toEqual({ tag: "Forbidden", message: "Forbidden" });
      expect(board.boardAutomations).toEqual(before);
    },
  );

  it.each(writes)(
    "returns 404 for a non-board, non-member or unknown room on $method $suffix",
    async ({ method, suffix, body }) => {
      const { viewer, board, request } = permissionHarness();
      viewer.role = "administrator";
      expect((await request(method, `/rooms/1/automations${suffix}`, body)).status).toBe(404);
      expect((await request(method, `/rooms/9999/automations${suffix}`, body)).status).toBe(404);
      board.memberIds = board.memberIds.filter((id) => id !== VIEWER_ID);
      expect(
        (await request(method, `/rooms/${BOARD_ROOM_ID}/automations${suffix}`, body)).status,
      ).toBe(404);
    },
  );

  it("admits the creator and administrators, and hides invisible or inactive memberships", async () => {
    const { viewer, board, request } = permissionHarness();
    viewer.role = "member";
    expect((await request()).status).toBe(200);
    board.room.creatorId = USER_IDS.maya;
    viewer.role = "administrator";
    expect((await request()).status).toBe(200);
    viewer.status = "deactivated";
    expect((await request()).status).toBe(404);
    viewer.status = "active";
    board.membership.involvement = "invisible";
    expect((await request()).status).toBe(404);
  });

  it("returns 404 when a rule belongs to a different board", async () => {
    const { request, world, board } = permissionHarness();
    world.rooms.set(901, {
      ...board,
      room: { ...board.room, id: 901 },
      boardAutomations: {
        tagRules: [{ id: 999, tag: "infra", assigneeId: VIEWER_ID }],
        slaTimers: [],
        nextTagRuleId: 1000,
      },
    });
    const response = await request("DELETE", `/rooms/${BOARD_ROOM_ID}/automations/tag_rules/999`);
    expect(response.status).toBe(404);
    expect(errorOf(response.json).message).toBe("Rule not found.");
    expect(world.rooms.get(901)?.boardAutomations?.tagRules).toHaveLength(1);
  });
});
