import { Schema } from "effect";
import type { Switcher as GeneratedSwitcher } from "../../gen/Switcher.ts";
import type { SwitcherPerson as GeneratedSwitcherPerson } from "../../gen/SwitcherPerson.ts";
import type { SwitcherRoom as GeneratedSwitcherRoom } from "../../gen/SwitcherRoom.ts";
import type { SwitcherRoomKind as GeneratedSwitcherRoomKind } from "../../gen/SwitcherRoomKind.ts";
import type { SwitcherThread as GeneratedSwitcherThread } from "../../gen/SwitcherThread.ts";
import { RoomId, ThreadId, UserId } from "./ids.ts";
import type { Assert, Pinned } from "./pin.ts";
import { User } from "./user.ts";

export const SwitcherRoomKind = Schema.Literals([
  "channel",
  "dm",
  "group",
  "voice",
  "stage",
  "board",
]);

export type SwitcherRoomKind = typeof SwitcherRoomKind.Type;

export type SwitcherRoomKindPin = Assert<
  Pinned<typeof SwitcherRoomKind, GeneratedSwitcherRoomKind>
>;

export const SwitcherRoom = Schema.Struct({
  roomId: RoomId,
  name: Schema.String,
  kind: SwitcherRoomKind,
  iconName: Schema.NullOr(Schema.String),
  unread: Schema.Boolean,
  muted: Schema.Boolean,
  favorite: Schema.Boolean,
});

export type SwitcherRoom = typeof SwitcherRoom.Type;

export type SwitcherRoomPin = Assert<Pinned<typeof SwitcherRoom, GeneratedSwitcherRoom>>;

export const SwitcherPerson = Schema.Struct({
  userId: UserId,
  directRoomId: Schema.NullOr(RoomId),
});

export type SwitcherPerson = typeof SwitcherPerson.Type;

export type SwitcherPersonPin = Assert<Pinned<typeof SwitcherPerson, GeneratedSwitcherPerson>>;

export const SwitcherThread = Schema.Struct({
  threadId: ThreadId,
  name: Schema.String,
  roomId: RoomId,
  roomName: Schema.NullOr(Schema.String),
});

export type SwitcherThread = typeof SwitcherThread.Type;

export type SwitcherThreadPin = Assert<Pinned<typeof SwitcherThread, GeneratedSwitcherThread>>;

/** `GET /api/v1/switcher`: rooms, people and recent threads; the client ranks. */
export const Switcher = Schema.Struct({
  rooms: Schema.Array(SwitcherRoom),
  people: Schema.Array(SwitcherPerson),
  threads: Schema.Array(SwitcherThread),
  users: Schema.Array(User),
});

export type Switcher = typeof Switcher.Type;

export type SwitcherPin = Assert<Pinned<typeof Switcher, GeneratedSwitcher>>;
