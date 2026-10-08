import type { ThreadPermissions } from "../../src/gen/ThreadPermissions.ts";
import type { WorkDetail } from "../../src/gen/WorkDetail.ts";
import type { WorkStatus } from "../../src/gen/WorkStatus.ts";
import { validation } from "../http.ts";
import { field, type Json, stringArrayField } from "../json.ts";
import type { S2Context } from "../s2/context.ts";
import { type ThreadRecord, threadStatus } from "../s2/model.ts";
import {
  emptyWork,
  isTracked,
  workDetail as projectWorkDetail,
  type WorkRecord,
  ownerCandidates as workOwnerCandidates,
} from "../s4/work-model.ts";
import { VIEWER_ID } from "../seed.ts";

export const emptyWorkDetail: WorkDetail = {
  resultMarkdown: null,
  resultHtml: null,
  resultUpdatedById: null,
  steps: [],
  history: [],
  ownerCandidates: [],
  handoffReceivers: [],
};

export function newWorkFacts(
  status: WorkStatus = "planned",
  at = "1970-01-01T00:00:00.000000Z",
): WorkRecord {
  return { ...emptyWork(at), status };
}

export const ownerCandidates = workOwnerCandidates;

export function tagsOf(body: Json | undefined): string[] {
  const input = stringArrayField(body, "tags");

  if (input === null && field(body, "tags") !== undefined)
    throw validation("tags", "Tags must be a list");

  const tags = [
    ...new Set((input ?? []).map((tag) => tag.trim().toLowerCase()).filter((tag) => tag !== "")),
  ].sort();

  if (tags.length > 5) throw validation("tags", "Tags must have at most 5 entries");

  if (tags.some((tag) => [...tag].length > 30))
    throw validation("tags", "Tags are too long (maximum is 30 characters)");

  if (tags.some((tag) => !/^[a-z0-9][a-z0-9-]*$/.test(tag)))
    throw validation("tags", "Tags must contain only letters, numbers and hyphens");

  return tags;
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
  const tracked = isTracked(thread);
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
  return projectWorkDetail(ctx.world(), thread, threadPermissions(ctx, thread));
}
