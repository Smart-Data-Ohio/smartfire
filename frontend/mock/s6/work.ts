import type { ThreadPermissions } from "../../src/gen/ThreadPermissions.ts";
import type { WorkDetail } from "../../src/gen/WorkDetail.ts";
import type { WorkFacts } from "../../src/gen/WorkFacts.ts";
import type { WorkOwnerCandidate } from "../../src/gen/WorkOwnerCandidate.ts";
import type { S2Context } from "../s2/context.ts";
import { type ThreadRecord, threadStatus } from "../s2/model.ts";
import { VIEWER_ID, type World } from "../seed.ts";

export const emptyWorkDetail: WorkDetail = {
  resultMarkdown: null,
  resultHtml: null,
  resultUpdatedById: null,
  steps: [],
  history: [],
  ownerCandidates: [],
  handoffReceivers: [],
};

export function newWorkFacts(status: WorkFacts["status"] = "planned"): WorkFacts {
  return {
    status,
    owner: null,
    ownerActive: false,
    runUrl: null,
    resultUpdatedAt: null,
    links: [],
    tags: [],
    messageCount: 0,
  };
}

/** Mock room agents have the post/read/manage capabilities; suspended agents cannot own work. */
export function ownerCandidates(world: World, roomId: number): WorkOwnerCandidate[] {
  const members = world.rooms.get(roomId)?.memberIds ?? [];

  return members
    .flatMap((id) => {
      const user = world.users.get(id);

      if (user === undefined || user.status !== "active" || user.agent?.suspended) return [];

      return [
        {
          userId: id,
          provider: user.role === "bot" ? "Smartfire" : null,
          description: user.role === "bot" ? "Workspace assistant" : null,
        },
      ];
    })
    .sort((a, b) => {
      const left = world.users.get(a.userId);
      const right = world.users.get(b.userId);

      return (
        Number(left?.role === "bot") - Number(right?.role === "bot") ||
        (left?.name ?? "").toLowerCase().localeCompare((right?.name ?? "").toLowerCase()) ||
        a.userId - b.userId
      );
    });
}

type WorkPermissionContext = Pick<S2Context, "world" | "now">;

export function threadPermissions(
  ctx: WorkPermissionContext,
  thread: ThreadRecord,
): ThreadPermissions {
  const world = ctx.world();
  const viewer = world.users.get(VIEWER_ID);

  const active =
    viewer?.status === "active" &&
    viewer.role !== "bot" &&
    (world.rooms.get(thread.roomId)?.memberIds.includes(VIEWER_ID) ?? false);

  const moderator =
    active &&
    (viewer?.role === "administrator" ||
      world.rooms.get(thread.roomId)?.room.creatorId === VIEWER_ID);

  const settings = active && (moderator || thread.creatorId === VIEWER_ID);
  const tracked = thread.work != null;
  const manager = tracked && active && (settings || thread.work?.owner?.id === VIEWER_ID);
  const board = world.rooms.get(thread.roomId)?.room.kind === "board";
  const status = threadStatus(thread, ctx.now());

  return {
    canRename: board ? manager : settings,
    canClose: (board ? manager : settings) && status === "active",
    canReopen: active && status === "closed",
    canLock: moderator && status !== "locked",
    canUnlock: moderator && status === "locked",
    canDelete: moderator,
    canConvertWork: settings && !tracked,
    canManageWork: manager,
    canUpdateWorkStatus: manager,
    canAssignWork: settings && tracked,
    canRemoveWork: settings && tracked && !board,
  };
}

export function workDetail(ctx: S2Context, thread: ThreadRecord): WorkDetail | null {
  if (thread.work == null) return null;
  const allowed = threadPermissions(ctx, thread);
  const candidates = ownerCandidates(ctx.world(), thread.roomId);

  return {
    ...(thread.workDetail ?? emptyWorkDetail),
    ownerCandidates: allowed.canAssignWork ? candidates : [],
    handoffReceivers: allowed.canManageWork
      ? candidates.flatMap(({ userId }) => {
          const user = ctx.world().users.get(userId);

          return user?.agent != null && userId !== thread.work?.owner?.id
            ? [{ userId, agentId: user.agent.agentId }]
            : [];
        })
      : [],
  };
}
