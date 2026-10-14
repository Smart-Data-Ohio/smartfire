import { Schema } from "effect";
import type { BotAvatarAccessory as GeneratedBotAvatarAccessory } from "../../gen/BotAvatarAccessory.ts";
import type { BotAvatarFace as GeneratedBotAvatarFace } from "../../gen/BotAvatarFace.ts";
import type { BotAvatarSpec as GeneratedBotAvatarSpec } from "../../gen/BotAvatarSpec.ts";
import type { BotAvatarType as GeneratedBotAvatarType } from "../../gen/BotAvatarType.ts";
import type { Assert, Pinned } from "./pin.ts";

/** The bot-avatars library's eighteen bodies. */
export const BotAvatarType = Schema.Literals([
  "clover",
  "flower",
  "triangle",
  "square",
  "blob",
  "ghost",
  "circle",
  "drop",
  "star",
  "droid",
  "mech",
  "alien",
  "hexagon",
  "cat",
  "cloud",
  "pill",
  "pebble",
  "puddle",
]);

export type BotAvatarTypePin = Assert<Pinned<typeof BotAvatarType, GeneratedBotAvatarType>>;

export const BotAvatarFace = Schema.Literals(["eyes", "mouth"]);

export type BotAvatarFacePin = Assert<Pinned<typeof BotAvatarFace, GeneratedBotAvatarFace>>;

export const BotAvatarAccessory = Schema.Literals([
  "beret",
  "beanie",
  "party",
  "crown",
  "roundGlasses",
  "squareGlasses",
  "shades",
  "headphones",
  "bowTie",
]);

export type BotAvatarAccessoryPin = Assert<
  Pinned<typeof BotAvatarAccessory, GeneratedBotAvatarAccessory>
>;

/** A bot's picked avatar: a body in a colour (`#rrggbb`), a face and at most one thing to wear. */
export const BotAvatarSpec = Schema.Struct({
  type: BotAvatarType,
  color: Schema.String,
  face: BotAvatarFace,
  accessory: Schema.NullOr(BotAvatarAccessory),
});

export type BotAvatarSpecPin = Assert<Pinned<typeof BotAvatarSpec, GeneratedBotAvatarSpec>>;
