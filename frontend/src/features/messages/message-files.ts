import type { Attachment } from "../../gen/Attachment.ts";
import type { MessageDTO } from "../../store/model.ts";

/**
 * How a sticker reads as text, as the server writes it for notifications, search, activity and
 * thread names when a message has no words or files of its own (`Message#display_text`).
 */
export function stickerText(name: string): string {
  return `Sticker: ${name}`;
}

export function messageFiles(
  message: Pick<MessageDTO, "attachment" | "attachments">,
): readonly Attachment[] {
  return message.attachments ?? (message.attachment === null ? [] : [message.attachment]);
}

export function fileSummary(message: Pick<MessageDTO, "attachment" | "attachments">): string {
  const files = messageFiles(message);

  return files.length > 1 ? `${files.length} files` : (files[0]?.filename ?? "Attachment");
}

/**
 * What stands for a message without words, as the server's text has it: its files, else its
 * sticker ("Sticker: Wave", kept after the sticker is deleted).
 */
export function wordlessSummary(
  message: Pick<MessageDTO, "attachment" | "attachments" | "sticker">,
): string {
  return messageFiles(message).length === 0 && message.sticker !== undefined
    ? stickerText(message.sticker.name)
    : fileSummary(message);
}
