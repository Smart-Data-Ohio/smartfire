import { Schema } from "effect";
import type { ClientFrame as GeneratedClientFrame } from "../../gen/ClientFrame.ts";
import type { ResumePoint as GeneratedResumePoint } from "../../gen/ResumePoint.ts";
import type { ServerFrame as GeneratedServerFrame } from "../../gen/ServerFrame.ts";
import type { SyncEvent as GeneratedSyncEvent } from "../../gen/SyncEvent.ts";
import type { SyncPayload as GeneratedSyncPayload } from "../../gen/SyncPayload.ts";
import type { Typing as GeneratedTyping } from "../../gen/Typing.ts";
import { RoomId, UserId } from "./ids.ts";
import { MessageDTO, MessageRemoved } from "./message.ts";
import type { Assert, Pinned } from "./pin.ts";

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
  Schema.Struct({ t: Schema.Literal("hb") }),
]);

export type ClientFrame = typeof ClientFrame.Type;

export type ClientFramePin = Assert<Pinned<typeof ClientFrame, GeneratedClientFrame>>;

export const Typing = Schema.Struct({ userId: UserId, on: Schema.Boolean });

export type TypingPin = Assert<Pinned<typeof Typing, GeneratedTyping>>;

/** What happened: `type` names the event and `data` carries its body. */
export const SyncPayload = Schema.Union([
  Schema.Struct({ type: Schema.Literal("message.created"), data: MessageDTO }),
  Schema.Struct({ type: Schema.Literal("message.updated"), data: MessageDTO }),
  Schema.Struct({ type: Schema.Literal("message.removed"), data: MessageRemoved }),
  Schema.Struct({ type: Schema.Literal("typing"), data: Typing }),
]);

export type SyncPayloadPin = Assert<Pinned<typeof SyncPayload, GeneratedSyncPayload>>;

const eventFields = { seq: Schema.Int, topic: Topic };

/** One event in a batch: the hub's sequence, its topic, and the payload's `type` and `data`. */
export const SyncEvent = Schema.Union([
  Schema.Struct({
    ...eventFields,
    type: Schema.Literal("message.created"),
    data: MessageDTO,
  }),
  Schema.Struct({
    ...eventFields,
    type: Schema.Literal("message.updated"),
    data: MessageDTO,
  }),
  Schema.Struct({
    ...eventFields,
    type: Schema.Literal("message.removed"),
    data: MessageRemoved,
  }),
  Schema.Struct({ ...eventFields, type: Schema.Literal("typing"), data: Typing }),
]);

export type SyncEvent = typeof SyncEvent.Type;

export type SyncEventPin = Assert<Pinned<typeof SyncEvent, GeneratedSyncEvent>>;

/** A frame the server sends on `/api/v1/sync`. */
export const ServerFrame = Schema.Union([
  Schema.Struct({
    t: Schema.Literal("welcome"),
    epoch: Schema.String,
    seq: Schema.Int,
    resumed: Schema.Boolean,
  }),
  Schema.Struct({ t: Schema.Literal("batch"), events: Schema.Array(SyncEvent) }),
  Schema.Struct({ t: Schema.Literal("resync"), topics: Topics, reason: Schema.String }),
  Schema.Struct({ t: Schema.Literal("bye"), reconnect: Schema.Boolean, reason: Schema.String }),
]);

export type ServerFrame = typeof ServerFrame.Type;

export type ServerFramePin = Assert<Pinned<typeof ServerFrame, GeneratedServerFrame>>;
