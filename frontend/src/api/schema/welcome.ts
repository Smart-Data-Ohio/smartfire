import { Schema } from "effect";
import type { GuideEntry as GeneratedGuideEntry } from "../../gen/GuideEntry.ts";
import type { GuideResourceEdit as GeneratedGuideResourceEdit } from "../../gen/GuideResourceEdit.ts";
import type { GuideRoom as GeneratedGuideRoom } from "../../gen/GuideRoom.ts";
import type { WelcomeAdmin as GeneratedWelcomeAdmin } from "../../gen/WelcomeAdmin.ts";
import type { WelcomeRoomCard as GeneratedWelcomeRoomCard } from "../../gen/WelcomeRoomCard.ts";
import type { WelcomeRoomChoice as GeneratedWelcomeRoomChoice } from "../../gen/WelcomeRoomChoice.ts";
import type { WelcomeRoomEdit as GeneratedWelcomeRoomEdit } from "../../gen/WelcomeRoomEdit.ts";
import type { WorkspaceWelcome as GeneratedWorkspaceWelcome } from "../../gen/WorkspaceWelcome.ts";
import { RoomId } from "./ids.ts";
import type { Assert, Pinned } from "./pin.ts";

/**
 * A suggested room in the editor: `roomId` for a room this administrator can see, or an opaque
 * `keep` token for a saved one they can't (sent back as is, it keeps that room).
 */
export const WelcomeRoomEdit = Schema.Struct({
  roomId: Schema.NullOr(RoomId),
  keep: Schema.NullOr(Schema.String),
  emoji: Schema.NullOr(Schema.String),
  description: Schema.String,
});

export type WelcomeRoomEdit = typeof WelcomeRoomEdit.Type;

export type WelcomeRoomEditPin = Assert<Pinned<typeof WelcomeRoomEdit, GeneratedWelcomeRoomEdit>>;

/** A guide entry in the editor: a room link (`roomId` or `keep`) or a short note, never both. */
export const GuideResourceEdit = Schema.Struct({
  title: Schema.String,
  emoji: Schema.NullOr(Schema.String),
  roomId: Schema.NullOr(RoomId),
  keep: Schema.NullOr(Schema.String),
  body: Schema.NullOr(Schema.String),
});

export type GuideResourceEdit = typeof GuideResourceEdit.Type;

export type GuideResourceEditPin = Assert<
  Pinned<typeof GuideResourceEdit, GeneratedGuideResourceEdit>
>;

/** A room the editor offers. */
export const WelcomeRoomChoice = Schema.Struct({ id: RoomId, name: Schema.String });

export type WelcomeRoomChoice = typeof WelcomeRoomChoice.Type;

export type WelcomeRoomChoicePin = Assert<
  Pinned<typeof WelcomeRoomChoice, GeneratedWelcomeRoomChoice>
>;

/** `GET /admin/welcome`, and the answer to the `PATCH`: what this administrator may see. */
export const WelcomeAdmin = Schema.Struct({
  message: Schema.String,
  rooms: Schema.Array(WelcomeRoomEdit),
  guide: Schema.Array(GuideResourceEdit),
  choices: Schema.Array(WelcomeRoomChoice),
});

export type WelcomeAdmin = typeof WelcomeAdmin.Type;

export type WelcomeAdminPin = Assert<Pinned<typeof WelcomeAdmin, GeneratedWelcomeAdmin>>;

/** A suggested room as one viewer sees it: one they belong to, or an open one they may join. */
export const WelcomeRoomCard = Schema.Struct({
  roomId: RoomId,
  name: Schema.String,
  emoji: Schema.NullOr(Schema.String),
  description: Schema.String,
  joined: Schema.Boolean,
});

export type WelcomeRoomCard = typeof WelcomeRoomCard.Type;

export type WelcomeRoomCardPin = Assert<Pinned<typeof WelcomeRoomCard, GeneratedWelcomeRoomCard>>;

/** A guide entry's room, as one viewer sees it. */
export const GuideRoom = Schema.Struct({ id: RoomId, name: Schema.String, joined: Schema.Boolean });

export type GuideRoom = typeof GuideRoom.Type;

export type GuideRoomPin = Assert<Pinned<typeof GuideRoom, GeneratedGuideRoom>>;

/** A guide entry as one viewer sees it: a rendered note or a room link. */
export const GuideEntry = Schema.Struct({
  title: Schema.String,
  emoji: Schema.NullOr(Schema.String),
  bodyHtml: Schema.NullOr(Schema.String),
  room: Schema.NullOr(GuideRoom),
});

export type GuideEntry = typeof GuideEntry.Type;

export type GuideEntryPin = Assert<Pinned<typeof GuideEntry, GeneratedGuideEntry>>;

/** `GET /welcome`: the welcome card and server guide for the signed-in person. */
export const WorkspaceWelcome = Schema.Struct({
  name: Schema.String,
  description: Schema.String,
  messageHtml: Schema.NullOr(Schema.String),
  rooms: Schema.Array(WelcomeRoomCard),
  guide: Schema.Array(GuideEntry),
  canAdminister: Schema.Boolean,
});

export type WorkspaceWelcome = typeof WorkspaceWelcome.Type;

export type WorkspaceWelcomePin = Assert<
  Pinned<typeof WorkspaceWelcome, GeneratedWorkspaceWelcome>
>;
