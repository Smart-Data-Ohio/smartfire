import { Schema } from "effect";
import type { ChangeStageRole as GeneratedChangeStageRole } from "../../gen/ChangeStageRole.ts";
import type { LowerHand as GeneratedLowerHand } from "../../gen/LowerHand.ts";
import type { StageDetail as GeneratedStageDetail } from "../../gen/StageDetail.ts";
import type { StageMember as GeneratedStageMember } from "../../gen/StageMember.ts";
import type { StageState as GeneratedStageState } from "../../gen/StageState.ts";
import type { StageStream as GeneratedStageStream } from "../../gen/StageStream.ts";
import type { StageStreamStopped as GeneratedStageStreamStopped } from "../../gen/StageStreamStopped.ts";
import type { StartStageStream as GeneratedStartStageStream } from "../../gen/StartStageStream.ts";
import type { StopStageStream as GeneratedStopStageStream } from "../../gen/StopStageStream.ts";
import type { StreamQuality as GeneratedStreamQuality } from "../../gen/StreamQuality.ts";
import { MembershipId, RoomId, UserId } from "./ids.ts";
import type { Assert, Pinned } from "./pin.ts";
import { StageRole } from "./room.ts";
import { Timestamp } from "./time.ts";
import { User } from "./user.ts";

/** A stage's live stream. */
export const StreamId = Schema.Int.pipe(Schema.brand("StreamId"));

export type StreamId = typeof StreamId.Type;

/** One stage member: their role, raised hand and server mute. */
export const StageMember = Schema.Struct({
  membershipId: MembershipId,
  userId: UserId,
  role: StageRole,
  handRaisedAt: Schema.NullOr(Timestamp),
  serverMuted: Schema.Boolean,
});

export type StageMember = typeof StageMember.Type;

export type StageMemberPin = Assert<Pinned<typeof StageMember, GeneratedStageMember>>;

export const StreamQuality = Schema.Literals(["720p15", "1080p15", "1080p30", "1080p60"]);

export type StreamQuality = typeof StreamQuality.Type;

export type StreamQualityPin = Assert<Pinned<typeof StreamQuality, GeneratedStreamQuality>>;

export const StageStream = Schema.Struct({
  id: StreamId,
  membershipId: MembershipId,
  userId: UserId,
  identity: Schema.NullOr(Schema.String),
  quality: StreamQuality,
  startedAt: Timestamp,
});

export type StageStream = typeof StageStream.Type;

export type StageStreamPin = Assert<Pinned<typeof StageStream, GeneratedStageStream>>;

/** A stage's roster and stream, the same for every viewer; also the `stage.updated` event's data. */
export const StageState = Schema.Struct({
  roomId: RoomId,
  members: Schema.Array(StageMember),
  live: Schema.NullOr(StageStream),
});

export type StageState = typeof StageState.Type;

export type StageStatePin = Assert<Pinned<typeof StageState, GeneratedStageState>>;

/** `GET /api/v1/rooms/:id/stage`. */
export const StageDetail = Schema.Struct({ stage: StageState, users: Schema.Array(User) });

export type StageDetail = typeof StageDetail.Type;

export type StageDetailPin = Assert<Pinned<typeof StageDetail, GeneratedStageDetail>>;

/** `PATCH /api/v1/rooms/:id/stage/members/:membershipId`'s body. */
export const ChangeStageRole = Schema.Struct({ role: StageRole });

export type ChangeStageRolePin = Assert<Pinned<typeof ChangeStageRole, GeneratedChangeStageRole>>;

/** `DELETE /api/v1/rooms/:id/stage/hand`'s query: `null` for the viewer's own hand. */
export const LowerHand = Schema.Struct({ membershipId: Schema.NullOr(MembershipId) });

export type LowerHandPin = Assert<Pinned<typeof LowerHand, GeneratedLowerHand>>;

/** `POST /api/v1/rooms/:id/stage/stream`'s body. */
export const StartStageStream = Schema.Struct({ quality: StreamQuality });

export type StartStageStreamPin = Assert<
  Pinned<typeof StartStageStream, GeneratedStartStageStream>
>;

/** `DELETE /api/v1/rooms/:id/stage/stream`'s query: only this stream, when given. */
export const StopStageStream = Schema.Struct({ streamId: Schema.NullOr(StreamId) });

export type StopStageStreamPin = Assert<Pinned<typeof StopStageStream, GeneratedStopStageStream>>;

/** The `stage.stream.stopped` event: someone else ended the viewer's stream. */
export const StageStreamStopped = Schema.Struct({ roomId: RoomId });

export type StageStreamStoppedPin = Assert<
  Pinned<typeof StageStreamStopped, GeneratedStageStreamStopped>
>;
