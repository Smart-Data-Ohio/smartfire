import { Schema } from "effect";
import type { CreateWorkHandoff as GeneratedCreateWorkHandoff } from "../../gen/CreateWorkHandoff.ts";
import type { UpdateWork as GeneratedUpdateWork } from "../../gen/UpdateWork.ts";
import type { WorkFilter as GeneratedWorkFilter } from "../../gen/WorkFilter.ts";
import type { WorkList as GeneratedWorkList } from "../../gen/WorkList.ts";
import type { WorkListRow as GeneratedWorkListRow } from "../../gen/WorkListRow.ts";
import { AgentId, UserId } from "./ids.ts";
import type { Assert, Pinned } from "./pin.ts";
import { Thread } from "./thread.ts";
import { Timestamp } from "./time.ts";
import { User } from "./user.ts";
import { WorkStatus } from "./work-parts.ts";

export {
  WorkDetail,
  WorkFacts,
  WorkHandoffReceiver,
  WorkHistoryEntry,
  WorkHistoryHandoff,
  WorkHistoryKind,
  WorkLink,
  WorkLinkKind,
  WorkOwnerCandidate,
  WorkOwnerSnapshot,
  WorkPullRequestState,
  WorkStatus,
} from "./work-parts.ts";

/** `GET /api/v1/work?state=`: `open` (the default) is everything not done. */
export const WorkFilter = Schema.Literals(["open", "done", "all", "agents", "boards"]);

export type WorkFilter = typeof WorkFilter.Type;

export type WorkFilterPin = Assert<Pinned<typeof WorkFilter, GeneratedWorkFilter>>;

/** A work list row: the thread (its `work` never `null`), the room's name, board or not. */
export const WorkListRow = Schema.Struct({
  thread: Thread,
  roomName: Schema.String,
  board: Schema.Boolean,
  updatedAt: Timestamp,
});

export type WorkListRow = typeof WorkListRow.Type;

export type WorkListRowPin = Assert<Pinned<typeof WorkListRow, GeneratedWorkListRow>>;

/** Most recently updated first, not paged, no live updates: refetch when shown again. */
export const WorkList = Schema.Struct({
  threads: Schema.Array(WorkListRow),
  users: Schema.Array(User),
});

export type WorkList = typeof WorkList.Type;

export type WorkListPin = Assert<Pinned<typeof WorkList, GeneratedWorkList>>;

/**
 * `PATCH /api/v1/threads/:id/work`. Leave a key out to leave it alone; `null` clears it
 * (`status: null` stops tracking, which needs `ownerId: null` too when there's an owner).
 */
export const UpdateWork = Schema.Struct({
  status: Schema.optionalKey(Schema.NullOr(WorkStatus)),
  ownerId: Schema.optionalKey(Schema.NullOr(UserId)),
  resultMarkdown: Schema.optionalKey(Schema.NullOr(Schema.String)),
});

export type UpdateWork = typeof UpdateWork.Type;

export type UpdateWorkPin = Assert<Pinned<typeof UpdateWork, GeneratedUpdateWork>>;

/** `POST /api/v1/threads/:id/work/handoff`: the receiver comes from `handoffReceivers`. */
export const CreateWorkHandoff = Schema.Struct({
  receiverAgentId: AgentId,
  summary: Schema.String,
  links: Schema.Array(Schema.String),
  openQuestions: Schema.Array(Schema.String),
});

export type CreateWorkHandoff = typeof CreateWorkHandoff.Type;

export type CreateWorkHandoffPin = Assert<
  Pinned<typeof CreateWorkHandoff, GeneratedCreateWorkHandoff>
>;
