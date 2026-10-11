import { Schema } from "effect";
import type { GuideEntry as GeneratedGuideEntry } from "../../gen/GuideEntry.ts";
import type { GuideResource as GeneratedGuideResource } from "../../gen/GuideResource.ts";
import type { GuideRoom as GeneratedGuideRoom } from "../../gen/GuideRoom.ts";
import type { WelcomeAdmin as GeneratedWelcomeAdmin } from "../../gen/WelcomeAdmin.ts";
import type { WelcomeRoom as GeneratedWelcomeRoom } from "../../gen/WelcomeRoom.ts";
import type { WelcomeRoomCard as GeneratedWelcomeRoomCard } from "../../gen/WelcomeRoomCard.ts";
import type { WelcomeRoomChoice as GeneratedWelcomeRoomChoice } from "../../gen/WelcomeRoomChoice.ts";
import type { WelcomeSettings as GeneratedWelcomeSettings } from "../../gen/WelcomeSettings.ts";
import type { WorkspaceWelcome as GeneratedWorkspaceWelcome } from "../../gen/WorkspaceWelcome.ts";
import { RoomId } from "./ids.ts";
import type { Assert, Pinned } from "./pin.ts";

/** A suggested room as administrators set it. */
export const WelcomeRoom = Schema.Struct({
  roomId: RoomId,
  emoji: Schema.NullOr(Schema.String),
  description: Schema.String,
});

export type WelcomeRoom = typeof WelcomeRoom.Type;

export type WelcomeRoomPin = Assert<Pinned<typeof WelcomeRoom, GeneratedWelcomeRoom>>;

/** A guide entry as administrators set it: a room link or a short note, never both. */
export const GuideResource = Schema.Struct({
  title: Schema.String,
  emoji: Schema.NullOr(Schema.String),
  roomId: Schema.NullOr(RoomId),
  body: Schema.NullOr(Schema.String),
});

export type GuideResource = typeof GuideResource.Type;

export type GuideResourcePin = Assert<Pinned<typeof GuideResource, GeneratedGuideResource>>;

/** `PUT /admin/welcome`: the whole welcome card and server guide. */
export const WelcomeSettings = Schema.Struct({
  message: Schema.String,
  rooms: Schema.Array(WelcomeRoom),
  guide: Schema.Array(GuideResource),
});

export type WelcomeSettings = typeof WelcomeSettings.Type;

export type WelcomeSettingsPin = Assert<Pinned<typeof WelcomeSettings, GeneratedWelcomeSettings>>;

/** A room the editor offers. */
export const WelcomeRoomChoice = Schema.Struct({ id: RoomId, name: Schema.String });

export type WelcomeRoomChoice = typeof WelcomeRoomChoice.Type;

export type WelcomeRoomChoicePin = Assert<
  Pinned<typeof WelcomeRoomChoice, GeneratedWelcomeRoomChoice>
>;

/** `GET /admin/welcome`, and the answer to the `PUT`. */
export const WelcomeAdmin = Schema.Struct({
  settings: WelcomeSettings,
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
