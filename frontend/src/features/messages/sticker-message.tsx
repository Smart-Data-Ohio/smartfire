import type { MessageSticker } from "../../gen/MessageSticker.ts";
import { EmojiImage } from "../../lib/emoji/emoji-image.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import { stickerText } from "./message-files.ts";
import "./messages.css";

/** A sticker's box in a message, in CSS pixels: stickers are at most 320 × 320, shown at half. */
export const STICKER_SIZE = 160;

/**
 * A message's sticker at a fixed size. An animated one plays, and rests on its first frame under
 * reduced motion (the OS setting or the person's own), as animated workspace icons do. A sticker
 * deleted from the catalog leaves a quiet line with the name it had.
 */
export function StickerMessage({ sticker }: { readonly sticker: MessageSticker }) {
  if (sticker.kind === "deleted") {
    return (
      <p className="message-sticker-deleted">
        <Icon name="sticker" size={14} />
        <span>
          <span className="message-sticker-deleted-name">{sticker.name}</span> · Sticker deleted
        </span>
      </p>
    );
  }

  return (
    <EmojiImage
      className="message-sticker"
      src={sticker.imageUrl}
      still={sticker.stillUrl}
      alt={stickerText(sticker.name)}
      title={sticker.name}
      width={STICKER_SIZE}
      height={STICKER_SIZE}
      loading="lazy"
      draggable={false}
      data-animated={sticker.animated || undefined}
    />
  );
}
