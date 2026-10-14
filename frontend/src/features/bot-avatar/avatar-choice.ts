import type { BotAvatarSpec } from "../../gen/BotAvatarSpec.ts";
import { loadForUpdate } from "../../service-worker/update-required.ts";
import { sameSpec } from "../../ui/bot-avatar-spec.ts";
import { uploaded } from "../admin/admin-parts.tsx";
import { stoppedAtConfirmation } from "../auth/confirmation.ts";

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
 * A form's last upload, kept while its write waits on the password confirmation: saving the same
 * picture again after the confirmation was closed (or left for Google) reuses it rather than
 * uploading another copy that nothing would ever attach. Any other outcome lets it go, as the
 * server may have attached it.
 */
export interface KeptUpload {
  /** The upload for `key` (a spec's text or the image file), started by `start` if it's new. */
  readonly take: (key: string | File, start: () => Promise<string>) => Promise<string>;
  /** After the write: `null` when it went through, else what stopped it. */
  readonly settle: (error: Error | null) => void;
}

export function keptUpload(): KeptUpload {
  let kept: { readonly key: string | File; readonly id: Promise<string> } | null = null;

  return {
    take: (key, start) => {
      if (kept !== null && kept.key === key) {
        return kept.id;
      }

      const id = start();
      const entry = { key, id };

      kept = entry;
      // A failed upload is tried afresh next time.
      id.catch(() => {
        if (kept === entry) kept = null;
      });

      return id;
    },
    settle: (error) => {
      if (error === null || !stoppedAtConfirmation(error)) kept = null;
    },
  };
}

/** Each call its own upload: for callers that don't keep one across retries. */
const FRESH: KeptUpload = { take: (_key, start) => start(), settle: () => undefined };

function specKey(spec: BotAvatarSpec): string {
  return [spec.type, spec.color.toLowerCase(), spec.face, spec.accessory ?? ""].join("|");
}

/**
 * Uploads what the choice needs and names it for the write: a picked avatar sends its spec with
 * its still render (the picture every non-SPA surface shows), an image just the image. A choice
 * that matches what the bot already has sends nothing. `kept` reuses the form's upload of the
 * same picture from an attempt that didn't go through.
 */
export async function avatarFields(
  choice: AvatarChoice,
  current: BotAvatarSpec | null,
  kept: KeptUpload = FRESH,
): Promise<AvatarFields> {
  switch (choice.kind) {
    case "bot": {
      const { spec } = choice;

      return sameSpec(spec, current)
        ? UNCHANGED
        : {
            avatar: await kept.take(specKey(spec), async () =>
              avatarUploads.upload(await avatarUploads.render(spec)),
            ),
            botAvatar: spec,
          };
    }

    case "upload": {
      const { file } = choice;

      return file === null
        ? UNCHANGED
        : { avatar: await kept.take(file, () => avatarUploads.upload(file)), botAvatar: null };
    }

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
