import type { BotAvatarSpec } from "../../gen/BotAvatarSpec.ts";
import { loadForUpdate } from "../../service-worker/update-required.ts";
import { sameSpec } from "../../ui/bot-avatar-spec.ts";
import { uploaded } from "../admin/admin-parts.tsx";

/**
 * What a bot form does with the bot's picture: a picked bot avatar, an uploaded image, or
 * (editing a bot that has an icon or picture of its own) leave it as it is.
 */
export type AvatarChoice =
  | { readonly kind: "bot"; readonly spec: BotAvatarSpec }
  | { readonly kind: "upload"; readonly file: File | null }
  | { readonly kind: "keep" };

/** The write's `avatar` and `botAvatar`: `null` for both leaves the picture alone. */
export interface AvatarFields {
  readonly avatar: string | null;
  readonly botAvatar: BotAvatarSpec | null;
}

const UNCHANGED: AvatarFields = { avatar: null, botAvatar: null };

/** A picked avatar's still render as an upload-ready file (about 256 px square). */
export async function renderedAvatar(spec: BotAvatarSpec): Promise<File> {
  const { specPng } = await loadForUpdate(() => import("../../ui/bot-bitmap.ts"));
  const png = await specPng(spec);

  if (png === null) {
    throw new Error("This browser couldn't draw the bot avatar.");
  }

  return new File([png], `bot-avatar-${spec.type}.png`, { type: "image/png" });
}

/** How a choice reaches the server: drawn, then uploaded (tests stand in for both). */
export const avatarUploads = {
  render: renderedAvatar,
  upload: uploaded,
};

/**
 * Uploads what the choice needs and names it for the write: a picked avatar sends its spec with
 * its still render (the picture every non-SPA surface shows), an image just the image. A choice
 * that matches what the bot already has sends nothing.
 */
export async function avatarFields(
  choice: AvatarChoice,
  current: BotAvatarSpec | null,
): Promise<AvatarFields> {
  switch (choice.kind) {
    case "bot":
      return sameSpec(choice.spec, current)
        ? UNCHANGED
        : {
            avatar: await avatarUploads.upload(await avatarUploads.render(choice.spec)),
            botAvatar: choice.spec,
          };
    case "upload":
      return choice.file === null
        ? UNCHANGED
        : { avatar: await avatarUploads.upload(choice.file), botAvatar: null };
    default:
      return UNCHANGED;
  }
}

/** Whether saving the choice would change the bot's picture. */
export function changesAvatar(choice: AvatarChoice, current: BotAvatarSpec | null): boolean {
  switch (choice.kind) {
    case "bot":
      return !sameSpec(choice.spec, current);
    case "upload":
      return choice.file !== null;
    default:
      return false;
  }
}
