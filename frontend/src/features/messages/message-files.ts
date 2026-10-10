import type { Attachment } from "../../gen/Attachment.ts";
import type { MessageDTO } from "../../store/model.ts";

export function messageFiles(
  message: Pick<MessageDTO, "attachment" | "attachments">,
): readonly Attachment[] {
  return message.attachments ?? (message.attachment === null ? [] : [message.attachment]);
}

export function fileSummary(message: Pick<MessageDTO, "attachment" | "attachments">): string {
  const files = messageFiles(message);

  return files.length > 1 ? `${files.length} files` : (files[0]?.filename ?? "Attachment");
}
