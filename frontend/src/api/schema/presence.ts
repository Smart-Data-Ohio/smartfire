import { Schema } from "effect";
import type { Presence as GeneratedPresence } from "../../gen/Presence.ts";
import type { PresenceList as GeneratedPresenceList } from "../../gen/PresenceList.ts";
import type { UserPresence as GeneratedUserPresence } from "../../gen/UserPresence.ts";
import { UserId } from "./ids.ts";
import type { Assert, Pinned } from "./pin.ts";

export const Presence = Schema.Literals(["online", "idle", "offline", "dnd"]);

export type Presence = typeof Presence.Type;

export type PresencePin = Assert<Pinned<typeof Presence, GeneratedPresence>>;

/** One person's workspace presence and status line; also the `presence` event's data. */
export const UserPresence = Schema.Struct({
  userId: UserId,
  presence: Presence,
  statusText: Schema.NullOr(Schema.String),
});

export type UserPresence = typeof UserPresence.Type;

export type UserPresencePin = Assert<Pinned<typeof UserPresence, GeneratedUserPresence>>;

/** `GET /api/v1/presence?ids=`. */
export const PresenceList = Schema.Struct({ presences: Schema.Array(UserPresence) });

export type PresenceListPin = Assert<Pinned<typeof PresenceList, GeneratedPresenceList>>;
