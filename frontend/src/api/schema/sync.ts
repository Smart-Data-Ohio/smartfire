import { Schema } from "effect";
import type { ClientFrame as GeneratedClientFrame } from "../../gen/ClientFrame.ts";
import type { ResumePoint as GeneratedResumePoint } from "../../gen/ResumePoint.ts";
import type { ServerFrame as GeneratedServerFrame } from "../../gen/ServerFrame.ts";
import type { SyncEvent as GeneratedSyncEvent } from "../../gen/SyncEvent.ts";
import type { SyncPayload as GeneratedSyncPayload } from "../../gen/SyncPayload.ts";
import type { Typing as GeneratedTyping } from "../../gen/Typing.ts";
import { PinState, SavedChanged } from "./actions.ts";
import { ActivityItemChanged, ActivityItemRemoved } from "./activity.ts";
import { CustomStyles, WorkspaceBranding } from "./admin.ts";
import { AgentStatusChanged, AgentStepsChanged, ApprovalUpdated } from "./agents.ts";
import { BoardAutomationsChanged } from "./board-automations.ts";
import { MessageCards, PollBallot, PollUpdated } from "./cards.ts";
import { ScheduledMessage, ScheduledMessageRemoved } from "./composer.ts";
import { EventsChanged } from "./events.ts";
import { HuddleNotice, HuddlePresence, HuddleRing, HuddleRoleChanged } from "./huddle.ts";
import { RoomId, ThreadId, UserId } from "./ids.ts";
import { MessageDTO, MessageRemoved } from "./message.ts";
import { RoomCategoryRemoved } from "./organize.ts";
import type { Assert, Pinned } from "./pin.ts";
import { UserPresence } from "./presence.ts";
import { MessageReactions } from "./reaction.ts";
import { RoomRead, RoomUnread } from "./read.ts";
import { Settings } from "./settings.ts";
import { RoomCategory, SidebarRow, SidebarRowRemoved } from "./sidebar.ts";
import { StageState, StageStreamStopped } from "./stage.ts";
import {
  Thread,
  ThreadIndicatorChanged,
  ThreadRead,
  ThreadRemoved,
  ThreadUnread,
} from "./thread.ts";
import { User } from "./user.ts";
import { WorkspaceLayout } from "./workspace-layout.ts";

/** A sync topic: `user`, `room:<id>` or `thread:<id>`. */
const Topic = Schema.String;

const Topics = Schema.Array(Topic);

export const ResumePoint = Schema.Struct({ epoch: Schema.String, seq: Schema.Int });

export type ResumePointPin = Assert<Pinned<typeof ResumePoint, GeneratedResumePoint>>;

/** A frame the client sends on `/api/v1/sync`. */
export const ClientFrame = Schema.Union([
  Schema.Struct({
    t: Schema.Literal("hello"),
    v: Schema.Int,
    resume: Schema.NullOr(ResumePoint),
    topics: Topics,
  }),
  Schema.Struct({ t: Schema.Literal("sub"), topics: Topics }),
  Schema.Struct({ t: Schema.Literal("unsub"), topics: Topics }),
  Schema.Struct({ t: Schema.Literal("typing"), conv: Topic, on: Schema.Boolean }),
  Schema.Struct({ t: Schema.Literal("present"), room: RoomId }),
  Schema.Struct({ t: Schema.Literal("absent"), room: RoomId }),
  Schema.Struct({ t: Schema.Literal("hb"), active: Schema.Boolean }),
]);

export type ClientFrame = typeof ClientFrame.Type;

export type ClientFramePin = Assert<Pinned<typeof ClientFrame, GeneratedClientFrame>>;

export const Typing = Schema.Struct({ userId: UserId, on: Schema.Boolean });

export type TypingPin = Assert<Pinned<typeof Typing, GeneratedTyping>>;

const ThreadGithubUpdated = Schema.Struct({
  roomId: RoomId,
  threadId: ThreadId,
  pullRequestId: Schema.Int,
});

/** What happened: `type` names the event and `data` carries its body. */
export const SyncPayload = Schema.Union([
  Schema.Struct({ type: Schema.Literal("user.updated"), data: User }),
  Schema.Struct({ type: Schema.Literal("settings.updated"), data: Settings }),
  Schema.Struct({ type: Schema.Literal("message.created"), data: MessageDTO }),
  Schema.Struct({ type: Schema.Literal("message.updated"), data: MessageDTO }),
  Schema.Struct({ type: Schema.Literal("message.removed"), data: MessageRemoved }),
  Schema.Struct({ type: Schema.Literal("typing"), data: Typing }),
  Schema.Struct({ type: Schema.Literal("room.unread"), data: RoomUnread }),
  Schema.Struct({ type: Schema.Literal("room.read"), data: RoomRead }),
  Schema.Struct({ type: Schema.Literal("sidebar.row.upserted"), data: SidebarRow }),
  Schema.Struct({ type: Schema.Literal("sidebar.row.removed"), data: SidebarRowRemoved }),
  Schema.Struct({ type: Schema.Literal("presence"), data: UserPresence }),
  Schema.Struct({ type: Schema.Literal("message.reactions"), data: MessageReactions }),
  Schema.Struct({ type: Schema.Literal("message.pinned"), data: PinState }),
  Schema.Struct({ type: Schema.Literal("thread.indicator"), data: ThreadIndicatorChanged }),
  Schema.Struct({ type: Schema.Literal("thread.created"), data: Thread }),
  Schema.Struct({ type: Schema.Literal("thread.updated"), data: Thread }),
  Schema.Struct({ type: Schema.Literal("thread.github.updated"), data: ThreadGithubUpdated }),
  Schema.Struct({ type: Schema.Literal("thread.removed"), data: ThreadRemoved }),
  Schema.Struct({
    type: Schema.Literal("board.automations.changed"),
    data: BoardAutomationsChanged,
  }),
  Schema.Struct({ type: Schema.Literal("thread.unread"), data: ThreadUnread }),
  Schema.Struct({ type: Schema.Literal("thread.read"), data: ThreadRead }),
  Schema.Struct({ type: Schema.Literal("saved.changed"), data: SavedChanged }),
  Schema.Struct({ type: Schema.Literal("activity.item"), data: ActivityItemChanged }),
  Schema.Struct({ type: Schema.Literal("activity.removed"), data: ActivityItemRemoved }),
  Schema.Struct({ type: Schema.Literal("scheduled.changed"), data: ScheduledMessage }),
  Schema.Struct({ type: Schema.Literal("scheduled.removed"), data: ScheduledMessageRemoved }),
  Schema.Struct({ type: Schema.Literal("sidebar.category.upserted"), data: RoomCategory }),
  Schema.Struct({ type: Schema.Literal("sidebar.category.removed"), data: RoomCategoryRemoved }),
  Schema.Struct({ type: Schema.Literal("poll.updated"), data: PollUpdated }),
  Schema.Struct({ type: Schema.Literal("poll.ballot"), data: PollBallot }),
  Schema.Struct({ type: Schema.Literal("message.cards"), data: MessageCards }),
  Schema.Struct({ type: Schema.Literal("events.changed"), data: EventsChanged }),
  Schema.Struct({ type: Schema.Literal("agent.status"), data: AgentStatusChanged }),
  Schema.Struct({ type: Schema.Literal("agent.steps"), data: AgentStepsChanged }),
  Schema.Struct({ type: Schema.Literal("approval.updated"), data: ApprovalUpdated }),
  Schema.Struct({ type: Schema.Literal("huddle.presence"), data: HuddlePresence }),
  Schema.Struct({ type: Schema.Literal("huddle.role"), data: HuddleRoleChanged }),
  Schema.Struct({ type: Schema.Literal("huddle.notice"), data: HuddleNotice }),
  Schema.Struct({ type: Schema.Literal("huddle.ring"), data: HuddleRing }),
  Schema.Struct({ type: Schema.Literal("stage.updated"), data: StageState }),
  Schema.Struct({ type: Schema.Literal("stage.stream.stopped"), data: StageStreamStopped }),
  Schema.Struct({ type: Schema.Literal("workspace.updated"), data: WorkspaceBranding }),
  Schema.Struct({ type: Schema.Literal("workspace.styles.updated"), data: CustomStyles }),
  Schema.Struct({ type: Schema.Literal("workspace.layout.updated"), data: WorkspaceLayout }),
]);

export type SyncPayload = typeof SyncPayload.Type;

export type SyncPayloadPin = Assert<Pinned<typeof SyncPayload, GeneratedSyncPayload>>;

const eventFields = { seq: Schema.Int, topic: Topic };

/**
 * One event in a batch: the hub's sequence, its topic, and the payload's `type` and `data`.
 * Kept in step with `SyncPayload` above (the pins fail otherwise).
 */
export const SyncEvent = Schema.Union([
  Schema.Struct({ ...eventFields, type: Schema.Literal("user.updated"), data: User }),
  Schema.Struct({ ...eventFields, type: Schema.Literal("settings.updated"), data: Settings }),
  Schema.Struct({ ...eventFields, type: Schema.Literal("message.created"), data: MessageDTO }),
  Schema.Struct({ ...eventFields, type: Schema.Literal("message.updated"), data: MessageDTO }),
  Schema.Struct({ ...eventFields, type: Schema.Literal("message.removed"), data: MessageRemoved }),
  Schema.Struct({ ...eventFields, type: Schema.Literal("typing"), data: Typing }),
  Schema.Struct({ ...eventFields, type: Schema.Literal("room.unread"), data: RoomUnread }),
  Schema.Struct({ ...eventFields, type: Schema.Literal("room.read"), data: RoomRead }),
  Schema.Struct({ ...eventFields, type: Schema.Literal("sidebar.row.upserted"), data: SidebarRow }),
  Schema.Struct({
    ...eventFields,
    type: Schema.Literal("sidebar.row.removed"),
    data: SidebarRowRemoved,
  }),
  Schema.Struct({ ...eventFields, type: Schema.Literal("presence"), data: UserPresence }),
  Schema.Struct({
    ...eventFields,
    type: Schema.Literal("message.reactions"),
    data: MessageReactions,
  }),
  Schema.Struct({ ...eventFields, type: Schema.Literal("message.pinned"), data: PinState }),
  Schema.Struct({
    ...eventFields,
    type: Schema.Literal("thread.indicator"),
    data: ThreadIndicatorChanged,
  }),
  Schema.Struct({ ...eventFields, type: Schema.Literal("thread.created"), data: Thread }),
  Schema.Struct({ ...eventFields, type: Schema.Literal("thread.updated"), data: Thread }),
  Schema.Struct({
    ...eventFields,
    type: Schema.Literal("thread.github.updated"),
    data: ThreadGithubUpdated,
  }),
  Schema.Struct({ ...eventFields, type: Schema.Literal("thread.removed"), data: ThreadRemoved }),
  Schema.Struct({
    ...eventFields,
    type: Schema.Literal("board.automations.changed"),
    data: BoardAutomationsChanged,
  }),
  Schema.Struct({ ...eventFields, type: Schema.Literal("thread.unread"), data: ThreadUnread }),
  Schema.Struct({ ...eventFields, type: Schema.Literal("thread.read"), data: ThreadRead }),
  Schema.Struct({ ...eventFields, type: Schema.Literal("saved.changed"), data: SavedChanged }),
  Schema.Struct({
    ...eventFields,
    type: Schema.Literal("activity.item"),
    data: ActivityItemChanged,
  }),
  Schema.Struct({
    ...eventFields,
    type: Schema.Literal("activity.removed"),
    data: ActivityItemRemoved,
  }),
  Schema.Struct({
    ...eventFields,
    type: Schema.Literal("scheduled.changed"),
    data: ScheduledMessage,
  }),
  Schema.Struct({
    ...eventFields,
    type: Schema.Literal("scheduled.removed"),
    data: ScheduledMessageRemoved,
  }),
  Schema.Struct({
    ...eventFields,
    type: Schema.Literal("sidebar.category.upserted"),
    data: RoomCategory,
  }),
  Schema.Struct({
    ...eventFields,
    type: Schema.Literal("sidebar.category.removed"),
    data: RoomCategoryRemoved,
  }),
  Schema.Struct({ ...eventFields, type: Schema.Literal("poll.updated"), data: PollUpdated }),
  Schema.Struct({ ...eventFields, type: Schema.Literal("poll.ballot"), data: PollBallot }),
  Schema.Struct({ ...eventFields, type: Schema.Literal("message.cards"), data: MessageCards }),
  Schema.Struct({ ...eventFields, type: Schema.Literal("events.changed"), data: EventsChanged }),
  Schema.Struct({
    ...eventFields,
    type: Schema.Literal("agent.status"),
    data: AgentStatusChanged,
  }),
  Schema.Struct({ ...eventFields, type: Schema.Literal("agent.steps"), data: AgentStepsChanged }),
  Schema.Struct({
    ...eventFields,
    type: Schema.Literal("approval.updated"),
    data: ApprovalUpdated,
  }),
  Schema.Struct({
    ...eventFields,
    type: Schema.Literal("huddle.presence"),
    data: HuddlePresence,
  }),
  Schema.Struct({ ...eventFields, type: Schema.Literal("huddle.role"), data: HuddleRoleChanged }),
  Schema.Struct({ ...eventFields, type: Schema.Literal("huddle.notice"), data: HuddleNotice }),
  Schema.Struct({ ...eventFields, type: Schema.Literal("huddle.ring"), data: HuddleRing }),
  Schema.Struct({ ...eventFields, type: Schema.Literal("stage.updated"), data: StageState }),
  Schema.Struct({
    ...eventFields,
    type: Schema.Literal("stage.stream.stopped"),
    data: StageStreamStopped,
  }),
  Schema.Struct({
    ...eventFields,
    type: Schema.Literal("workspace.updated"),
    data: WorkspaceBranding,
  }),
  Schema.Struct({
    ...eventFields,
    type: Schema.Literal("workspace.styles.updated"),
    data: CustomStyles,
  }),
  Schema.Struct({
    ...eventFields,
    type: Schema.Literal("workspace.layout.updated"),
    data: WorkspaceLayout,
  }),
]);

export type SyncEvent = typeof SyncEvent.Type;

export type SyncEventPin = Assert<Pinned<typeof SyncEvent, GeneratedSyncEvent>>;

/** The `type` of every event this client understands. */
export type SyncEventType = SyncEvent["type"];

/** A frame the server sends on `/api/v1/sync`. */
export const ServerFrame = Schema.Union([
  Schema.Struct({
    t: Schema.Literal("welcome"),
    epoch: Schema.String,
    seq: Schema.Int,
    resumed: Schema.Boolean,
    replayThrough: Schema.Int,
  }),
  Schema.Struct({ t: Schema.Literal("batch"), events: Schema.Array(SyncEvent) }),
  Schema.Struct({ t: Schema.Literal("resync"), topics: Topics, reason: Schema.String }),
  Schema.Struct({ t: Schema.Literal("bye"), reconnect: Schema.Boolean, reason: Schema.String }),
  Schema.Struct({ t: Schema.Literal("ping") }),
]);

export type ServerFrame = typeof ServerFrame.Type;

export type ServerFramePin = Assert<Pinned<typeof ServerFrame, GeneratedServerFrame>>;
