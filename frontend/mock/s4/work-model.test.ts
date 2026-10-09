import { describe, expect, it } from "vitest";
import { buildWorld } from "../s2/seed.ts";
import { NOW } from "../s2/testing.ts";
import { USER_IDS } from "../seed.ts";
import { S4_WORK_IDS, seedWork } from "./seed.ts";
import {
  handoffReceivers,
  isAgentOwner,
  receiverError,
  workDetail,
  workPermissions,
  workUserIds,
} from "./work-model.ts";
import { workStateOf } from "./work-state.ts";

function model() {
  const world = seedWork(buildWorld(NOW, 1), NOW);
  const ember = world.users.get(USER_IDS.ember);
  const thread = world.threads.get(S4_WORK_IDS.agentOwned);

  if (ember === undefined || thread === undefined) throw new Error("Missing seeded work or Ember");

  world.users.set(ember.id, {
    ...ember,
    agent: { agentId: ember.id, kind: "workspace", status: "idle", suspended: false },
  });

  return { world, ember, thread };
}

describe("work permission checks match the real backend / classic", () => {
  it("checks posting, manage_threads and read_messages before already the owner", () => {
    const { world, ember, thread } = model();
    const capabilities = workStateOf(world).agentCapabilities;
    const error = () => receiverError(world, thread.roomId, ember.id, ember.id);

    capabilities.set(`${thread.roomId}:${ember.id}`, new Set(["read_messages"]));
    expect(error()).toBe(
      "Receiver must be an active agent member of this room with permission to post",
    );
    capabilities.set(`${thread.roomId}:${ember.id}`, new Set(["post_messages"]));
    expect(error()).toBe("Receiver must hold the manage_threads capability in this room");
    capabilities.set(`${thread.roomId}:${ember.id}`, new Set(["post_messages", "manage_threads"]));
    expect(error()).toBe("Receiver must hold the read_messages capability in this room");
    expect(handoffReceivers(world, thread.roomId, null)).toEqual([]);
    capabilities.set(
      `${thread.roomId}:${ember.id}`,
      new Set(["post_messages", "manage_threads", "read_messages"]),
    );
    expect(error()).toBe("Receiver is already the owner of this work");
    expect(handoffReceivers(world, thread.roomId, null)).toEqual([
      { agentId: ember.id, userId: ember.id },
    ]);
  });

  it("hides stop tracking on a managed board post", () => {
    const { world } = model();
    const post = workStateOf(world).boardPosts[0];

    if (post === undefined) throw new Error("Missing seeded board post");

    expect(workPermissions(post, true, USER_IDS.riel)).toMatchObject({
      canManageWork: true,
      canUpdateWorkStatus: true,
      canAssignWork: true,
      canRemoveWork: false,
    });
  });

  it("includes actors and editors in users, without including history owner snapshots", () => {
    const { world, thread } = model();
    const detail = workDetail(world, thread, { canAssignWork: false, canManageWork: false });

    if (detail === null) throw new Error("Missing seeded work detail");

    const snapshotOnly = {
      ...detail,
      resultUpdatedById: USER_IDS.maya,
      history: detail.history.map((entry) => ({
        ...entry,
        actorId: USER_IDS.jonah,
        fromOwner: { userId: USER_IDS.dana, name: "Dana" },
        toOwner: { userId: USER_IDS.theo, name: "Theo" },
      })),
    };

    expect([...new Set(workUserIds(thread, snapshotOnly))]).toEqual([
      USER_IDS.maya,
      USER_IDS.jonah,
    ]);
  });

  it("matches an agent record rather than any bot owner", () => {
    const { world, ember } = model();

    expect(isAgentOwner(world, ember.id)).toBe(true);
    world.users.set(ember.id, { ...ember, role: "bot", agent: null });
    expect(isAgentOwner(world, ember.id)).toBe(false);
    expect(isAgentOwner(world, null)).toBe(false);
  });
});
