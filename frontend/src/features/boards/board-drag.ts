/** The rules a board card's drag follows: who may move it and where it lands. */
import type { WorkStatus } from "../../gen/WorkStatus.ts";
import { boardPostIds } from "../../store/boards.ts";
import type { State } from "../../store/state.ts";
import { WORK_STATUSES } from "./board-format.ts";

/**
 * Whether the viewer may move the post to another status, as the server's
 * `canUpdateWorkStatus` decides it for a board post: the board's creator or an administrator
 * (`canAdminister`), the post's creator, or its owner. The listing carries no per-post
 * permissions, so this follows the owner live.
 */
export function canMoveWork(state: State, roomId: number, threadId: number): boolean {
  const board = state.boards[roomId];
  const work = state.threads[threadId]?.work;
  const viewerId = state.me?.user.id ?? state.boot?.user.id;

  if (board === undefined || work == null || viewerId === undefined) {
    return false;
  }

  return (
    board.canAdminister ||
    state.threads[threadId]?.creatorId === viewerId ||
    work.owner?.id === viewerId
  );
}

/** Where a post would show in another status's column, or `null` where it already is. */
export interface Landing {
  readonly status: WorkStatus;
  /** Its place among that column's cards. */
  readonly index: number;
}

/**
 * Where moving the post to `status` puts it. Columns keep the board's order (latest activity
 * first) and a status change doesn't touch activity, so the post lands among the column's cards
 * where that order places it, not where it was dropped.
 */
export function landingFor(
  state: State,
  roomId: number,
  threadId: number,
  status: WorkStatus | null,
): Landing | null {
  const current = state.threads[threadId]?.work?.status;

  if (status === null || current === undefined || current === status) {
    return null;
  }

  const order = boardPostIds(state, roomId);
  const at = order.indexOf(threadId);

  if (at === -1) {
    return null;
  }

  const index = order
    .slice(0, at)
    .filter((id) => state.threads[id]?.work?.status === status).length;

  return { status, index };
}

/** The status `by` columns over from `status`, stopping at the first and last. */
export function statusBeside(status: WorkStatus, by: number): WorkStatus {
  const at = WORK_STATUSES.indexOf(status);
  const next = Math.min(Math.max(at + by, 0), WORK_STATUSES.length - 1);

  return WORK_STATUSES[next] ?? status;
}
