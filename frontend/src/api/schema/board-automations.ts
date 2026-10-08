import { Schema } from "effect";
import type { BoardAutomations as GeneratedBoardAutomations } from "../../gen/BoardAutomations.ts";
import type { BoardSlaTimer as GeneratedBoardSlaTimer } from "../../gen/BoardSlaTimer.ts";
import type { BoardSlaTimerInput as GeneratedBoardSlaTimerInput } from "../../gen/BoardSlaTimerInput.ts";
import type { BoardTagRule as GeneratedBoardTagRule } from "../../gen/BoardTagRule.ts";
import type { CreateBoardTagRule as GeneratedCreateBoardTagRule } from "../../gen/CreateBoardTagRule.ts";
import type { UpdateBoardSlaTimers as GeneratedUpdateBoardSlaTimers } from "../../gen/UpdateBoardSlaTimers.ts";
import { RoomId, UserId } from "./ids.ts";
import type { Assert, Pinned } from "./pin.ts";
import { User } from "./user.ts";
import { WorkStatus } from "./work-parts.ts";

export const BoardTagRule = Schema.Struct({
  id: Schema.Int,
  tag: Schema.String,
  assigneeId: UserId,
});

export type BoardTagRule = typeof BoardTagRule.Type;

export type BoardTagRulePin = Assert<Pinned<typeof BoardTagRule, GeneratedBoardTagRule>>;

export const BoardSlaTimer = Schema.Struct({
  status: WorkStatus,
  nudgeAfterMinutes: Schema.Int,
  escalateAfterMinutes: Schema.Int,
});

export type BoardSlaTimer = typeof BoardSlaTimer.Type;

export type BoardSlaTimerPin = Assert<Pinned<typeof BoardSlaTimer, GeneratedBoardSlaTimer>>;

export const BoardAutomations = Schema.Struct({
  roomId: RoomId,
  tagRules: Schema.Array(BoardTagRule),
  slaTimers: Schema.Array(BoardSlaTimer),
  candidates: Schema.Array(UserId),
  users: Schema.Array(User),
});

export type BoardAutomations = typeof BoardAutomations.Type;

export type BoardAutomationsPin = Assert<
  Pinned<typeof BoardAutomations, GeneratedBoardAutomations>
>;

export const CreateBoardTagRule = Schema.Struct({
  tag: Schema.String,
  assigneeId: Schema.NullOr(UserId),
});

export type CreateBoardTagRule = typeof CreateBoardTagRule.Type;

export type CreateBoardTagRulePin = Assert<
  Pinned<typeof CreateBoardTagRule, GeneratedCreateBoardTagRule>
>;

export const BoardSlaTimerInput = Schema.Struct({
  nudgeAfterMinutes: Schema.NullOr(Schema.Int),
  escalateAfterMinutes: Schema.NullOr(Schema.Int),
});

export type BoardSlaTimerInput = typeof BoardSlaTimerInput.Type;

export type BoardSlaTimerInputPin = Assert<
  Pinned<typeof BoardSlaTimerInput, GeneratedBoardSlaTimerInput>
>;

export const UpdateBoardSlaTimers = Schema.Struct({
  planned: BoardSlaTimerInput,
  inProgress: BoardSlaTimerInput,
  blocked: BoardSlaTimerInput,
});

export type UpdateBoardSlaTimers = typeof UpdateBoardSlaTimers.Type;

export type UpdateBoardSlaTimersPin = Assert<
  Pinned<typeof UpdateBoardSlaTimers, GeneratedUpdateBoardSlaTimers>
>;
