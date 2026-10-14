import type { BotAvatarAccessory } from "../gen/BotAvatarAccessory.ts";
import type { BotAvatarFace } from "../gen/BotAvatarFace.ts";
import type { BotAvatarSpec } from "../gen/BotAvatarSpec.ts";
import type { BotAvatarType } from "../gen/BotAvatarType.ts";

export type { BotAvatarAccessory, BotAvatarFace, BotAvatarSpec, BotAvatarType };

/**
 * The bot-avatars library's eighteen bodies with their own colours (`botAvatarPresets`), kept
 * here so a form can pick and label one before the library's chunk loads. A test holds them to
 * the library's.
 */
export const BOT_AVATAR_TYPES: readonly { type: BotAvatarType; label: string; color: string }[] = [
  { type: "clover", label: "Clover", color: "#41c4ff" },
  { type: "flower", label: "Flower", color: "#2fcb7a" },
  { type: "triangle", label: "Triangle", color: "#dc48ff" },
  { type: "square", label: "Square", color: "#35b8ff" },
  { type: "blob", label: "Blob", color: "#2fcb7a" },
  { type: "ghost", label: "Ghost", color: "#f4f2fa" },
  { type: "circle", label: "Circle", color: "#9a62ff" },
  { type: "drop", label: "Drop", color: "#1ed3c6" },
  { type: "star", label: "Star", color: "#efdb8e" },
  { type: "droid", label: "Droid", color: "#d5dbea" },
  { type: "mech", label: "Mech", color: "#95a6c4" },
  { type: "alien", label: "Alien", color: "#9cd66a" },
  { type: "hexagon", label: "Hexagon", color: "#eb7575" },
  { type: "cat", label: "Cat", color: "#eaa06f" },
  { type: "cloud", label: "Cloud", color: "#cfd5e4" },
  { type: "pill", label: "Pill", color: "#acaaf3" },
  { type: "pebble", label: "Pebble", color: "#abc793" },
  { type: "puddle", label: "Puddle", color: "#ee8bdb" },
];

/** The colours a picker offers, round the hue wheel: the library's palette plus a charcoal. */
export const BOT_AVATAR_SWATCHES: readonly { color: string; label: string }[] = [
  { color: "#eb7575", label: "Coral" },
  { color: "#eaa06f", label: "Apricot" },
  { color: "#efdb8e", label: "Butter" },
  { color: "#9cd66a", label: "Lime" },
  { color: "#2fcb7a", label: "Green" },
  { color: "#1ed3c6", label: "Teal" },
  { color: "#41c4ff", label: "Sky" },
  { color: "#9a62ff", label: "Violet" },
  { color: "#acaaf3", label: "Lavender" },
  { color: "#dc48ff", label: "Magenta" },
  { color: "#ee8bdb", label: "Pink" },
  { color: "#95a6c4", label: "Slate" },
  { color: "#f4f2fa", label: "Snow" },
  { color: "#3a3a44", label: "Charcoal" },
];

export const BOT_AVATAR_FACES: readonly { face: BotAvatarFace; label: string }[] = [
  { face: "eyes", label: "Eyes" },
  { face: "mouth", label: "Eyes and mouth" },
];

export const BOT_AVATAR_ACCESSORIES: readonly { accessory: BotAvatarAccessory; label: string }[] = [
  { accessory: "beret", label: "Beret" },
  { accessory: "beanie", label: "Beanie" },
  { accessory: "party", label: "Party hat" },
  { accessory: "crown", label: "Crown" },
  { accessory: "roundGlasses", label: "Round glasses" },
  { accessory: "squareGlasses", label: "Square glasses" },
  { accessory: "shades", label: "Shades" },
  { accessory: "headphones", label: "Headphones" },
  { accessory: "bowTie", label: "Bow tie" },
];

const DEFAULT_TYPE = BOT_AVATAR_TYPES[0] ?? { type: "clover", label: "Clover", color: "#41c4ff" };

function entryOf(type: BotAvatarType) {
  return BOT_AVATAR_TYPES.find((entry) => entry.type === type) ?? DEFAULT_TYPE;
}

/** The body's own colour, the one the library draws it in when given none. */
export function ownColor(type: BotAvatarType): string {
  return entryOf(type).color;
}

export function typeLabel(type: BotAvatarType): string {
  return entryOf(type).label;
}

/** What a picked avatar reads as, e.g. "Cat in Coral with a crown". */
export function describeSpec(spec: BotAvatarSpec): string {
  const swatch = BOT_AVATAR_SWATCHES.find((each) => each.color === spec.color.toLowerCase());

  const colour =
    spec.color.toLowerCase() === ownColor(spec.type)
      ? "its own colour"
      : (swatch?.label ?? spec.color);

  const wearing = BOT_AVATAR_ACCESSORIES.find((each) => each.accessory === spec.accessory);

  return `${typeLabel(spec.type)} in ${colour}${wearing === undefined ? "" : `, ${wearing.label.toLowerCase()}`}`;
}

/** A fresh form's avatar: any body in its own colour, either face, nothing to wear. */
export function randomSpec(random: () => number = Math.random): BotAvatarSpec {
  const entry = BOT_AVATAR_TYPES[Math.floor(random() * BOT_AVATAR_TYPES.length)] ?? DEFAULT_TYPE;

  return {
    type: entry.type,
    color: entry.color,
    face: random() < 0.5 ? "eyes" : "mouth",
    accessory: null,
  };
}

/** A new body keeps a picked colour, but swaps the old body's own colour for the new one's. */
export function withType(spec: BotAvatarSpec, type: BotAvatarType): BotAvatarSpec {
  const color = spec.color.toLowerCase() === ownColor(spec.type) ? ownColor(type) : spec.color;

  return { ...spec, type, color };
}

export function sameSpec(a: BotAvatarSpec | null, b: BotAvatarSpec | null): boolean {
  if (a === null || b === null) {
    return a === b;
  }

  return (
    a.type === b.type &&
    a.color.toLowerCase() === b.color.toLowerCase() &&
    a.face === b.face &&
    a.accessory === b.accessory
  );
}

/** What the library's component and draw config wear for an accessory. */
export interface BotAvatarWear {
  readonly hat: "none" | "beret" | "beanie" | "party" | "crown";
  readonly glasses: "none" | "round" | "square" | "shades";
  readonly headphones: boolean;
  readonly bowTie: boolean;
}

export function wearOf(accessory: BotAvatarAccessory | null): BotAvatarWear {
  const none: BotAvatarWear = { hat: "none", glasses: "none", headphones: false, bowTie: false };

  switch (accessory) {
    case "beret":
    case "beanie":
    case "party":
    case "crown":
      return { ...none, hat: accessory };
    case "roundGlasses":
      return { ...none, glasses: "round" };
    case "squareGlasses":
      return { ...none, glasses: "square" };
    case "shades":
      return { ...none, glasses: "shades" };
    case "headphones":
      return { ...none, headphones: true };
    case "bowTie":
      return { ...none, bowTie: true };
    default:
      return none;
  }
}

/**
 * How much of its tile a toy fills, and how far below centre it sits: a hat stands above the
 * body, so a hatted toy is drawn smaller and lower to keep the hat inside the tile.
 */
export function fitOf(
  accessory: BotAvatarAccessory | null,
  size: number,
): { readonly scale: number; readonly drop: number } {
  const snug = size <= 24;

  return wearOf(accessory).hat === "none"
    ? { scale: snug ? 0.9 : 0.8, drop: 0 }
    : { scale: snug ? 0.76 : 0.68, drop: snug ? 0.07 : 0.08 };
}

/** The colour the library takes: `#rrggbb`, else the body's own. */
export function colorOf(spec: BotAvatarSpec): string {
  return /^#[0-9a-f]{6}$/i.test(spec.color) ? spec.color.toLowerCase() : ownColor(spec.type);
}

/** A picked avatar kept with a draft as JSON; `null` for none or anything the picker can't show. */
export function keptSpec(json: string): BotAvatarSpec | null {
  if (json === "") {
    return null;
  }

  try {
    const kept: Partial<Record<keyof BotAvatarSpec, string | null>> = JSON.parse(json);
    const type = BOT_AVATAR_TYPES.find((entry) => entry.type === kept.type)?.type;
    const face = BOT_AVATAR_FACES.find((entry) => entry.face === kept.face)?.face;

    const accessory =
      kept.accessory === null
        ? null
        : BOT_AVATAR_ACCESSORIES.find((entry) => entry.accessory === kept.accessory)?.accessory;

    const color = kept.color ?? "";

    if (
      type === undefined ||
      face === undefined ||
      accessory === undefined ||
      !/^#[0-9a-f]{6}$/i.test(color)
    ) {
      return null;
    }

    return { type, color, face, accessory };
  } catch {
    return null;
  }
}
