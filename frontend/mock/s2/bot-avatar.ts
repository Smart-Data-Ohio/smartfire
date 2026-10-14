/** A bot's picked avatar on writes, as the server takes it: checked, colour lowercased. */
import type { BotAvatarSpec } from "../../src/gen/BotAvatarSpec.ts";
import {
  BOT_AVATAR_ACCESSORIES,
  BOT_AVATAR_FACES,
  BOT_AVATAR_TYPES,
} from "../../src/ui/bot-avatar-spec.ts";
import type { HttpError } from "../http.ts";
import { field, isRecord, type Json, stringField } from "../json.ts";

/** What a write does to the spec: set it, clear it (a plain picture came), or leave it. */
export type BotAvatarChange =
  | { readonly kind: "set"; readonly spec: BotAvatarSpec }
  | { readonly kind: "clear" }
  | { readonly kind: "keep" };

/**
 * The write's `botAvatar` beside whether it uploads a picture. A spec the library doesn't have, or
 * a colour that isn't `#rrggbb`, is refused on `botAvatar` through `refuse`.
 */
export function botAvatarChange(
  body: Json | undefined,
  refuse: (message: string) => HttpError,
): BotAvatarChange {
  const given = field(body, "botAvatar");
  const newPicture = stringField(body, "avatar") !== null;

  if (given === undefined || given === null) {
    return newPicture ? { kind: "clear" } : { kind: "keep" };
  }

  if (!isRecord(given)) throw refuse("is invalid");

  const type = BOT_AVATAR_TYPES.find((entry) => entry.type === stringField(given, "type"))?.type;
  const face = BOT_AVATAR_FACES.find((entry) => entry.face === stringField(given, "face"))?.face;
  const named = stringField(given, "accessory");

  const accessory =
    named === null
      ? null
      : BOT_AVATAR_ACCESSORIES.find((entry) => entry.accessory === named)?.accessory;

  const color = stringField(given, "color") ?? "";

  if (type === undefined || face === undefined || accessory === undefined) {
    throw refuse("is invalid");
  }

  if (!/^#[0-9a-f]{6}$/i.test(color)) throw refuse("needs a colour like #7aa2ff");

  return { kind: "set", spec: { type, color: color.toLowerCase(), face, accessory } };
}

/** The spec after `change`, from `current`. */
export function applied(current: BotAvatarSpec | null, change: BotAvatarChange) {
  switch (change.kind) {
    case "set":
      return change.spec;
    case "clear":
      return null;
    default:
      return current;
  }
}
