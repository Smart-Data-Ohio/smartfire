import { describe, expect, it } from "vitest";
import { NOW } from "../s2/testing.ts";
import { buildWorld } from "../s3/seed.ts";
import { BOT_ID, USER_IDS, VIEWER_ID } from "../seed.ts";
import { BOARD_ROOM_ID } from "./seed.ts";
import { ownerCandidates, threadPermissions } from "./work.ts";

function memberWorld() {
  const world = buildWorld(NOW, 1);
  const viewer = world.users.get(VIEWER_ID);
  const room = world.rooms.get(BOARD_ROOM_ID);
  const post = world.threads.get(9006);

  if (viewer === undefined || room === undefined || post === undefined)
    throw new Error("Seed missing");
  viewer.role = "member";
  room.room.creatorId = USER_IDS.priya;

  return { world, viewer, post, ctx: { world: () => world, now: () => NOW } };
}

describe("work permissions and candidates", () => {
  it("lets the owner move status and edit results, while assignment belongs to creator/moderator", () => {
    const { viewer, post, ctx } = memberWorld();
    expect(threadPermissions(ctx, post).canManageWork).toBe(false);

    if (post.work === undefined || post.work === null) throw new Error("Work missing");
    post.work.owner = viewer;
    const allowed = threadPermissions(ctx, post);
    expect(allowed.canManageWork).toBe(true);
    expect(allowed.canUpdateWorkStatus).toBe(true);
    expect(allowed.canAssignWork).toBe(false);
    expect(allowed.canRemoveWork).toBe(false);
    viewer.status = "deactivated";
    expect(threadPermissions(ctx, post).canManageWork).toBe(false);
  });
  it("offers stop tracking only outside boards and convert only before tracking", () => {
    const { world, viewer, post, ctx } = memberWorld();
    viewer.role = "administrator";
    expect(threadPermissions(ctx, post).canRemoveWork).toBe(false);
    const ordinary = world.threads.get(1);

    if (ordinary === undefined) throw new Error("Thread missing");
    expect(threadPermissions(ctx, ordinary).canConvertWork).toBe(true);
    ordinary.work = post.work ?? null;
    expect(threadPermissions(ctx, ordinary).canConvertWork).toBe(false);
    expect(threadPermissions(ctx, ordinary).canRemoveWork).toBe(true);
  });
  it("orders humans before eligible agents and excludes unavailable owners", () => {
    const { world } = memberWorld();
    const candidates = ownerCandidates(world, BOARD_ROOM_ID);
    expect(candidates.at(-1)?.userId).toBe(BOT_ID);
    expect(candidates.map(({ userId }) => userId)).not.toContain(10);
    const bot = world.users.get(BOT_ID);

    if (bot?.agent === undefined || bot.agent === null) throw new Error("Agent missing");
    bot.agent.suspended = true;
    expect(ownerCandidates(world, BOARD_ROOM_ID).map(({ userId }) => userId)).not.toContain(BOT_ID);
  });
});
